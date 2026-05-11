//! Request types for the agent adapter.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::types::{ExternalSessionRef, ProviderId, ResumeMode, SandboxMode};

/// Chat request sent to an agent provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// The prompt to send to the agent.
    pub prompt: String,

    /// Current working directory.
    pub cwd: PathBuf,

    /// Target provider.
    pub provider: ProviderId,

    /// Specific model to use (provider-specific).
    pub model: Option<String>,

    /// Session to resume.
    pub session: Option<ExternalSessionRef>,

    /// Resume mode.
    pub resume: ResumeMode,

    /// Files to include in context.
    pub files: Vec<PathBuf>,

    /// Images to include (multimodal providers only).
    pub images: Vec<PathBuf>,

    /// Approval policy for dangerous operations.
    pub approval_policy: ApprovalPolicy,

    /// Sandbox mode for workspace access.
    pub sandbox: SandboxMode,

    /// Timeout in milliseconds.
    pub timeout_ms: Option<u64>,

    /// Environment variables to pass (whitelist).
    pub env: HashMap<String, String>,

    /// Provider-specific overrides.
    pub extra: serde_json::Value,
}

impl ChatRequest {
    /// Creates a new chat request.
    pub fn new(prompt: impl Into<String>, cwd: impl Into<PathBuf>, provider: ProviderId) -> Self {
        Self {
            prompt: prompt.into(),
            cwd: cwd.into(),
            provider,
            model: None,
            session: None,
            resume: ResumeMode::None,
            files: Vec::new(),
            images: Vec::new(),
            approval_policy: ApprovalPolicy::DangerousDefault,
            sandbox: SandboxMode::WorkspaceWrite,
            timeout_ms: None,
            env: HashMap::new(),
            extra: serde_json::Value::Null,
        }
    }

    /// Creates a new chat request with session resumption.
    pub fn with_resume(
        prompt: impl Into<String>,
        cwd: impl Into<PathBuf>,
        provider: ProviderId,
        resume: ResumeMode,
    ) -> Self {
        let mut req = Self::new(prompt, cwd, provider);
        req.resume = resume;
        req
    }

    /// Creates a new chat request for resuming an external session.
    pub fn resume_external(
        prompt: impl Into<String>,
        cwd: impl Into<PathBuf>,
        provider: ProviderId,
        session_id: impl Into<String>,
    ) -> Self {
        let session_provider = provider.clone();
        let mut req = Self::new(prompt, cwd, provider);
        req.resume = ResumeMode::External(ExternalSessionRef {
            provider: session_provider,
            id: session_id.into(),
        });
        req
    }

    /// Returns true if the request includes images.
    pub fn has_images(&self) -> bool {
        !self.images.is_empty()
    }

    /// Returns true if the request has a timeout set.
    pub fn has_timeout(&self) -> bool {
        self.timeout_ms.is_some()
    }

    /// Returns true if the workspace should be isolated (safe mode).
    pub fn needs_isolated_workspace(&self) -> bool {
        matches!(
            self.sandbox,
            SandboxMode::IsolatedWorktree | SandboxMode::Container
        )
    }

    /// Returns true if the workspace is read-only.
    pub fn is_read_only(&self) -> bool {
        matches!(self.sandbox, SandboxMode::ReadOnly)
    }

    /// Returns the workspace path (the cwd).
    pub fn workspace(&self) -> &PathBuf {
        &self.cwd
    }

    /// Returns a new request with an isolated worktree workspace.
    ///
    /// The new request will have sandbox set to IsolatedWorktree.
    pub fn with_isolated_workspace(mut self, workspace_path: impl Into<PathBuf>) -> Self {
        self.cwd = workspace_path.into();
        self.sandbox = SandboxMode::IsolatedWorktree;
        self
    }

    /// Returns a new request with read-only workspace access.
    pub fn with_read_only_workspace(mut self) -> Self {
        self.sandbox = SandboxMode::ReadOnly;
        self
    }
}

/// Approval policy for dangerous operations.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalPolicy {
    /// Dangerous default: allow dangerous operations without prompting.
    #[default]
    DangerousDefault,
    /// Safe mode: require explicit approval for dangerous operations.
    Safe,
}

impl ApprovalPolicy {
    /// Returns the mode string for events.
    pub fn as_mode_string(&self) -> &'static str {
        match self {
            ApprovalPolicy::DangerousDefault => crate::events::DANGEROUS_DEFAULT_ACTIVE,
            ApprovalPolicy::Safe => crate::events::SAFE_MODE_ACTIVE,
        }
    }

    /// Returns true if this policy is safe.
    pub fn is_safe(&self) -> bool {
        matches!(self, ApprovalPolicy::Safe)
    }

    /// Returns true if this policy is dangerous (the default).
    pub fn is_dangerous(&self) -> bool {
        matches!(self, ApprovalPolicy::DangerousDefault)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approval_policy_default() {
        assert!(ApprovalPolicy::default().is_dangerous());
        assert!(!ApprovalPolicy::default().is_safe());
    }

    #[test]
    fn test_approval_policy_mode_string() {
        assert_eq!(
            ApprovalPolicy::DangerousDefault.as_mode_string(),
            crate::events::DANGEROUS_DEFAULT_ACTIVE
        );
        assert_eq!(
            ApprovalPolicy::Safe.as_mode_string(),
            crate::events::SAFE_MODE_ACTIVE
        );
    }

    #[test]
    fn test_chat_request_new() {
        let req = ChatRequest::new("hello", "/tmp", ProviderId::Claude);

        assert_eq!(req.prompt, "hello");
        assert_eq!(req.cwd.as_os_str(), "/tmp");
        assert_eq!(req.provider, ProviderId::Claude);
    }

    #[test]
    fn test_chat_request_has_images() {
        let mut req = ChatRequest::new("hello", "/tmp", ProviderId::Claude);
        assert!(!req.has_images());

        req.images.push(PathBuf::from("/tmp/image.png"));
        assert!(req.has_images());
    }

    #[test]
    fn test_workspace_isolated() {
        let mut req = ChatRequest::new("hello", "/tmp", ProviderId::Claude);
        assert!(!req.needs_isolated_workspace());

        req.sandbox = SandboxMode::IsolatedWorktree;
        assert!(req.needs_isolated_workspace());

        req.sandbox = SandboxMode::Container;
        assert!(req.needs_isolated_workspace());
    }

    #[test]
    fn test_workspace_read_only() {
        let mut req = ChatRequest::new("hello", "/tmp", ProviderId::Claude);
        assert!(!req.is_read_only());

        req.sandbox = SandboxMode::ReadOnly;
        assert!(req.is_read_only());
    }

    #[test]
    fn test_workspace_path() {
        let req = ChatRequest::new("hello", "/workspace", ProviderId::Claude);
        assert_eq!(req.workspace().as_os_str(), "/workspace");
    }
}
