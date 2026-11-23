//! Typed tool schema layer — the single source of truth for every Devdeck
//! capability. UI, CLI, MCP server and agents all consume these definitions.
#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use devdeck_toolchain::{DetectedTool, ToolId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

/// Default policy applied when a tool is called. Workspace policies can raise
/// this (never lower it for high-risk tools).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PermissionLevel {
    Allow,
    Ask,
    Deny,
}

/// Blast radius of a tool call. Drives approval prompts and audit retention.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "kind", content = "id")]
pub enum ToolSource {
    BuiltIn,
    Extension { extension_id: String },
}

/// Canonical, language-neutral tool definition. Names and descriptions are
/// stable English strings; localized display text lives in `devdeck_i18n`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub output_schema: Value,
    pub permission: PermissionLevel,
    pub risk: RiskLevel,
    pub source: ToolSource,
    pub version: String,
    pub keybinding: Option<String>,
    pub command_palette_key: String,
    pub cli_subcommand: Option<String>,
    pub mcp_exposed: bool,
    pub activation_events: Vec<String>,
}

impl ToolDefinition {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: Value,
        permission: PermissionLevel,
        risk: RiskLevel,
        source: ToolSource,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
            output_schema: json!({ "type": "object" }),
            permission,
            risk,
            source,
            version: "1.0.0".into(),
            keybinding: None,
            command_palette_key: String::new(),
            cli_subcommand: None,
            mcp_exposed: true,
            activation_events: Vec::new(),
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RegistryError {
    #[error("tool name '{0}' is invalid: expected 'namespace.name' in lowercase")]
    InvalidName(String),
    #[error("tool '{0}' is already registered")]
    Duplicate(String),
    #[error("tool '{0}' was not found")]
    NotFound(String),
}

/// Every capability — built-in or extension-provided — is registered here.
#[derive(Debug, Default)]
pub struct ToolRegistry {
    tools: BTreeMap<String, ToolDefinition>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, tool: ToolDefinition) -> Result<(), RegistryError> {
        let parts: Vec<&str> = tool.name.split('.').collect();
        if parts.len() < 2
            || parts.iter().any(|p| p.is_empty())
            || tool
                .name
                .chars()
                .any(|c| c.is_uppercase() || c.is_whitespace())
        {
            return Err(RegistryError::InvalidName(tool.name.clone()));
        }
        if self.tools.contains_key(&tool.name) {
            return Err(RegistryError::Duplicate(tool.name.clone()));
        }
        self.tools.insert(tool.name.clone(), tool);
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&ToolDefinition> {
        self.tools.get(name)
    }

    pub fn list(&self) -> impl Iterator<Item = &ToolDefinition> {
        self.tools.values()
    }

    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Stable JSON representation for MCP `tools/list` and `devdeck --json`.
    pub fn to_mcp_json(&self) -> Value {
        Value::Array(
            self.tools
                .values()
                .map(|t| {
                    json!({
                        "name": t.name,
                        "description": t.description,
                        "inputSchema": t.input_schema,
                        "outputSchema": t.output_schema,
                        "annotations": {
                            "permission": t.permission,
                            "risk": t.risk,
                            "source": t.source,
                        }
                    })
                })
                .collect(),
        )
    }
}

/// Input of the first typed tool (Phase 1 acceptance criterion).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ReadFileInput {
    /// Path relative to the active workspace root.
    pub path: String,
    /// Refuse files larger than this many bytes; protects the UI thread.
    #[serde(default)]
    pub max_bytes: Option<u64>,
}

/// Build the canonical `files.read` tool definition, deriving its JSON Schema
/// from the typed input rather than hand-writing it.
pub fn files_read_tool() -> ToolDefinition {
    let mut tool = ToolDefinition::new(
        "files.read",
        "Read a UTF-8 text file inside the active workspace.",
        serde_json::to_value(schemars::schema_for!(ReadFileInput))
            .expect("JSON Schema serialization cannot fail"),
        PermissionLevel::Allow,
        RiskLevel::Low,
        ToolSource::BuiltIn,
    );
    tool.command_palette_key = "tool-files-read".into();
    tool.keybinding = Some("ctrl+o".into());
    tool.cli_subcommand = Some("read".into());
    tool.activation_events = vec!["workspace.opened".into()];
    tool
}

/// Input of `projects.open`: the workspace root to activate and remember.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct OpenProjectInput {
    /// Absolute or workspace-relative path to the project root.
    pub root: String,
}

