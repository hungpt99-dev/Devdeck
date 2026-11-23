//! Read-only detection of developer tools on this host.
//!
//! Detection runs `<tool> --version` in a child process with a restricted
//! environment, a short timeout and no writes. It never installs, upgrades,
//! generates shims or mutates PATH/the app-owned directory. Raw tool output is
//! reduced to a typed status and a sanitized version string; stderr is
//! discarded so no host detail can reach the UI or telemetry.
#![forbid(unsafe_code)]

use std::env;
use std::ffi::{OsStr, OsString};
use std::io::{self, Read};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Conservative upper bound for a version probe (the CLI flag only prints a
/// version string; anything slower is treated as a timeout).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_millis(1_500);
/// Maximum bytes read from a child's stdout. Bounds hostile or broken output.
const MAX_OUTPUT_BYTES: u64 = 4_096;
const MAX_VERSION_LINE: usize = 128;
const MAX_VERSION_TOKEN: usize = 32;
const POLL_INTERVAL: Duration = Duration::from_millis(5);

/// Fixed allow-list of tools Devdeck can report on in this slice.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ToolId {
    Cargo,
    Git,
    Go,
    Mise,
    Node,
    Python,
    Rustc,
}

impl ToolId {
    /// Alphabetical, hence deterministic, order used by [`Detector::detect_all`].
    pub const ALL: [Self; 7] = [
        Self::Cargo,
        Self::Git,
        Self::Go,
        Self::Mise,
        Self::Node,
        Self::Python,
        Self::Rustc,
    ];

    /// Executable name looked up in the (possibly fake) PATH.
    pub fn command(self) -> &'static str {
        match self {
            Self::Cargo => "cargo",
            Self::Git => "git",
            Self::Go => "go",
            Self::Mise => "mise",
            Self::Node => "node",
            Self::Python => "python",
            Self::Rustc => "rustc",
        }
    }

    /// Canonical, language-neutral name. UI text is localized separately.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Cargo => "Cargo",
            Self::Git => "Git",
            Self::Go => "Go",
            Self::Mise => "mise",
            Self::Node => "Node.js",
            Self::Python => "Python",
            Self::Rustc => "rustc",
        }
    }

    /// Stable i18n key for localizing this name in the Tool Center.
    pub fn name_key(self) -> &'static str {
        match self {
            Self::Cargo => "detection-tool-cargo",
            Self::Git => "detection-tool-git",
            Self::Go => "detection-tool-go",
            Self::Mise => "detection-tool-mise",
            Self::Node => "detection-tool-node",
            Self::Python => "detection-tool-python",
            Self::Rustc => "detection-tool-rustc",
        }
    }

    fn version_prefix(self) -> &'static str {
        match self {
            Self::Cargo => "cargo ",
            Self::Git => "git version ",
            Self::Go => "go version go",
            Self::Mise => "mise ",
            Self::Node => "v",
            Self::Python => "Python ",
            Self::Rustc => "rustc ",
        }
    }
}

/// Typed, closed set of reasons for a negative result. No raw OS error,
/// stderr, path or host detail is carried into the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DetectionFailure {
    /// No executable with that name on the probed PATH.
    NotFound,
    /// The executable exists but could not be started.
    SpawnFailed,
    /// The probe exceeded the configured timeout and the child was killed.
    Timeout,
    /// The probe produced output but no recognizable version token.
    UnparseableVersion,
    /// The probe exited with a non-zero status.
    ExitFailure,
}

impl DetectionFailure {
    /// Stable i18n key used by the Tool Center for a human-readable reason.
    pub fn reason_key(self) -> &'static str {
        match self {
            Self::NotFound => "detection-reason-not-found",
            Self::SpawnFailed => "detection-reason-spawn-failed",
            Self::Timeout => "detection-reason-timeout",
            Self::UnparseableVersion => "detection-reason-unparseable-version",
            Self::ExitFailure => "detection-reason-exit-failure",
        }
    }
}

/// Result of one probe. Serialized with a `status` tag so consumers cannot
/// confuse a missing version with a detected tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum DetectionStatus {
    Detected {
        /// Sanitized version token, e.g. `1.85.0`.
        version: String,
    },
    NotDetected {
        reason: DetectionFailure,
    },
}

impl DetectionStatus {
    pub fn is_detected(&self) -> bool {
        matches!(self, Self::Detected { .. })
    }
}

