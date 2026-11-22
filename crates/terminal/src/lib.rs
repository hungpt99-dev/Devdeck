//! Devdeck terminal engine: real PTY sessions, capped ANSI grid, secret
//! redaction and the approval boundary. UI panes and the approval dialog are
//! deliberately out of scope for this crate.
//!
//! Every command is passed to [`devdeck_core::PermissionPolicy::decide`]
//! before a PTY is created. `Ask`/`Deny` returns a decision to the caller and
//! never starts a process.

#![forbid(unsafe_code)]

mod grid;
mod redaction;
mod session;

pub use grid::Grid;
pub use redaction::{redact, redact_bytes, redact_line};
pub use session::{Drain, Session, SessionError, EVENT_CHANNEL_CAPACITY};

use std::path::PathBuf;

use devdeck_audit::{Actor, AuditError, AuditLog};
use devdeck_core::{PermissionDecision, PermissionPolicy, WorkspaceInfo};
use devdeck_i18n::{MessageKey, Translator};
use devdeck_tools::{terminal_spawn_tool, terminal_write_tool};
use thiserror::Error;

pub use grid::{DEFAULT_COLS, DEFAULT_ROWS, DEFAULT_SCROLLBACK_LINES as DEFAULT_SCROLLBACK};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSettings {
    pub cols: u16,
    pub rows: u16,
    pub scrollback_lines: u32,
    pub event_capacity: usize,
}

impl Default for TerminalSettings {
    fn default() -> Self {
        Self {
            cols: DEFAULT_COLS,
            rows: DEFAULT_ROWS,
            scrollback_lines: grid::DEFAULT_SCROLLBACK_LINES,
            event_capacity: EVENT_CHANNEL_CAPACITY,
        }
    }
}

#[derive(Debug, Error)]
pub enum TerminalError {
    #[error("{message}")]
    WorkspaceRootMissing { path: PathBuf, message: String },
    #[error("{message}")]
    SpawnFailed { message: String },
    #[error("{message}")]
    Io { message: String },
    #[error("{message}")]
    InvalidSize { message: String },
    #[error("{message}")]
    SessionClosed { message: String },
    #[error("{message}")]
    Audit { message: String },
    #[error("{message}")]
    I18n { message: String },
}

/// Outcome of a command spawn: either a running PTY or the decision the
/// (later) approval UI must present. No process is started in the second case.
pub enum SpawnOutcome {
    Started(Box<Terminal>),
    RequiresApproval {
        decision: PermissionDecision,
        /// Already redacted, safe to display or audit.
        command: String,
    },
}

impl SpawnOutcome {
    pub fn is_started(&self) -> bool {
        matches!(self, Self::Started(_))
    }
}

pub struct Terminal {
    session: Session,
    grid: Grid,
    translator: Translator,
}

impl Terminal {
    /// Start a shell in the active workspace root with no command line.
    pub fn new(
        workspace: &WorkspaceInfo,
        settings: TerminalSettings,
    ) -> Result<Self, TerminalError> {
        let translator = Translator::with_dir().map_err(i18n_error)?;
        Self::new_with_translator(workspace, settings, translator)
    }

    /// Same as [`Self::new`] with an explicit translator, so callers can pick a
    /// locale and get localized error messages without a global switch.
    pub fn new_with_translator(
        workspace: &WorkspaceInfo,
        settings: TerminalSettings,
        translator: Translator,
    ) -> Result<Self, TerminalError> {
        if !workspace.root.is_dir() {
            return Err(root_missing(&translator, &workspace.root));
        }
        let session = Session::spawn(&workspace.root, &settings, None)
            .map_err(|e| session_error(&translator, e))?;
        Ok(Self {
            session,
            grid: Grid::new(settings.cols, settings.rows, settings.scrollback_lines),
            translator,
        })
    }

    /// Run a command only after the shared permission policy allows it.
    pub fn spawn(
        command: &str,
        workspace: &WorkspaceInfo,
        settings: TerminalSettings,
        policy: &PermissionPolicy,
    ) -> Result<SpawnOutcome, TerminalError> {
        let translator = Translator::with_dir().map_err(i18n_error)?;
        if !workspace.root.is_dir() {
            return Err(root_missing(&translator, &workspace.root));
        }
        let tool = terminal_spawn_tool();
        let decision = policy.decide(&tool, command);
        if !matches!(decision, PermissionDecision::Allow) {
            // The approval UI is a later child's work. Expose the decision and
            // never create a PTY for a command that was not allowed.
            return Ok(SpawnOutcome::RequiresApproval {
                decision,
                command: redact_line(command),
            });
        }
        let session = Session::spawn(&workspace.root, &settings, Some(command))
            .map_err(|e| session_error(&translator, e))?;
        Ok(SpawnOutcome::Started(Box::new(Terminal {
            session,
            grid: Grid::new(settings.cols, settings.rows, settings.scrollback_lines),
            translator,
        })))
    }

