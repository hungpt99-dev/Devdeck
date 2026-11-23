//! Terminal-command task runner.
//!
//! Tasks are named shell commands declared in the workspace's versioned
//! `devdeck.toml`. There is no workflow DAG, job queue or process manager:
//! a task run is one command handed to the F3 PTY session, after the shared
//! `PermissionPolicy::decide` gate and with a hash-chained audit entry.
//!
//! The command's cwd is always the workspace root that owns the config. A cwd
//! is deliberately not a config field, so a task can never redirect execution
//! into another project; a missing config is an error, never a fallback.

#![forbid(unsafe_code)]

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::Duration;

use devdeck_audit::{redact, Actor, AuditLog};
use devdeck_core::{
    audit_tool_call, PermissionDecision, PermissionPolicy, WorkspaceConfig, WorkspaceInfo,
    WORKSPACE_CONFIG_FILE,
};
use devdeck_i18n::{MessageKey, Translator};
use devdeck_terminal::{SpawnOutcome, Terminal, TerminalError, TerminalSettings};
use devdeck_tools::tasks_run_tool;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Poll interval while waiting for a spawned task to exit. Output is drained
/// on every poll so a chatty child cannot block on a full PTY event channel.
const EXIT_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// A named command from `[tasks]` in a workspace `devdeck.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub name: String,
    pub command: String,
}

#[derive(Debug, Error)]
pub enum TaskError {
    #[error("{message}")]
    WorkspaceRootMissing { root: PathBuf, message: String },
    #[error("{message}")]
    ConfigMissing { path: PathBuf, message: String },
    #[error("{message}")]
    MalformedConfig { path: PathBuf, message: String },
    #[error("{message}")]
    UnsupportedVersion { version: u32, message: String },
    #[error("{message}")]
    UnknownTask { name: String, message: String },
    #[error("{message}")]
    MalformedCommand { name: String, message: String },
    #[error("{message}")]
    I18n { message: String },
    #[error("{message}")]
    Terminal { message: String },
    #[error("{message}")]
    Audit { message: String },
}

/// Result of a task run. `RequiresApproval` is returned instead of starting
/// any process when the shared policy asks or denies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskRunOutcome {
    Completed {
        task: String,
        /// Exact command string sent into the F3 terminal session.
        command: String,
        /// Exit status of the command.
        status: Option<u32>,
    },
    RequiresApproval {
        task: String,
        command: String,
        decision: PermissionDecision,
    },
}

impl TaskRunOutcome {
    pub fn status(&self) -> Option<u32> {
        match self {
            Self::Completed { status, .. } => *status,
            Self::RequiresApproval { .. } => None,
        }
    }

    pub fn decision(&self) -> &PermissionDecision {
        const ALLOW: PermissionDecision = PermissionDecision::Allow;
        match self {
            Self::Completed { .. } => &ALLOW,
            Self::RequiresApproval { decision, .. } => decision,
        }
    }
}

fn root_missing(translator: &Translator, root: &Path) -> TaskError {
    let path = root.to_string_lossy();
    TaskError::WorkspaceRootMissing {
        root: root.to_path_buf(),
        message: translator.t_args(
            MessageKey::TaskRootMissing.as_str(),
            &[("path", path.as_ref())],
        ),
    }
}

fn config_missing(translator: &Translator, path: &Path) -> TaskError {
    let display = path.to_string_lossy();
    TaskError::ConfigMissing {
        path: path.to_path_buf(),
        message: translator.t_args(
            MessageKey::TaskConfigMissing.as_str(),
            &[("path", display.as_ref())],
        ),
    }
}

fn malformed_config(translator: &Translator, path: &Path, reason: &str) -> TaskError {
    let display = path.to_string_lossy();
    TaskError::MalformedConfig {
        path: path.to_path_buf(),
        message: translator.t_args(
            MessageKey::TaskMalformedConfig.as_str(),
            &[("path", display.as_ref()), ("reason", reason)],
        ),
    }
}

fn unsupported_version(translator: &Translator, version: u32) -> TaskError {
    let value = version.to_string();
    TaskError::UnsupportedVersion {
        version,
        message: translator.t_args(
            MessageKey::TaskUnsupportedVersion.as_str(),
            &[("version", value.as_str())],
        ),
    }
}

