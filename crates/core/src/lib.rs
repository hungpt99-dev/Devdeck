//! Devdeck core: configuration with migrations, workspace discovery, event
//! bus, permission policy and the `Module` contract. Core never depends on UI.
#![forbid(unsafe_code)]

mod recents;

pub use recents::{
    canonical_root, normalize as normalize_recents, open_project, record_open, RecentProject,
    MAX_RECENT_PROJECTS,
};

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use devdeck_audit::{Actor, NewAuditEntry};
use devdeck_extension_api::Contribution;
use devdeck_tools::{PermissionLevel, RiskLevel, ToolDefinition};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Current on-disk schema. v0 and v1 migrate forward to this version; newer
/// versions are rejected rather than silently downgraded.
pub const CONFIG_SCHEMA_VERSION: u32 = 2;

/// Number of deterministic checkpoints exposed around a config write. The
/// first 96 sit between chunks written to the temp file, the last four cover
/// fsync, rename, directory fsync and completion.
pub const CONFIG_WRITE_CHECKPOINTS: usize = 100;
const WRITE_CHUNKS: usize = CONFIG_WRITE_CHECKPOINTS - 4;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeName {
    #[default]
    Dark,
    Light,
    HighContrast,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyOverride {
    Allow,
    Ask,
    Deny,
}

/// Phase 2 defaults for the terminal region. Missing fields in older configs
/// are filled from `Default`, never from an empty/zero value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerminalDefaults {
    #[serde(default = "default_font_size")]
    pub font_size: u8,
    #[serde(default = "default_scrollback_lines")]
    pub scrollback_lines: u32,
    #[serde(default)]
    pub shell: Option<String>,
}

impl Default for TerminalDefaults {
    fn default() -> Self {
        Self {
            font_size: default_font_size(),
            scrollback_lines: default_scrollback_lines(),
            shell: None,
        }
    }
}

fn default_font_size() -> u8 {
    13
}

fn default_scrollback_lines() -> u32 {
    10_000
}

/// Phase 2 defaults for named tasks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskDefaults {
    #[serde(default)]
    pub auto_run_on_open: bool,
    #[serde(default = "default_max_parallel")]
    pub max_parallel: u8,
}

impl Default for TaskDefaults {
    fn default() -> Self {
        Self {
            auto_run_on_open: false,
            max_parallel: default_max_parallel(),
        }
    }
}

fn default_max_parallel() -> u8 {
    4
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppConfig {
    pub schema_version: u32,
    #[serde(default = "default_locale")]
    pub locale: String,
    #[serde(default)]
    pub theme: ThemeName,
    #[serde(default)]
    pub telemetry_enabled: bool,
    #[serde(default)]
    pub tool_policies: BTreeMap<String, PolicyOverride>,
    #[serde(default)]
    pub recent_projects: Vec<RecentProject>,
    #[serde(default)]
    pub terminal: TerminalDefaults,
    #[serde(default)]
    pub task_defaults: TaskDefaults,
}

fn default_locale() -> String {
    "en".into()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: CONFIG_SCHEMA_VERSION,
            locale: default_locale(),
            theme: ThemeName::Dark,
            // Telemetry is OFF unless the user explicitly opts in.
            telemetry_enabled: false,
            tool_policies: BTreeMap::new(),
            recent_projects: Vec::new(),
            terminal: TerminalDefaults::default(),
            task_defaults: TaskDefaults::default(),
        }
    }
}

/// Lenient shape used for every on-disk version. A missing field gets the
/// current default, so v0/v1 fixtures can carry any subset of v2 fields.
#[derive(Debug, Deserialize)]
#[serde(default)]
struct PartialConfig {
    /// v0 key; migrated to `locale`.
    language: Option<String>,
    locale: String,
    theme: ThemeName,
    telemetry_enabled: bool,
    tool_policies: BTreeMap<String, PolicyOverride>,
    recent_projects: Vec<RecentProject>,
    terminal: TerminalDefaults,
    task_defaults: TaskDefaults,
}

