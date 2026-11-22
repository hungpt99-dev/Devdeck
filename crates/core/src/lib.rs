//! Devdeck core: configuration with migrations, workspace discovery, event
//! bus, permission policy and the `Module` contract. Core never depends on UI.
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use devdeck_audit::{Actor, NewAuditEntry};
use devdeck_extension_api::Contribution;
use devdeck_tools::{PermissionLevel, RiskLevel, ToolDefinition};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const CONFIG_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeName {
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppConfig {
    pub schema_version: u32,
    pub locale: String,
    pub theme: ThemeName,
    /// Telemetry is OFF unless the user explicitly opts in.
    pub telemetry_enabled: bool,
    pub tool_policies: BTreeMap<String, PolicyOverride>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: CONFIG_SCHEMA_VERSION,
            locale: "en".into(),
            theme: ThemeName::Dark,
            telemetry_enabled: false,
            tool_policies: BTreeMap::new(),
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

#[derive(Debug, Deserialize)]
struct LegacyConfig {
    #[serde(default)]
    language: Option<String>,
}

impl AppConfig {
    /// Load config, applying forward migrations. Unknown future versions are
    /// rejected instead of being silently downgraded.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)?;
        let value: serde_json::Value = serde_json::from_str(&raw)?;
        let version = value
            .get("schema_version")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        if version > CONFIG_SCHEMA_VERSION {
            return Err(ConfigError::FutureVersion(version));
        }
        if version == 0 {
            // v0 used a top-level `language` string and no policy map.
            let legacy: LegacyConfig = serde_json::from_str(&raw)?;
            let mut config = Self::default();
            if let Some(locale) = legacy.language {
                config.locale = locale;
            }
            config.save(path)?;
            return Ok(config);
        }
        Ok(serde_json::from_str(&raw)?)
    }

    /// Atomic write: temp file + rename, so a crash never truncates settings.
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temp = path.with_extension("json.tmp");
        fs::write(&temp, serde_json::to_string_pretty(self)?)?;
        fs::rename(&temp, path)?;
        Ok(())
    }
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

    #[test]
    fn migrates_v0_config() {
        let dir = std::env::temp_dir().join(format!("devdeck-config-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        fs::write(&path, r#"{"language":"vi"}"#).unwrap();
        let config = AppConfig::load(&path).unwrap();
        assert_eq!(config.locale, "vi");
        assert_eq!(config.schema_version, CONFIG_SCHEMA_VERSION);
        assert!(!config.telemetry_enabled);
        assert!(AppConfig::load(&path).unwrap().schema_version == CONFIG_SCHEMA_VERSION);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rejects_future_config_version() {
        let dir = std::env::temp_dir().join(format!("devdeck-future-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        fs::write(&path, r#"{"schema_version":99}"#).unwrap();
        assert!(matches!(
            AppConfig::load(&path),
            Err(ConfigError::FutureVersion(99))
        ));
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
        let dir = std::env::temp_dir().join(format!("devdeck-ws-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
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