fn unknown_task(translator: &Translator, name: &str) -> TaskError {
    TaskError::UnknownTask {
        name: name.to_string(),
        message: translator.t_args(MessageKey::TaskUnknownTask.as_str(), &[("task", name)]),
    }
}

fn malformed_command(translator: &Translator, name: &str, reason: MessageKey) -> TaskError {
    let reason = translator.t(reason);
    TaskError::MalformedCommand {
        name: name.to_string(),
        message: translator.t_args(
            MessageKey::TaskMalformedCommand.as_str(),
            &[("task", name), ("reason", reason.as_str())],
        ),
    }
}

fn terminal_error(error: TerminalError) -> TaskError {
    match error {
        TerminalError::Audit { message } => TaskError::Audit { message },
        TerminalError::I18n { message } => TaskError::I18n { message },
        other => TaskError::Terminal {
            message: other.to_string(),
        },
    }
}

fn validate_name(translator: &Translator, name: &str) -> Result<(), TaskError> {
    if name.trim().is_empty() {
        return Err(malformed_command(
            translator,
            name,
            MessageKey::TaskReasonEmptyName,
        ));
    }
    if name != name.trim() {
        return Err(malformed_command(
            translator,
            name,
            MessageKey::TaskReasonNameWhitespace,
        ));
    }
    Ok(())
}

/// Reject commands that cannot be handed to a shell verbatim. Deliberately
/// permissive otherwise: a task IS a shell command string, so `;`, pipes and
/// newlines are valid user input and are gated by the permission policy.
fn validate_command(translator: &Translator, name: &str, command: &str) -> Result<(), TaskError> {
    if command.trim().is_empty() {
        return Err(malformed_command(
            translator,
            name,
            MessageKey::TaskReasonEmptyCommand,
        ));
    }
    if command.contains('\0') {
        return Err(malformed_command(
            translator,
            name,
            MessageKey::TaskReasonNulCommand,
        ));
    }
    Ok(())
}

/// Parse a `devdeck.toml` document into named tasks. `path` is only used to
/// produce a localized error message; the cwd is never read from the document.
pub fn parse(raw: &str, translator: &Translator) -> Result<Vec<Task>, TaskError> {
    parse_with_path(raw, Path::new(WORKSPACE_CONFIG_FILE), translator)
}

fn parse_with_path(
    raw: &str,
    path: &Path,
    translator: &Translator,
) -> Result<Vec<Task>, TaskError> {
    let config: WorkspaceConfig = toml::from_str(raw)
        .map_err(|error| malformed_config(translator, path, &error.to_string()))?;
    if let Some(version) = config.unsupported_version() {
        return Err(unsupported_version(translator, version));
    }
    let mut tasks = Vec::with_capacity(config.tasks.len());
    for (name, command) in config.tasks {
        validate_name(translator, &name)?;
        validate_command(translator, &name, &command)?;
        tasks.push(Task { name, command });
    }
    Ok(tasks)
}

/// Load tasks from `root/devdeck.toml`. A missing file is a typed error: there
/// is no implicit global config and no search in parent or sibling projects.
pub fn load(root: &Path, translator: &Translator) -> Result<Vec<Task>, TaskError> {
    if !root.is_dir() {
        return Err(root_missing(translator, root));
    }
    let path = WorkspaceConfig::path_in(root);
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Err(config_missing(translator, &path));
        }
        Err(error) => return Err(malformed_config(translator, &path, &error.to_string())),
    };
    parse_with_path(&raw, &path, translator)
}

/// Names declared in the workspace config, in stable order.
pub fn list(root: &Path, translator: &Translator) -> Result<Vec<String>, TaskError> {
    Ok(load(root, translator)?
        .into_iter()
        .map(|task| task.name)
        .collect())
}