/// One detection result. Sorted by [`ToolId::ALL`] in batch results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DetectedTool {
    pub tool: ToolId,
    pub status: DetectionStatus,
}

impl DetectedTool {
    pub fn is_detected(&self) -> bool {
        self.status.is_detected()
    }

    pub fn version(&self) -> Option<&str> {
        match &self.status {
            DetectionStatus::Detected { version } => Some(version),
            DetectionStatus::NotDetected { .. } => None,
        }
    }

    pub fn failure(&self) -> Option<DetectionFailure> {
        match &self.status {
            DetectionStatus::Detected { .. } => None,
            DetectionStatus::NotDetected { reason } => Some(*reason),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum CommandError {
    NotFound,
    SpawnFailed,
    ExitFailure,
}

/// Read-only version prober. All knobs affect only the child process; the
/// parent environment and filesystem are never modified.
#[derive(Debug, Clone)]
pub struct Detector {
    timeout: Duration,
    path: Option<OsString>,
    home: Option<OsString>,
}

impl Default for Detector {
    fn default() -> Self {
        Self::new()
    }
}

impl Detector {
    pub fn new() -> Self {
        Self {
            timeout: DEFAULT_TIMEOUT,
            path: None,
            home: None,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Override the child's PATH. Tests point this at a fake-PATH fixture;
    /// production leaves it unset and inherits the host PATH.
    pub fn with_path(mut self, path: impl AsRef<OsStr>) -> Self {
        self.path = Some(path.as_ref().to_os_string());
        self
    }

    /// Override the child's HOME (USERPROFILE on Windows). Read-only in the
    /// tests; the detector itself never writes there.
    pub fn with_home(mut self, home: impl AsRef<OsStr>) -> Self {
        self.home = Some(home.as_ref().to_os_string());
        self
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Probe every allow-listed tool, in stable alphabetical order.
    pub fn detect_all(&self) -> Vec<DetectedTool> {
        ToolId::ALL.map(|tool| self.detect(tool)).to_vec()
    }

    /// Probe one tool. Never panics: every failure becomes a typed result.
    pub fn detect(&self, tool: ToolId) -> DetectedTool {
        let status = match self.run_version_probe(tool.command()) {
            Ok(Some(output)) => match parse_version(tool, &output) {
                Some(version) => DetectionStatus::Detected { version },
                None => DetectionStatus::NotDetected {
                    reason: DetectionFailure::UnparseableVersion,
                },
            },
            Ok(None) => DetectionStatus::NotDetected {
                reason: DetectionFailure::Timeout,
            },
            Err(CommandError::NotFound) => DetectionStatus::NotDetected {
                reason: DetectionFailure::NotFound,
            },
            Err(CommandError::SpawnFailed) => DetectionStatus::NotDetected {
                reason: DetectionFailure::SpawnFailed,
            },
            Err(CommandError::ExitFailure) => DetectionStatus::NotDetected {
                reason: DetectionFailure::ExitFailure,
            },
        };
        DetectedTool { tool, status }
    }

    fn run_version_probe(&self, executable: &str) -> Result<Option<String>, CommandError> {
        let mut command = Command::new(executable);
        command
            .arg("--version")
            .env_clear()
            .env(
                "PATH",
                self.path
                    .clone()
                    .unwrap_or_else(|| env::var_os("PATH").unwrap_or_else(|| OsString::from(""))),
            )
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            // Never surface raw stderr; it can contain paths or host details.
            .stderr(Stdio::null());

        #[cfg(windows)]
        {
            // Minimal variables Windows needs to launch a console process.
            for key in ["SystemRoot", "PATHEXT", "COMSPEC"] {
                if let Some(value) = env::var_os(key) {
                    command.env(key, value);
                }
            }
            if let Some(profile) = self.home.clone().or_else(|| env::var_os("USERPROFILE")) {
                command.env("USERPROFILE", profile);
            }
        }
        #[cfg(not(windows))]
        if let Some(home) = self.home.clone().or_else(|| env::var_os("HOME")) {
            command.env("HOME", home);
        }

        let mut child = command.spawn().map_err(|error| match error.kind() {
            io::ErrorKind::NotFound => CommandError::NotFound,
            _ => CommandError::SpawnFailed,
        })?;

        // `checked_add` avoids overflow panics for pathological timeouts.
        let deadline = Instant::now().checked_add(self.timeout);
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let mut output = Vec::new();
                    if let Some(mut stdout) = child.stdout.take() {
                        let _ = stdout
                            .by_ref()
                            .take(MAX_OUTPUT_BYTES)
                            .read_to_end(&mut output);
                    }
                    if !status.success() {
                        return Err(CommandError::ExitFailure);
                    }
                    return Ok(Some(String::from_utf8_lossy(&output).into_owned()));
                }
                Ok(None) => {
                    if deadline.is_some_and(|at| Instant::now() >= at) {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Ok(None);
                    }
                    thread::sleep(POLL_INTERVAL);
                }
                Err(_) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(CommandError::SpawnFailed);
                }
            }
        }
    }
}

/// Extract a sanitized version token from a typed tool's `--version` output.
/// Only the first non-empty line is considered; anything unexpected yields
/// `None` (typed as [`DetectionFailure::UnparseableVersion`]).
pub fn parse_version(tool: ToolId, output: &str) -> Option<String> {
    let line = output
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())?;
    if line.len() > MAX_VERSION_LINE {
        return None;
    }
    let tail = line.strip_prefix(tool.version_prefix())?;
    let token = tail.split_whitespace().next()?;
    if is_version_token(token) {
        Some(token.to_string())
    } else {
        None
    }
}

fn is_version_token(token: &str) -> bool {
    if token.is_empty() || token.len() > MAX_VERSION_TOKEN || !token.as_bytes()[0].is_ascii_digit()
    {
        return false;
    }
    let mut dots = 0;
    for byte in token.bytes() {
        match byte {
            b'0'..=b'9' => {}
            b'.' => dots += 1,
            b'-' | b'+' | b'a'..=b'z' | b'A'..=b'Z' => {}
            _ => return false,
        }
    }
    dots > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allow_list_is_fixed_and_sorted() {
        let ids = ToolId::ALL;
        let mut sorted = ids;
        sorted.sort();
        assert_eq!(ids, sorted);
        assert_eq!(ids.len(), 7);
        assert_eq!(
            ids.map(ToolId::command),
            ["cargo", "git", "go", "mise", "node", "python", "rustc"]
        );
    }

    #[test]
    fn parses_every_allow_listed_version_format() {
        let cases = [
            (ToolId::Cargo, "cargo 1.85.0 (d73d2caf9 2024-12-31)\n"),
            (ToolId::Git, "git version 2.43.0 (Apple Git-154)\n"),
            (ToolId::Go, "go version go1.22.5 linux/amd64\n"),
            (ToolId::Mise, "mise 2025.1.2 linux-x64 (abc)\n"),
            (ToolId::Node, "v22.13.1\n"),
            (ToolId::Python, "Python 3.12.3\n"),
            (ToolId::Rustc, "rustc 1.85.0 (abc 2025-01-01)\n"),
        ];
        for (tool, output) in cases {
            let version = parse_version(tool, output)
                .unwrap_or_else(|| panic!("{} version should parse", tool.command()));
            assert!(version.chars().next().is_some_and(|c| c.is_ascii_digit()));
            assert!(version.contains('.'));
            assert!(!version.contains("linux/"));
            assert!(version.len() <= MAX_VERSION_TOKEN);
        }
    }

    #[test]
    fn rejects_unparseable_version_output() {
        assert_eq!(parse_version(ToolId::Git, ""), None);
        assert_eq!(parse_version(ToolId::Git, "git version unknown\n"), None);
        assert_eq!(parse_version(ToolId::Node, "v22\n"), None);
        assert_eq!(parse_version(ToolId::Git, "cargo 1.2.3\n"), None);
        assert_eq!(
            parse_version(ToolId::Git, &format!("git version {}\n", "9".repeat(200))),
            None
        );
    }

    #[test]
    fn reason_keys_are_stable_i18n_keys() {
        let reasons = [
            DetectionFailure::NotFound,
            DetectionFailure::SpawnFailed,
            DetectionFailure::Timeout,
            DetectionFailure::UnparseableVersion,
            DetectionFailure::ExitFailure,
        ];
        for reason in reasons {
            assert!(reason.reason_key().starts_with("detection-reason-"));
        }
        for tool in ToolId::ALL {
            assert!(tool.name_key().starts_with("detection-tool-"));
        }
    }

    #[test]
    fn detector_defaults_are_read_only_and_bounded() {
        let detector = Detector::new();
        assert_eq!(detector.timeout(), DEFAULT_TIMEOUT);
        assert_eq!(Detector::default().timeout(), DEFAULT_TIMEOUT);
    }
}
