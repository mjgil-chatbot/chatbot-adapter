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

impl fmt::Display for ProviderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProviderId::Codex => write!(f, "codex"),
            ProviderId::Claude => write!(f, "claude"),
            ProviderId::MiniMax => write!(f, "minimax"),
            ProviderId::Gemini => write!(f, "gemini"),
            ProviderId::Kimi => write!(f, "kimi"),
            ProviderId::OpenCode => write!(f, "opencode"),
            ProviderId::Hermes => write!(f, "hermes"),
            ProviderId::Forge => write!(f, "forge"),
            ProviderId::Claw => write!(f, "claw"),
            ProviderId::Custom(name) => write!(f, "{}", name),
        }
    }
}

impl ProviderId {
    /// Returns true if this is a custom provider.
    pub fn is_custom(&self) -> bool {
        matches!(self, ProviderId::Custom(_))
    }
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

/// Resume mode for continuing sessions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum ResumeMode {
    #[default]
    None,
    ProviderDefault,
    External(ExternalSessionRef),
    LastInWorkspace,
}

/// Approval mode for dangerous operations.
///
/// **DEPRECATED**: This type is retained for backward compatibility only.
/// Providers must use `ApprovalPolicy` for all command construction.
/// `ApprovalMode` will be removed in a future release.
#[deprecated(since = "0.3.0", note = "Use ApprovalPolicy instead")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalMode {
    Ask,
    AutoEditOnly,
    AutoApprove,
    DenyTools,
}

#[allow(clippy::derivable_impls, deprecated)]
impl Default for ApprovalMode {
    fn default() -> Self {
        ApprovalMode::Ask
    }
}

/// Sandbox mode for workspace access.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxMode {
    ReadOnly,
    #[default]
    WorkspaceWrite,
    IsolatedWorktree,
    Container,
    DangerFullAccess,
}

impl SandboxMode {
    /// Returns true if this mode uses an isolated workspace.
    pub fn is_isolated(&self) -> bool {
        matches!(self, SandboxMode::IsolatedWorktree | SandboxMode::Container)
    }

    /// Returns true if this mode is read-only.
    pub fn is_read_only(&self) -> bool {
        matches!(self, SandboxMode::ReadOnly)
    }
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

impl WorkspaceRef {
    /// Creates a Direct workspace ref from a path and sandbox mode.
    pub fn from_cwd(path: PathBuf, sandbox: SandboxMode) -> Self {
        match sandbox {
            SandboxMode::IsolatedWorktree => WorkspaceRef::IsolatedWorktree { path, branch: None },
            SandboxMode::Container => WorkspaceRef::Container {
                path,
                container_id: None,
            },
            _ => WorkspaceRef::Direct { path },
        }
    }

    /// Returns the workspace path.
    pub fn path(&self) -> &PathBuf {
        match self {
            WorkspaceRef::Direct { path } => path,
            WorkspaceRef::Temp { path } => path,
            WorkspaceRef::IsolatedWorktree { path, .. } => path,
            WorkspaceRef::Container { path, .. } => path,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_id_display() {
        assert_eq!(ProviderId::Codex.to_string(), "codex");
        assert_eq!(ProviderId::MiniMax.to_string(), "minimax");
        assert_eq!(ProviderId::Custom("foo".to_string()).to_string(), "foo");
    }

    #[test]
    fn test_provider_id_is_custom() {
        assert!(!ProviderId::Claude.is_custom());
        assert!(ProviderId::Custom("foo".to_string()).is_custom());
    }

    #[test]
    fn test_resume_mode_default() {
        assert!(matches!(ResumeMode::default(), ResumeMode::None));
    }

    #[test]
    #[allow(deprecated)]
    fn test_approval_mode_default() {
        assert!(matches!(ApprovalMode::default(), ApprovalMode::Ask));
    }

    #[test]
    fn test_sandbox_mode_default() {
        assert!(matches!(
            SandboxMode::default(),
            SandboxMode::WorkspaceWrite
        ));
    }

    #[test]
    fn test_sandbox_mode_is_isolated() {
        assert!(!SandboxMode::ReadOnly.is_isolated());
        assert!(!SandboxMode::WorkspaceWrite.is_isolated());
        assert!(SandboxMode::IsolatedWorktree.is_isolated());
        assert!(SandboxMode::Container.is_isolated());
        assert!(!SandboxMode::DangerFullAccess.is_isolated());
    }

    #[test]
    fn test_sandbox_mode_is_read_only() {
        assert!(SandboxMode::ReadOnly.is_read_only());
        assert!(!SandboxMode::WorkspaceWrite.is_read_only());
        assert!(!SandboxMode::IsolatedWorktree.is_read_only());
        assert!(!SandboxMode::Container.is_read_only());
        assert!(!SandboxMode::DangerFullAccess.is_read_only());
    }

    #[test]
    fn test_workspace_ref_direct() {
        let path = PathBuf::from("/workspace");
        let refirect = WorkspaceRef::from_cwd(path.clone(), SandboxMode::WorkspaceWrite);
        assert!(matches!(refirect, WorkspaceRef::Direct { path: p } if p == path));
    }

    #[test]
    fn test_workspace_ref_isolated_worktree() {
        let path = PathBuf::from("/workspace");
        let refirect = WorkspaceRef::from_cwd(path.clone(), SandboxMode::IsolatedWorktree);
        assert!(matches!(
            refirect,
            WorkspaceRef::IsolatedWorktree { path: p, .. } if p == path
        ));
    }

    #[test]
    fn test_workspace_ref_container() {
        let path = PathBuf::from("/workspace");
        let refirect = WorkspaceRef::from_cwd(path.clone(), SandboxMode::Container);
        assert!(matches!(
            refirect,
            WorkspaceRef::Container { path: p, .. } if p == path
        ));
    }

    #[test]
    fn test_workspace_ref_path() {
        let path = PathBuf::from("/test/workspace");
        let refirect = WorkspaceRef::Direct { path: path.clone() };
        assert_eq!(refirect.path(), &path);
    }

    #[test]
    fn test_workspace_ref_serde() {
        let workspace = WorkspaceRef::Direct {
            path: PathBuf::from("/workspace"),
        };
        let json = serde_json::to_string(&workspace).unwrap();
        let parsed: WorkspaceRef = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, WorkspaceRef::Direct { .. }));
    }

    #[test]
    fn test_workspace_ref_container_with_id() {
        let workspace = WorkspaceRef::Container {
            path: PathBuf::from("/workspace"),
            container_id: Some("abc123".to_string()),
        };
        let json = serde_json::to_string(&workspace).unwrap();
        let parsed: WorkspaceRef = serde_json::from_str(&json).unwrap();
        assert!(matches!(
            parsed,
            WorkspaceRef::Container {
                container_id: Some(id),
                ..
            } if id == "abc123"
        ));
    }
}