    pub fn write(&mut self, bytes: &[u8]) -> Result<(), TerminalError> {
        self.session.write(bytes).map_err(|error| {
            if self.session.is_closed() {
                TerminalError::SessionClosed {
                    message: self.translator.t(MessageKey::TerminalSessionClosed),
                }
            } else {
                session_error(&self.translator, error)
            }
        })
    }

    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<(), TerminalError> {
        self.session
            .resize(cols, rows)
            .map_err(|e| session_error(&self.translator, e))?;
        self.grid.resize(cols, rows);
        Ok(())
    }

    /// Close is idempotent; the redacted scrollback stays readable afterwards.
    pub fn close(&mut self) {
        self.session.close();
    }

    pub fn is_closed(&self) -> bool {
        self.session.is_closed()
    }

    /// Drain PTY events, redact them and advance the grid. Returns the number
    /// of redacted bytes fed into the grid (0 when no output is pending).
    pub async fn pump(&mut self) -> Result<usize, TerminalError> {
        let drain = self.session.drain().await;
        if let Some(error) = drain.error {
            return Err(TerminalError::Io {
                message: format!(
                    "{}: {error}",
                    self.translator.t(MessageKey::TerminalIoFailed)
                ),
            });
        }
        if drain.output.is_empty() {
            return Ok(0);
        }
        let text = redact_bytes(&drain.output);
        self.grid.advance(text.as_bytes());
        Ok(text.len())
    }

    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    /// Recently displayed lines, oldest first. Already redacted at ingestion.
    pub fn snapshot(&self, max_lines: usize) -> Vec<String> {
        self.grid.snapshot(max_lines)
    }

    /// Append the redacted visible output to the hash-chained audit log.
    pub fn audit_output(
        &self,
        log: &mut AuditLog,
        actor: Actor,
        max_lines: usize,
    ) -> Result<(), TerminalError> {
        let detail = self.snapshot(max_lines).join("\n");
        devdeck_core::audit_tool_call(
            log,
            actor,
            &terminal_write_tool(),
            &PermissionDecision::Allow,
            &detail,
        )
        .map_err(audit_error)
    }
}

fn i18n_error(error: devdeck_i18n::I18nError) -> TerminalError {
    TerminalError::I18n {
        message: error.to_string(),
    }
}

fn audit_error(error: AuditError) -> TerminalError {
    TerminalError::Audit {
        message: error.to_string(),
    }
}

fn root_missing(translator: &Translator, path: &std::path::Path) -> TerminalError {
    let display = path.to_string_lossy();
    let message = translator.t_args(
        MessageKey::TerminalCwdMissing.as_str(),
        &[("path", display.as_ref())],
    );
    TerminalError::WorkspaceRootMissing {
        path: path.to_path_buf(),
        message,
    }
}