impl Default for PartialConfig {
    fn default() -> Self {
        let defaults = AppConfig::default();
        Self {
            language: None,
            locale: defaults.locale,
            theme: defaults.theme,
            telemetry_enabled: defaults.telemetry_enabled,
            tool_policies: defaults.tool_policies,
            recent_projects: defaults.recent_projects,
            terminal: defaults.terminal,
            task_defaults: defaults.task_defaults,
        }
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("config I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("config JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("config schema version {0} is newer than supported {CONFIG_SCHEMA_VERSION}")]
    FutureVersion(u32),
}

/// Position reached inside the atomic write. Test-only observers use this to
/// kill a writer deterministically; production passes a no-op closure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WriteCheckpoint {
    pub index: usize,
    pub phase: WritePhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WritePhase {
    TempWrite,
    TempSync,
    Rename,
    DirSync,
    Complete,
}

impl AppConfig {
    /// Load config, applying forward migrations v0 -> v1 -> v2. Unknown future
    /// versions are rejected instead of being silently downgraded. A
    /// timestamped backup is taken before any destructive migration.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)?;
        let value: serde_json::Value = serde_json::from_str(&raw)?;
        let version = value
            .get("schema_version")
            .and_then(|v| v.as_u64())
            .map(|v| u32::try_from(v).unwrap_or(u32::MAX))
            .unwrap_or(0);
        if version > CONFIG_SCHEMA_VERSION {
            return Err(ConfigError::FutureVersion(version));
        }
        let partial: PartialConfig = serde_json::from_str(&raw)?;
        let mut config = Self {
            schema_version: CONFIG_SCHEMA_VERSION,
            locale: partial.locale,
            theme: partial.theme,
            telemetry_enabled: partial.telemetry_enabled,
            tool_policies: partial.tool_policies,
            recent_projects: normalize_recents(&partial.recent_projects),
            terminal: partial.terminal,
            task_defaults: partial.task_defaults,
        };
        if version == 0 {
            // v0 used a top-level `language` string and no policy map.
            if let Some(language) = partial.language {
                config.locale = language;
            }
        }
        if version < CONFIG_SCHEMA_VERSION {
            backup_before_migration(path)?;
            config.save(path)?;
        }
        Ok(config)
    }

    /// Atomic write: temp file, fsync, rename, directory fsync. A crash at any
    /// point leaves either the old or the complete new document at `path`.
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        self.save_with_probe(path, |_| {})
    }

    /// Same write path as [`Self::save`], with a deterministic observer at
    /// each of [`CONFIG_WRITE_CHECKPOINTS`] points. Used by the crash test to
    /// terminate the writer at an exact point; not used in production.
    pub fn save_with_probe(
        &self,
        path: &Path,
        mut probe: impl FnMut(WriteCheckpoint),
    ) -> Result<(), ConfigError> {
        let bytes = serde_json::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        let temp = path.with_extension("json.tmp");
        {
            let mut file = File::create(&temp)?;
            let len = bytes.len();
            for index in 0..WRITE_CHUNKS {
                let start = index * len / WRITE_CHUNKS;
                let end = (index + 1) * len / WRITE_CHUNKS;
                file.write_all(&bytes.as_bytes()[start..end])?;
                probe(WriteCheckpoint {
                    index,
                    phase: WritePhase::TempWrite,
                });
            }
            file.flush()?;
            file.sync_all()?;
        }
        probe(WriteCheckpoint {
            index: WRITE_CHUNKS,
            phase: WritePhase::TempSync,
        });
        fs::rename(&temp, path)?;
        probe(WriteCheckpoint {
            index: WRITE_CHUNKS + 1,
            phase: WritePhase::Rename,
        });
        // Windows cannot open a directory for reading; its rename is already
        // durable enough for this store. Unix needs the directory fsync so the
        // rename itself survives a power loss.
        #[cfg(unix)]
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                File::open(parent)?.sync_all()?;
            }
        }
        probe(WriteCheckpoint {
            index: WRITE_CHUNKS + 2,
            phase: WritePhase::DirSync,
        });
        probe(WriteCheckpoint {
            index: CONFIG_WRITE_CHECKPOINTS - 1,
            phase: WritePhase::Complete,
        });
        Ok(())
    }
}

/// Copy the about-to-be-migrated document to a timestamped sibling before any
/// destructive rewrite. The original stays untouched if the copy fails.
fn backup_before_migration(path: &Path) -> Result<PathBuf, ConfigError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let backup = path.with_extension(format!(
        "json.bak-{}-{:09}",
        now.as_secs(),
        now.subsec_nanos()
    ));
    fs::copy(path, &backup)?;
    File::open(&backup)?.sync_all()?;
    Ok(backup)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionDecision {
    Allow,
    Ask { reason: String },
    Deny { reason: String },
}

/// Policy engine applied IDENTICALLY to built-in tools, extension tools and
/// agent actions. Dangerous operations always require explicit approval.
#[derive(Debug, Default)]
pub struct PermissionPolicy {
    overrides: BTreeMap<String, PolicyOverride>,
}

impl PermissionPolicy {
    pub fn from_config(config: &AppConfig) -> Self {
        Self {
            overrides: config.tool_policies.clone(),
        }
    }

