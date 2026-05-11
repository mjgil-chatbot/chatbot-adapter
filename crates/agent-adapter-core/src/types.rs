//! Basic types for the agent adapter.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
/// Provider identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderId {
    Codex,
    Claude,
    MiniMax,
    Gemini,
    Kimi,
    OpenCode,
    Hermes,
    Forge,
    Claw,
    Custom(String),
}
/// Role in a conversation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    #[default]
    User,
    Assistant,
    System,
}
/// Log level.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}
/// File operation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileOperation {
    Create,
    Read,
    Update,
    Delete,
    Rename,
    Copy,
    Resume,
}
/// Risk level for permission requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    None,
    Low,
    Medium,
    High,
    Critical,
}
/// Plan item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanItem {
    pub id: String,
    pub description: String,
    pub status: String,
}
/// External session reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalSessionRef {
    pub provider: ProviderId,
    pub id: String,
}
/// Workspace reference for lifecycle events.
///
/// Captures the workspace identity including the path and access mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorkspaceRef {
    /// Direct current working directory.
    Direct { path: PathBuf },
    /// Temporary workspace.
    Temp { path: PathBuf },
    /// Isolated git worktree.
    IsolatedWorktree {
        path: PathBuf,
        branch: Option<String>,
    },
    /// Container-based workspace.
    Container {
        path: PathBuf,
        container_id: Option<String>,
    },
}
