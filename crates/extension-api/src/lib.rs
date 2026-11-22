//! Stable public types shared by Devdeck core, built-in modules and extensions.
//! Built-in features register through these SAME contribution points — no
//! private back doors.
#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Semver range of the extension API world (e.g. `^1.0.0`).
pub type ApiVersionRange = String;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum ActivationEvent {
    StartupFinished,
    Command { command: String },
    Language { language: String },
    FileGlob { glob: String },
    WorkspaceType { kind: String },
    ToolCall { tool: String },
}

/// A named capability the host may grant. Denied unless explicitly approved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum Permission {
    Fs { paths: Vec<String>, write: bool },
    Network { hosts: Vec<String> },
    ProcessSpawn { programs: Vec<String> },
    Env { names: Vec<String> },
    Secrets { names: Vec<String> },
    Clipboard,
    Database,
    AgentAccess,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum Contribution {
    Command {
        id: String,
        title_key: String,
        keybinding: Option<String>,
    },
    Tool {
        name: String,
        description_key: String,
        permission: String,
        risk: String,
    },
    Panel {
        id: String,
        title_key: String,
        placement: PanelPlacement,
    },
    StatusBarItem {
        id: String,
        text_key: String,
        alignment: StatusBarAlignment,
    },
    Theme {
        id: String,
        path: String,
    },
    Language {
        id: String,
        extensions: Vec<String>,
        tree_sitter_grammar: Option<String>,
        lsp: Option<String>,
    },
    ApiClientProvider {
        id: String,
        provider: String,
    },
    DatabaseDriver {
        id: String,
        scheme: String,
    },
    Agent {
        id: String,
        protocol: AgentProtocol,
    },
    SettingsPage {
        id: String,
        schema_path: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelPlacement {
    Left,
    Right,
    Bottom,
    Center,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusBarAlignment {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentProtocol {
    Acp,
    Mcp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtensionManifest {
    pub id: String,
    pub name: String,
    pub publisher: String,
    pub version: semver::Version,
    pub description: String,
    pub license: String,
    #[serde(default)]
    pub repository: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    pub devdeck_api: ApiVersionRange,
    #[serde(default)]
    pub activation_events: Vec<ActivationEvent>,
    #[serde(default)]
    pub contributions: Vec<Contribution>,
    #[serde(default)]
    pub permissions: Vec<Permission>,
    #[serde(default)]
    pub settings_schema: Option<String>,
    #[serde(default)]
    pub extension_dependencies: BTreeMap<String, String>,
    #[serde(default)]
    pub supported_platforms: Vec<String>,
    #[serde(default)]
    pub localized: BTreeMap<String, BTreeMap<String, String>>,
}

impl ExtensionManifest {
    pub fn from_toml(input: &str) -> Result<Self, ManifestError> {
        toml::from_str(input).map_err(ManifestError::Parse)
    }

    /// Permission diff used by the registry on update; escalation must be
    /// surfaced to the user, never accepted silently.
    pub fn added_permissions(&self, old: &Self) -> Vec<Permission> {
        self.permissions
            .iter()
            .filter(|p| !old.permissions.contains(p))
            .cloned()
            .collect()
    }
}

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("manifest TOML is invalid: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("API range '{0}' is not valid semver")]
    InvalidApiRange(String),
}

/// Validate the semver requirement at load time so incompatible extensions
/// fail with a clear error before any code is executed.
pub fn api_range_is_valid(range: &str) -> bool {
    semver::VersionReq::parse(range).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"
id = "acme.theme.night"
name = "Night Theme"
publisher = "acme"
version = "1.2.0"
description = "A dark color theme"
license = "MIT"
devdeck_api = "^1.0.0"
repository = "https://example.com/acme/theme"

[[contributions]]
kind = "theme"
value = { id = "night", path = "theme.json" }

[[contributions]]
kind = "command"
value = { id = "night.apply", title_key = "ext.night.apply", keybinding = "ctrl+shift+n" }

[[permissions]]
type = "env"
value = { names = ["TERM"] }
"#;

    #[test]
    fn parses_manifest_and_contributions() {
        let m = ExtensionManifest::from_toml(MANIFEST).unwrap();
        assert_eq!(m.id, "acme.theme.night");
        assert_eq!(m.version, semver::Version::new(1, 2, 0));
        assert_eq!(m.contributions.len(), 2);
        assert_eq!(m.permissions.len(), 1);
        assert!(api_range_is_valid(&m.devdeck_api));
    }

    #[test]
    fn permission_escalation_is_detected() {
        let old = ExtensionManifest::from_toml(MANIFEST).unwrap();
        let mut new = old.clone();
        new.permissions.push(Permission::Clipboard);
        assert_eq!(new.added_permissions(&old), vec![Permission::Clipboard]);
    }
}