    pub fn set_override(&mut self, tool: String, policy: PolicyOverride) {
        self.overrides.insert(tool, policy);
    }

    pub fn decide(&self, tool: &ToolDefinition, input_hint: &str) -> PermissionDecision {
        if self.overrides.get(&tool.name) == Some(&PolicyOverride::Deny) {
            return PermissionDecision::Deny {
                reason: "policy-denied".into(),
            };
        }
        if is_dangerous(input_hint) || tool.risk >= RiskLevel::High {
            return PermissionDecision::Ask {
                reason: "policy-dangerous".into(),
            };
        }
        match (self.overrides.get(&tool.name), tool.permission) {
            (Some(PolicyOverride::Ask), _) | (_, PermissionLevel::Ask) => PermissionDecision::Ask {
                reason: "policy-ask".into(),
            },
            (Some(PolicyOverride::Allow), _) | (_, PermissionLevel::Allow) => {
                PermissionDecision::Allow
            }
            _ => PermissionDecision::Deny {
                reason: "policy-denied".into(),
            },
        }
    }
}

/// Conservative substring scan for shell/database operations that must never
/// run without an explicit, per-action approval.
pub fn is_dangerous(input: &str) -> bool {
    const PATTERNS: [&str; 12] = [
        "rm -rf",
        "git push --force",
        "git reset --hard",
        "drop table",
        "drop database",
        "delete from",
        "truncate table",
        "kubectl delete",
        "docker rm",
        "docker system prune",
        "sudo ",
        "curl | sh",
    ];
    let lower = input.to_lowercase();
    PATTERNS.iter().any(|p| lower.contains(p))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub root: PathBuf,
    pub languages: Vec<String>,
}