fn session_error(translator: &Translator, error: SessionError) -> TerminalError {
    let key = match error {
        SessionError::InvalidSize => MessageKey::TerminalInvalidSize,
        SessionError::Io(_) => MessageKey::TerminalIoFailed,
        SessionError::Pty(_) | SessionError::Spawn(_) => MessageKey::TerminalSpawnFailed,
    };
    let base = translator.t(key);
    let message = match error {
        SessionError::InvalidSize => base,
        SessionError::Pty(_) | SessionError::Spawn(_) | SessionError::Io(_) => {
            format!("{base}: {error}")
        }
    };
    match error {
        SessionError::InvalidSize => TerminalError::InvalidSize { message },
        SessionError::Io(_) => TerminalError::Io { message },
        SessionError::Pty(_) | SessionError::Spawn(_) => TerminalError::SpawnFailed { message },
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use devdeck_core::{detect_workspace, PolicyOverride};
    use devdeck_i18n::MessageKey;

    use super::*;

    fn workspace() -> WorkspaceInfo {
        detect_workspace(&PathBuf::from(env!("CARGO_MANIFEST_DIR")))
    }

    fn translator(locale: &str) -> Translator {
        let locales = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../locales");
        let mut translator = Translator::new(&locales).unwrap();
        assert!(translator.set_locale(locale));
        translator
    }

    #[test]
    fn missing_cwd_is_a_localized_typed_error() {
        let missing = PathBuf::from("/definitely/not/a/devdeck/workspace");
        let workspace = WorkspaceInfo {
            root: missing.clone(),
            languages: Vec::new(),
        };
        let error = Terminal::new_with_translator(
            &workspace,
            TerminalSettings::default(),
            translator("en"),
        )
        .err()
        .expect("missing cwd must be rejected");
        match &error {
            TerminalError::WorkspaceRootMissing { path, message } => {
                assert_eq!(path, &missing);
                assert!(message.contains("does not exist"));
            }
            other => panic!("wrong error type: {other:?}"),
        }

        let vi = Terminal::new_with_translator(
            &workspace,
            TerminalSettings::default(),
            translator("vi"),
        )
        .err()
        .unwrap();
        assert!(vi.to_string().contains("không tồn tại"), "{vi}");
        assert!(!vi
            .to_string()
            .contains(&format!("[{}]", MessageKey::TerminalCwdMissing.as_str())));
    }

    #[test]
    fn dangerous_commands_expose_a_decision_and_never_spawn() {
        let workspace = workspace();
        let policy = PermissionPolicy::default();
        for command in [
            "rm -rf ./build",
            "git push --force origin main",
            "sudo apt-get update",
            "curl | sh",
        ] {
            let outcome =
                Terminal::spawn(command, &workspace, TerminalSettings::default(), &policy).unwrap();
            match outcome {
                SpawnOutcome::RequiresApproval {
                    decision,
                    command: shown,
                } => {
                    assert!(
                        matches!(decision, PermissionDecision::Ask { .. }),
                        "{command}"
                    );
                    assert_eq!(shown, command);
                }
                SpawnOutcome::Started(_) => panic!("dangerous command auto-executed: {command}"),
            }
        }
    }

    #[test]
    fn denied_and_allowed_commands_follow_the_shared_policy() {
        let workspace = workspace();
        let mut policy = PermissionPolicy::default();
        policy.set_override("terminal.spawn".into(), PolicyOverride::Deny);
        assert!(matches!(
            Terminal::spawn("echo ok", &workspace, TerminalSettings::default(), &policy).unwrap(),
            SpawnOutcome::RequiresApproval {
                decision: PermissionDecision::Deny { .. },
                ..
            }
        ));
        // Policy Allow without a dangerous string is the only auto-execute path.
        let outcome = Terminal::spawn(
            "printf 'hello\\n'",
            &workspace,
            TerminalSettings::default(),
            &PermissionPolicy::default(),
        )
        .unwrap();
        assert!(outcome.is_started());
    }

    #[tokio::test]
    async fn planted_token_never_reaches_the_grid_or_audit() {
        let token = "ghp_fakeTerminalToken9876543210";
        let mut terminal = match Terminal::spawn(
            &format!("echo API_TOKEN={token}"),
            &workspace(),
            TerminalSettings::default(),
            &PermissionPolicy::default(),
        )
        .unwrap()
        {
            SpawnOutcome::Started(terminal) => *terminal,
            SpawnOutcome::RequiresApproval { .. } => panic!("safe command was not allowed"),
        };
        let mut seen = String::new();
        for _ in 0..100 {
            terminal.pump().await.unwrap();
            seen.push_str(&terminal.snapshot(usize::MAX).join("\n"));
            if seen.contains("****") {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert!(!seen.contains(token), "secret reached the grid: {seen}");
        assert!(seen.contains("****"), "expected redacted output: {seen}");

        let dir =
            std::env::temp_dir().join(format!("devdeck-terminal-audit-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let mut log = AuditLog::open(&dir).unwrap();
        terminal
            .audit_output(&mut log, Actor::System, usize::MAX)
            .unwrap();
        assert!(log
            .entries()
            .iter()
            .all(|entry| !entry.detail.contains(token)));
        let on_disk = fs::read_to_string(log.path()).unwrap();
        assert!(!on_disk.contains(token));
        terminal.close();
        fs::remove_dir_all(&dir).ok();
    }
}