/// Input of `projects.recent`; omitting `limit` returns the full bounded list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(default)]
pub struct RecentProjectsInput {
    /// Maximum number of entries to return (the store itself caps at 10).
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectSummary {
    pub root: String,
    /// Unix time in milliseconds.
    pub last_opened: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct OpenProjectOutput {
    pub root: String,
    pub languages: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RecentProjectsOutput {
    pub projects: Vec<ProjectSummary>,
}

/// Canonical `projects.open` tool: changes the active workspace and records a
/// recent project, so it is auditable through the normal tool-call path.
pub fn projects_open_tool() -> ToolDefinition {
    let mut tool = ToolDefinition::new(
        "projects.open",
        "Open a project workspace and add it to recent projects.",
        serde_json::to_value(schemars::schema_for!(OpenProjectInput))
            .expect("JSON Schema serialization cannot fail"),
        PermissionLevel::Allow,
        RiskLevel::Low,
        ToolSource::BuiltIn,
    );
    tool.output_schema = serde_json::to_value(schemars::schema_for!(OpenProjectOutput))
        .expect("JSON Schema serialization cannot fail");
    tool.command_palette_key = "tool-projects-open".into();
    tool.keybinding = Some("ctrl+shift+o".into());
    tool.cli_subcommand = Some("open".into());
    tool.mcp_exposed = true;
    tool.activation_events = vec!["workspace.opened".into()];
    tool
}

/// Canonical read-only `projects.recent` tool used by the shell and MCP.
pub fn projects_recent_tool() -> ToolDefinition {
    let mut tool = ToolDefinition::new(
        "projects.recent",
        "List recently opened project workspaces, newest first.",
        serde_json::to_value(schemars::schema_for!(RecentProjectsInput))
            .expect("JSON Schema serialization cannot fail"),
        PermissionLevel::Allow,
        RiskLevel::Low,
        ToolSource::BuiltIn,
    );
    tool.output_schema = serde_json::to_value(schemars::schema_for!(RecentProjectsOutput))
        .expect("JSON Schema serialization cannot fail");
    tool.command_palette_key = "tool-projects-recent".into();
    tool.keybinding = Some("ctrl+shift+r".into());
    tool.cli_subcommand = Some("recent".into());
    tool.mcp_exposed = true;
    tool.activation_events = Vec::new();
    tool
}

/// Input of `tools.detect`: one tool from the fixed allow-list. Callers
/// cannot name an arbitrary executable, so this tool cannot probe or reveal
/// unrelated host binaries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ToolsDetectInput {
    pub tool: ToolId,
}

/// Output of `tools.detect`: a typed status with a sanitized version or a
/// closed-set failure reason. Never contains raw stderr, paths or env data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ToolsDetectOutput {
    pub result: DetectedTool,
}

/// Input of `tools.list_detected`. Empty by design: detection is read-only and
/// takes no caller-supplied path or command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct ListDetectedInput {}

/// Output of `tools.list_detected`, sorted by canonical `ToolId` order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListDetectedOutput {
    pub tools: Vec<DetectedTool>,
}

/// Canonical read-only `tools.detect` tool. Detection is executed by
/// `devdeck_toolchain::Detector`; this definition is only the typed boundary.
pub fn tools_detect_tool() -> ToolDefinition {
    let mut tool = ToolDefinition::new(
        "tools.detect",
        "Check whether a known developer tool is installed on this host, read-only.",
        serde_json::to_value(schemars::schema_for!(ToolsDetectInput))
            .expect("JSON Schema serialization cannot fail"),
        PermissionLevel::Allow,
        RiskLevel::Low,
        ToolSource::BuiltIn,
    );
    tool.output_schema = serde_json::to_value(schemars::schema_for!(ToolsDetectOutput))
        .expect("JSON Schema serialization cannot fail");
    tool.command_palette_key = "tool-tools-detect".into();
    tool.cli_subcommand = Some("detect".into());
    tool.activation_events = vec!["workspace.opened".into()];
    tool
}

/// Canonical read-only `tools.list_detected` tool for the Tool Center and MCP.
pub fn tools_list_detected_tool() -> ToolDefinition {
    let mut tool = ToolDefinition::new(
        "tools.list_detected",
        "List known developer tools with their read-only detection status.",
        serde_json::to_value(schemars::schema_for!(ListDetectedInput))
            .expect("JSON Schema serialization cannot fail"),
        PermissionLevel::Allow,
        RiskLevel::Low,
        ToolSource::BuiltIn,
    );
    tool.output_schema = serde_json::to_value(schemars::schema_for!(ListDetectedOutput))
        .expect("JSON Schema serialization cannot fail");
    tool.command_palette_key = "tool-tools-list-detected".into();
    tool.cli_subcommand = Some("tools".into());
    tool.activation_events = vec!["workspace.opened".into()];
    tool
}