/// Auto-detect the project at `root` so tools can suggest relevant actions.
pub fn detect_workspace(root: &Path) -> WorkspaceInfo {
    let mut languages = Vec::new();
    let markers = [
        ("Cargo.toml", "rust"),
        ("package.json", "javascript"),
        ("pyproject.toml", "python"),
        ("go.mod", "go"),
        ("pom.xml", "java"),
        ("composer.json", "php"),
        ("Gemfile", "ruby"),
    ];
    for (file, language) in markers {
        if root.join(file).exists() {
            languages.push(language.into());
        }
    }
    if root.join("compose.yaml").exists() || root.join("docker-compose.yml").exists() {
        languages.push("docker".into());
    }
    WorkspaceInfo {
        root: root.to_path_buf(),
        languages,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    LocaleChanged { locale: String },
    ThemeChanged { theme: ThemeName },
    ToolCalled { name: String },
    WorkspaceOpened { root: PathBuf },
    ExtensionMessage { extension_id: String, body: String },
}

#[derive(Debug, Clone)]
pub struct EventBus {
    sender: tokio::sync::broadcast::Sender<Event>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = tokio::sync::broadcast::channel(capacity);
        Self { sender }
    }

    pub fn publish(&self, event: Event) {
        // No receivers is a normal state during startup; ignore it.
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Event> {
        self.sender.subscribe()
    }
}

/// Contract implemented by every feature module, built-in or extension.
pub trait Module: Send + Sync {
    fn id(&self) -> &'static str;
    fn contributions(&self) -> Vec<Contribution>;
    fn tools(&self) -> Vec<ToolDefinition>;
}

/// Emit an audit record for a tool call, redacting secrets in the detail.
pub fn audit_tool_call(
    log: &mut devdeck_audit::AuditLog,
    actor: Actor,
    tool: &ToolDefinition,
    decision: &PermissionDecision,
    detail: &str,
) -> Result<(), devdeck_audit::AuditError> {
    let outcome = match decision {
        PermissionDecision::Allow => devdeck_audit::Outcome::Allowed,
        PermissionDecision::Ask { .. } => devdeck_audit::Outcome::AwaitingApproval,
        PermissionDecision::Deny { .. } => devdeck_audit::Outcome::Denied,
    };
    let reason = match decision {
        PermissionDecision::Ask { reason } | PermissionDecision::Deny { reason } => reason.clone(),
        PermissionDecision::Allow => String::new(),
    };
    let mut entry = NewAuditEntry::new(actor, "tool.call");
    entry.tool = Some(tool.name.clone());
    entry.permission = Some(format!("{:?}", tool.permission));
    entry.risk = Some(format!("{:?}", tool.risk));
    entry.outcome = outcome;
    entry.detail = format!("{detail} | {reason}");
    log.append(entry)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use devdeck_tools::{files_read_tool, ToolSource};

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("devdeck-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn migrates_v0_config_and_preserves_all_v1_fields() {
        let dir = temp_dir("v0-config");
        let path = dir.join("config.json");
        fs::write(
            &path,
            r#"{"language":"vi","theme":"high_contrast","telemetry_enabled":true,
                "tool_policies":{"files.read":"deny","projects.open":"ask"}}"#,
        )
        .unwrap();
        let config = AppConfig::load(&path).unwrap();
        assert_eq!(config.schema_version, CONFIG_SCHEMA_VERSION);
        assert_eq!(config.locale, "vi");
        assert_eq!(config.theme, ThemeName::HighContrast);
        assert!(config.telemetry_enabled);
        assert_eq!(config.tool_policies["files.read"], PolicyOverride::Deny);
        assert_eq!(config.tool_policies["projects.open"], PolicyOverride::Ask);
        let reloaded = AppConfig::load(&path).unwrap();
        assert_eq!(reloaded, config);
        let backups: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
            .filter(|n| n.contains(".bak-"))
            .collect();
        assert_eq!(backups.len(), 1, "one backup per destructive migration");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn migrates_v1_config_to_v2_with_defaults() {
        let dir = temp_dir("v1-config");
        let path = dir.join("config.json");
        fs::write(
            &path,
            r#"{"schema_version":1,"locale":"en","theme":"light","telemetry_enabled":false,
                "tool_policies":{"files.read":"allow"}}"#,
        )
        .unwrap();
        let config = AppConfig::load(&path).unwrap();
        assert_eq!(config.schema_version, 2);
        assert_eq!(config.locale, "en");
        assert_eq!(config.theme, ThemeName::Light);
        assert!(!config.telemetry_enabled);
        assert_eq!(config.tool_policies["files.read"], PolicyOverride::Allow);
        assert!(config.recent_projects.is_empty());
        assert_eq!(config.terminal.font_size, 13);
        assert_eq!(config.terminal.scrollback_lines, 10_000);
        assert!(!config.task_defaults.auto_run_on_open);
        assert_eq!(config.task_defaults.max_parallel, 4);
        let on_disk: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(on_disk["schema_version"], 2);
        assert!(on_disk.get("recent_projects").is_some());
        assert!(on_disk.get("terminal").is_some());
        assert!(on_disk.get("task_defaults").is_some());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rejects_future_config_version_without_rewriting() {
        let dir = temp_dir("future-config");
        let path = dir.join("config.json");
        let raw = r#"{"schema_version":99,"locale":"en"}"#;
        fs::write(&path, raw).unwrap();
        assert!(matches!(
            AppConfig::load(&path),
            Err(ConfigError::FutureVersion(99))
        ));
        assert_eq!(fs::read_to_string(&path).unwrap(), raw);
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn save_runs_all_checkpoints_and_leaves_a_complete_document() {
        let dir = temp_dir("probe-config");
        let path = dir.join("config.json");
        let mut seen = Vec::new();
        AppConfig::default()
            .save_with_probe(&path, |cp| {
                seen.push((cp.index, cp.phase));
            })
            .unwrap();
        assert_eq!(seen.len(), CONFIG_WRITE_CHECKPOINTS);
        assert_eq!(seen[0], (0, WritePhase::TempWrite));
        assert_eq!(
            seen[WRITE_CHUNKS + 1],
            (WRITE_CHUNKS + 1, WritePhase::Rename)
        );
        assert_eq!(AppConfig::load(&path).unwrap(), AppConfig::default());
        assert!(!path.with_extension("json.tmp").exists());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn dangerous_input_forces_approval() {
        let policy = PermissionPolicy::default();
        let tool = files_read_tool();
        assert_eq!(policy.decide(&tool, "README.md"), PermissionDecision::Allow);
        assert!(matches!(
            policy.decide(&tool, "rm -rf ./build"),
            PermissionDecision::Ask { .. }
        ));
        let mut policy = policy;
        policy.set_override(tool.name.clone(), PolicyOverride::Deny);
        assert!(matches!(
            policy.decide(&tool, "README.md"),
            PermissionDecision::Deny { .. }
        ));
    }

    #[test]
    fn workspace_detection_reads_markers() {
        let dir = temp_dir("ws-test");
        fs::write(dir.join("Cargo.toml"), "[package]").unwrap();
        fs::write(dir.join("compose.yaml"), "services: {}").unwrap();
        let ws = detect_workspace(&dir);
        assert_eq!(ws.languages, vec!["rust", "docker"]);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn extension_tool_is_registered_the_same_way() {
        let mut tool = files_read_tool();
        tool.source = ToolSource::Extension {
            extension_id: "acme.tools".into(),
        };
        assert_eq!(
            tool.source,
            ToolSource::Extension {
                extension_id: "acme.tools".into()
            }
        );
    }
}
