// Shared data types exchanged between the Rust backend and the React
// frontend. All structs implement serde's Serialize/Deserialize so Tauri
// commands can return them directly and `invoke<T>()` on the JS side gets
// strongly-typed payloads.
//
// Field names are camelCase on the wire (via `#[serde(rename_all)]`) so the
// TypeScript side reads naturally, but snake_case in Rust so Clippy is happy.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// App mode (Desktop vs CLI)
// ---------------------------------------------------------------------------

/// Which Claude config the user is currently editing. Persisted to a simple
/// preferences file by the frontend — the backend is stateless and takes this
/// as a parameter on every command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppMode {
    Desktop,
    Cli,
}

impl AppMode {
    pub fn as_str(self) -> &'static str {
        match self {
            AppMode::Desktop => "desktop",
            AppMode::Cli => "cli",
        }
    }
}

impl Default for AppMode {
    fn default() -> Self {
        AppMode::Desktop
    }
}

// ---------------------------------------------------------------------------
// Server entries
// ---------------------------------------------------------------------------

/// Where a server lives — Active means it's in Claude's real config file and
/// will run at next restart; Stored means it's parked in Configonaut's own
/// sidecar file waiting to be turned on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServerSource {
    Active,
    Stored,
}

/// A single MCP server row in the UI. `config_json` is a pre-formatted
/// pretty-printed JSON snippet ready to drop into a TextArea — the React side
/// never has to re-stringify it to show the editor.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerEntry {
    pub name: String,
    pub config_json: String,
}

/// A group of MCP servers scoped to a specific project directory.
/// Only populated in CLI mode — Claude Code stores project-local servers
/// under `projects[path].mcpServers` in ~/.claude.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMcpGroup {
    pub project_path: String,
    pub servers: Vec<ServerEntry>,
}

/// Snapshot of the MCP Servers tab for a given mode — active, inactive, and
/// the target config file path (so the UI can show it as a breadcrumb).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerListing {
    pub active_servers: Vec<ServerEntry>,
    pub stored_servers: Vec<ServerEntry>,
    pub config_path: String,
    /// True if the Claude config file had a mutation since the last time the
    /// user relaunched Claude — the frontend uses this to show the "Restart
    /// Claude to apply" banner.
    pub needs_restart: bool,
    /// Project-scoped MCP servers (CLI mode only). Each group contains
    /// servers defined under `projects[path].mcpServers` in ~/.claude.json.
    pub project_groups: Vec<ProjectMcpGroup>,
}

// ---------------------------------------------------------------------------
// Hooks
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookRule {
    /// Stable id for React lists — index-based, regenerated on every load.
    pub id: String,
    pub event: String,
    pub matcher: String,
    pub commands: Vec<String>,
    pub is_enabled: bool,
}

// ---------------------------------------------------------------------------
// Backups
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupFile {
    /// Absolute path on disk — frontend never parses it, just round-trips it
    /// back to the restore / delete commands.
    pub path: String,
    /// Filename only (e.g. `config_2026-04-10_14-05-22.json`). Used as the
    /// React key.
    pub file_name: String,
    /// ISO-8601 UTC timestamp parsed from file metadata (creation time, or
    /// modified time as a fallback on filesystems without birthtime).
    pub created_at: String,
    /// Size in bytes. The frontend formats it for display.
    pub size_bytes: u64,
}

// ---------------------------------------------------------------------------
// Agents & Skills
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentSource {
    Personal,
    Plugin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentEntry {
    pub name: String,
    pub description: String,
    pub tools: Vec<String>,
    pub model: String,
    pub color: String,
    pub plugin_name: String,
    pub file_path: String,
    pub source: AgentSource,
    pub is_plugin_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillSource {
    Command,
    Skill,
    Plugin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillEntry {
    pub name: String,
    pub description: String,
    pub source: SkillSource,
    pub file_path: String,
    pub is_enabled: bool,
}

// ---------------------------------------------------------------------------
// Runtime prerequisite detection
// ---------------------------------------------------------------------------

/// Result of probing the user's PATH for runtimes that stdio MCP servers
/// depend on. Each field is `Some("v20.11.0")` if the tool is found, or
/// `None` if `which` / version check fails.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatus {
    pub node: Option<String>,
    pub python: Option<String>,
    pub uv: Option<String>,
    pub docker: Option<String>,
}

// ---------------------------------------------------------------------------
// Command errors — serialized to JS as plain strings
// ---------------------------------------------------------------------------

/// Lightweight error wrapper. Tauri serializes `Err` values to JS using
/// `Display`, so we keep this as a plain string.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Msg(String),
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError::Msg(format!("{e:#}"))
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Msg(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Msg(format!("JSON error: {e}"))
    }
}

impl serde::Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
