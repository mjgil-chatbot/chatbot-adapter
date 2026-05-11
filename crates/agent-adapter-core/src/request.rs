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