/// Run a named task through the F3 terminal session in `workspace.root`.
///
/// The exact configured command is passed to `Terminal::spawn`, which writes
/// it into the workspace shell PTY. The shared policy decides first: `Ask`
/// and `Deny` return [`TaskRunOutcome::RequiresApproval`] and never spawn a
/// process. Every attempt — allowed or not — is audited via `tasks.run`.
pub async fn run(
    name: &str,
    workspace: &WorkspaceInfo,
    policy: &PermissionPolicy,
    audit: &mut AuditLog,
    actor: Actor,
    settings: TerminalSettings,
    translator: &Translator,
) -> Result<TaskRunOutcome, TaskError> {
    let task = load(&workspace.root, translator)?
        .into_iter()
        .find(|task| task.name == name)
        .ok_or_else(|| unknown_task(translator, name))?;

    let tool = tasks_run_tool();
    let decision = policy.decide(&tool, &task.command);
    // The audit log redacts credential-shaped values on append.
    let detail = format!("task={} command={}", task.name, redact(&task.command));
    audit_tool_call(audit, actor, &tool, &decision, &detail).map_err(|error| TaskError::Audit {
        message: error.to_string(),
    })?;

    if !matches!(decision, PermissionDecision::Allow) {
        return Ok(TaskRunOutcome::RequiresApproval {
            task: task.name,
            command: task.command,
            decision,
        });
    }

    let outcome =
        Terminal::spawn(&task.command, workspace, settings, policy).map_err(terminal_error)?;
    let mut terminal = match outcome {
        SpawnOutcome::Started(terminal) => *terminal,
        // F3 applied a stricter decision (e.g. a terminal.spawn override).
        // Surface it to the approval UI; nothing was started.
        SpawnOutcome::RequiresApproval { decision, .. } => {
            return Ok(TaskRunOutcome::RequiresApproval {
                task: task.name,
                command: task.command,
                decision,
            });
        }
    };

    let status = loop {
        terminal.pump().await.map_err(terminal_error)?;
        if let Some(status) = terminal.try_wait().map_err(terminal_error)? {
            break status.exit_code();
        }
        tokio::time::sleep(EXIT_POLL_INTERVAL).await;
    };
    terminal.close();

    Ok(TaskRunOutcome::Completed {
        task: task.name,
        command: task.command,
        status: Some(status),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn translator(locale: &str) -> Translator {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../locales");
        let mut translator = Translator::new(&dir).unwrap();
        assert!(translator.set_locale(locale));
        translator
    }

    #[test]
    fn parse_reads_named_commands_and_rejects_malformed_input() {
        let t = translator("en");
        let tasks = parse(
            "[tasks]\nbuild = \"cargo build\"\ntest = \"cargo test\"\n",
            &t,
        )
        .unwrap();
        assert_eq!(
            tasks,
            vec![
                Task {
                    name: "build".into(),
                    command: "cargo build".into(),
                },
                Task {
                    name: "test".into(),
                    command: "cargo test".into(),
                },
            ]
        );
        assert!(matches!(
            parse("[tasks]\nbuild = 7\n", &t),
            Err(TaskError::MalformedConfig { .. })
        ));
        assert!(matches!(
            parse("[tasks]\nbuild = \"  \"\n", &t),
            Err(TaskError::MalformedCommand { .. })
        ));
        assert!(matches!(
            parse("[tasks]\n\"\" = \"echo hi\"\n", &t),
            Err(TaskError::MalformedCommand { .. })
        ));
        assert!(matches!(
            parse("schema_version = 99\n[tasks]\nbuild = \"echo hi\"\n", &t),
            Err(TaskError::UnsupportedVersion { version: 99, .. })
        ));
    }

    #[test]
    fn task_errors_are_localized_in_both_bundles() {
        let en = translator("en");
        let vi = translator("vi");
        let missing_en = config_missing(&en, Path::new("/tmp/p/devdeck.toml")).to_string();
        let missing_vi = config_missing(&vi, Path::new("/tmp/p/devdeck.toml")).to_string();
        assert!(missing_en.contains("config is missing"), "{missing_en}");
        assert!(missing_vi.contains("Thiếu cấu hình"), "{missing_vi}");
        assert!(!missing_vi.contains("[task-"));
        let command_en =
            malformed_command(&en, "build", MessageKey::TaskReasonEmptyCommand).to_string();
        let command_vi =
            malformed_command(&vi, "build", MessageKey::TaskReasonEmptyCommand).to_string();
        assert!(command_en.contains("command is empty"), "{command_en}");
        assert!(command_vi.contains("lệnh để trống"), "{command_vi}");
    }
}