/// Input of `terminal.spawn`. The workspace cwd comes from the active
/// `WorkspaceInfo`, never from the caller, so a tool cannot escape the project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TerminalSpawnInput {
    /// Command line executed by the workspace shell.
    pub command: String,
    /// Initial terminal width in columns.
    pub cols: u16,
    /// Initial terminal height in rows.
    pub rows: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TerminalSpawnOutput {
    /// Present only when the command was allowed and a PTY was started.
    pub session_id: Option<String>,
    /// `allow`, `ask` or `deny` from the shared permission policy.
    pub decision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TerminalWriteInput {
    pub session_id: String,
    /// Raw PTY input (UTF-8), already inside an approved session.
    pub input: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TerminalResizeInput {
    pub session_id: String,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TerminalCloseInput {
    pub session_id: String,
}

/// Canonical `terminal.spawn` tool. Execution is gated by
/// `devdeck_core::PermissionPolicy::decide` in the terminal engine.
pub fn terminal_spawn_tool() -> ToolDefinition {
    let mut tool = ToolDefinition::new(
        "terminal.spawn",
        "Run a shell command in a PTY inside the active workspace.",
        serde_json::to_value(schemars::schema_for!(TerminalSpawnInput))
            .expect("JSON Schema serialization cannot fail"),
        PermissionLevel::Allow,
        RiskLevel::Medium,
        ToolSource::BuiltIn,
    );
    tool.output_schema = serde_json::to_value(schemars::schema_for!(TerminalSpawnOutput))
        .expect("JSON Schema serialization cannot fail");
    tool.command_palette_key = "tool-terminal-spawn".into();
    tool.keybinding = Some("ctrl+`".into());
    tool.cli_subcommand = Some("terminal".into());
    tool.activation_events = vec!["workspace.opened".into()];
    tool
}

pub fn terminal_write_tool() -> ToolDefinition {
    let mut tool = ToolDefinition::new(
        "terminal.write",
        "Send input to an active terminal session.",
        serde_json::to_value(schemars::schema_for!(TerminalWriteInput))
            .expect("JSON Schema serialization cannot fail"),
        PermissionLevel::Allow,
        RiskLevel::Medium,
        ToolSource::BuiltIn,
    );
    tool.command_palette_key = "tool-terminal-write".into();
    tool.activation_events = vec!["terminal.opened".into()];
    tool
}

pub fn terminal_resize_tool() -> ToolDefinition {
    let mut tool = ToolDefinition::new(
        "terminal.resize",
        "Resize an active terminal session.",
        serde_json::to_value(schemars::schema_for!(TerminalResizeInput))
            .expect("JSON Schema serialization cannot fail"),
        PermissionLevel::Allow,
        RiskLevel::Low,
        ToolSource::BuiltIn,
    );
    tool.command_palette_key = "tool-terminal-resize".into();
    tool.activation_events = vec!["terminal.opened".into()];
    tool
}

pub fn terminal_close_tool() -> ToolDefinition {
    let mut tool = ToolDefinition::new(
        "terminal.close",
        "Close an active terminal session.",
        serde_json::to_value(schemars::schema_for!(TerminalCloseInput))
            .expect("JSON Schema serialization cannot fail"),
        PermissionLevel::Allow,
        RiskLevel::Low,
        ToolSource::BuiltIn,
    );
    tool.command_palette_key = "tool-terminal-close".into();
    tool.activation_events = vec!["terminal.opened".into()];
    tool
}

/// Input of `tasks.list`: only the workspace root; tasks always live in that
/// project's `devdeck.toml`, never in a global or neighbouring project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TasksListInput {
    /// Absolute path to the workspace root that owns `devdeck.toml`.
    pub workspace: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TasksListOutput {
    /// Declared task names in stable (B-tree) order.
    pub names: Vec<String>,
}

/// Input of `tasks.run`. The command is resolved from the workspace config by
/// name; callers cannot inject an arbitrary command line. The cwd is the
/// workspace root, enforced by the F3 terminal engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TasksRunInput {
    /// Absolute path to the workspace root that owns `devdeck.toml`.
    pub workspace: String,
    /// Name of the task declared in `[tasks]`.
    pub task: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TasksRunOutput {
    pub task: String,
    /// Exact command string read from the config and sent to the F3 session.
    pub command: String,
    /// Numeric exit status when the task completed, or `None` when it was
    /// awaiting approval.
    pub status: Option<u32>,
    /// `allow`, `ask` or `deny` from the shared permission policy.
    pub decision: String,
}

/// Canonical `tasks.list` tool. Read-only, so its default permission is Allow.
pub fn tasks_list_tool() -> ToolDefinition {
    let mut tool = ToolDefinition::new(
        "tasks.list",
        "List named tasks declared in the workspace devdeck.toml.",
        serde_json::to_value(schemars::schema_for!(TasksListInput))
            .expect("JSON Schema serialization cannot fail"),
        PermissionLevel::Allow,
        RiskLevel::Low,
        ToolSource::BuiltIn,
    );
    tool.output_schema = serde_json::to_value(schemars::schema_for!(TasksListOutput))
        .expect("JSON Schema serialization cannot fail");
    tool.command_palette_key = "tool-tasks-list".into();
    tool.cli_subcommand = Some("tasks".into());
    tool.activation_events = vec!["workspace.opened".into()];
    tool
}

/// Canonical `tasks.run` tool. The command's approval is decided by
/// `devdeck_core::PermissionPolicy::decide` in the task runner before F3 is
/// allowed to start a PTY; dangerous commands are never auto-executed.
pub fn tasks_run_tool() -> ToolDefinition {
    let mut tool = ToolDefinition::new(
        "tasks.run",
        "Run a named task as a shell command in the workspace terminal.",
        serde_json::to_value(schemars::schema_for!(TasksRunInput))
            .expect("JSON Schema serialization cannot fail"),
        PermissionLevel::Allow,
        RiskLevel::Medium,
        ToolSource::BuiltIn,
    );
    tool.output_schema = serde_json::to_value(schemars::schema_for!(TasksRunOutput))
        .expect("JSON Schema serialization cannot fail");
    tool.command_palette_key = "tool-tasks-run".into();
    tool.cli_subcommand = Some("task".into());
    tool.activation_events = vec!["workspace.opened".into()];
    tool
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_tool_schema_is_generated() {
        let tool = files_read_tool();
        let schema = tool.input_schema.clone();
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["properties"]["path"]["type"], "string");
        assert_eq!(tool.name, "files.read");
        assert_eq!(tool.permission, PermissionLevel::Allow);
        assert_eq!(tool.risk, RiskLevel::Low);
    }

    #[test]
    fn registry_rejects_bad_and_duplicate_names() {
        let mut registry = ToolRegistry::new();
        registry.register(files_read_tool()).unwrap();
        assert!(matches!(
            registry.register(files_read_tool()),
            Err(RegistryError::Duplicate(_))
        ));
        let mut bad = files_read_tool();
        bad.name = "no_namespace".into();
        assert!(matches!(
            registry.register(bad),
            Err(RegistryError::InvalidName(_))
        ));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn mcp_export_contains_annotations() {
        let mut registry = ToolRegistry::new();
        registry.register(files_read_tool()).unwrap();
        let json = registry.to_mcp_json();
        assert_eq!(json[0]["name"], "files.read");
        assert_eq!(json[0]["annotations"]["risk"], "low");
    }

    #[test]
    fn project_tools_have_generated_schemas_and_full_metadata() {
        let open = projects_open_tool();
        let recent = projects_recent_tool();
        assert_eq!(open.name, "projects.open");
        assert_eq!(recent.name, "projects.recent");
        assert_eq!(open.input_schema["properties"]["root"]["type"], "string");
        assert!(recent.input_schema["properties"]["limit"]["type"]
            .as_array()
            .is_some_and(|types| types.iter().any(|t| t == "integer")));
        assert!(recent.input_schema["required"].is_null());
        assert_eq!(
            open.output_schema["properties"]["languages"]["type"],
            "array"
        );
        assert_eq!(
            recent.output_schema["properties"]["projects"]["type"],
            "array"
        );
        for tool in [&open, &recent] {
            assert_eq!(tool.permission, PermissionLevel::Allow);
            assert_eq!(tool.risk, RiskLevel::Low);
            assert_eq!(tool.source, ToolSource::BuiltIn);
            assert!(tool.mcp_exposed);
            assert!(tool.keybinding.is_some());
            assert!(tool.cli_subcommand.is_some());
            assert!(!tool.command_palette_key.is_empty());
        }
        let mut registry = ToolRegistry::new();
        registry.register(open).unwrap();
        registry.register(recent).unwrap();
        assert_eq!(registry.len(), 2);
        let json = registry.to_mcp_json();
        assert_eq!(json[0]["name"], "projects.open");
        assert_eq!(json[1]["name"], "projects.recent");
    }

    #[test]
    fn detection_tools_have_generated_schemas_and_register() {
        let detect = tools_detect_tool();
        let list = tools_list_detected_tool();
        assert_eq!(detect.name, "tools.detect");
        assert_eq!(list.name, "tools.list_detected");
        assert_eq!(
            detect.input_schema["properties"]["tool"]["$ref"],
            "#/$defs/ToolId"
        );
        assert_eq!(
            detect.input_schema["$defs"]["ToolId"]["enum"],
            json!(["cargo", "git", "go", "mise", "node", "python", "rustc"])
        );
        assert_eq!(
            detect.output_schema["properties"]["result"]["$ref"],
            "#/$defs/DetectedTool"
        );
        let reasons = detect.output_schema["$defs"]["DetectionFailure"]["oneOf"].clone();
        assert!(reasons.to_string().contains("unparseable_version"));
        assert_eq!(list.input_schema["type"], "object");
        assert_eq!(list.output_schema["properties"]["tools"]["type"], "array");
        assert_eq!(detect.permission, PermissionLevel::Allow);
        assert_eq!(list.permission, PermissionLevel::Allow);
        assert_eq!(detect.risk, RiskLevel::Low);
        assert_eq!(list.risk, RiskLevel::Low);
        assert!(detect.mcp_exposed && list.mcp_exposed);
        assert!(!detect.command_palette_key.is_empty());
        assert!(!list.command_palette_key.is_empty());
        let serialized = serde_json::to_string(&detect.output_schema).expect("serializable");
        assert!(!serialized.contains("\"stderr\""));
        assert!(!serialized.contains("\"command\""));
        let mut registry = ToolRegistry::new();
        registry.register(detect).unwrap();
        registry.register(list).unwrap();
        assert_eq!(registry.len(), 2);
        let json = registry.to_mcp_json();
        assert_eq!(json[0]["name"], "tools.detect");
        assert_eq!(json[1]["name"], "tools.list_detected");
        assert_eq!(json[0]["annotations"]["risk"], "low");
    }

    #[test]
    fn terminal_tools_have_generated_schemas_and_register() {
        let tools = [
            terminal_spawn_tool(),
            terminal_write_tool(),
            terminal_resize_tool(),
            terminal_close_tool(),
        ];
        assert_eq!(
            tools[0].input_schema["properties"]["command"]["type"],
            "string"
        );
        assert_eq!(
            tools[0].output_schema["properties"]["decision"]["type"],
            "string"
        );
        assert_eq!(
            tools[1].input_schema["properties"]["session_id"]["type"],
            "string"
        );
        assert_eq!(
            tools[2].input_schema["properties"]["cols"]["type"],
            "integer"
        );
        assert_eq!(
            tools[3].input_schema["properties"]["session_id"]["type"],
            "string"
        );
        let mut registry = ToolRegistry::new();
        for tool in tools {
            assert!(tool.mcp_exposed);
            assert!(!tool.command_palette_key.is_empty());
            registry.register(tool).unwrap();
        }
        assert_eq!(registry.len(), 4);
    }

    #[test]
    fn task_tools_have_generated_schemas_and_register() {
        let list = tasks_list_tool();
        let run = tasks_run_tool();
        assert_eq!(list.name, "tasks.list");
        assert_eq!(run.name, "tasks.run");
        assert_eq!(
            list.input_schema["properties"]["workspace"]["type"],
            "string"
        );
        assert_eq!(run.input_schema["properties"]["task"]["type"], "string");
        assert_eq!(list.output_schema["properties"]["names"]["type"], "array");
        assert_eq!(
            run.output_schema["properties"]["status"]["type"],
            json!(["integer", "null"])
        );
        assert_eq!(list.permission, PermissionLevel::Allow);
        assert_eq!(run.permission, PermissionLevel::Allow);
        assert_eq!(list.risk, RiskLevel::Low);
        assert_eq!(run.risk, RiskLevel::Medium);
        let mut registry = ToolRegistry::new();
        registry.register(list).unwrap();
        registry.register(run).unwrap();
        assert_eq!(registry.len(), 2);
        let json = registry.to_mcp_json();
        assert_eq!(json[0]["name"], "tasks.list");
        assert_eq!(json[1]["name"], "tasks.run");
    }
}
