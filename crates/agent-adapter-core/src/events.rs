//! Canonical event model for agent adapters.
//!
//! All providers emit events through this unified interface.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;
use crate::types::{
    ExternalSessionRef, FileOperation, LogLevel, PlanItem, ProviderId, RiskLevel, Role,
    WorkspaceRef,
};
/// Agent events emitted by adapters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    /// A run has started.
    RunStarted {
        run_id: Uuid,
        provider: ProviderId,
        external_session: Option<ExternalSessionRef>,
        approval_policy: String,
        workspace: WorkspaceRef,
    },

    /// Text delta from the agent.
    TextDelta { role: Role, text: String },

    /// Tool call started.
    ToolCallStarted {
        id: String,
        name: String,
        input: serde_json::Value,
    },

    /// Tool call delta (streaming).
    ToolCallDelta {
        id: String,
        delta: serde_json::Value,
    },

    /// Tool call finished.
    ToolCallFinished {
        id: String,
        output: Option<String>,
        error: Option<String>,
    },

    /// File was changed.
    FileChange {
        path: PathBuf,
        operation: FileOperation,
    },

    /// Permission requested by the agent.
    PermissionRequest {
        id: String,
        action: String,
        risk: RiskLevel,
        details: serde_json::Value,
    },

    /// Plan update from the agent.
    PlanUpdate { items: Vec<PlanItem> },

    /// Usage statistics.
    Usage {
        input_tokens: Option<u64>,
        output_tokens: Option<u64>,
        cost_usd: Option<f64>,
    },

    /// Log message.
    Log { level: LogLevel, message: String },

    /// Run completed successfully.
    RunCompleted {
        final_text: Option<String>,
        external_session: Option<ExternalSessionRef>,
    },

    /// Run failed.
    RunFailed { code: Option<i32>, message: String },

    /// Run was cancelled.
    RunCancelled,

    /// Raw provider-specific event preserved for passthrough.
    RawProviderEvent {
        provider: String,
        payload: serde_json::Value,
    },
}
/// Final text event emitted when a run completes with full text in one shot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunCompleted {
    pub final_text: Option<String>,
    pub external_session: Option<ExternalSessionRef>,
}
/// Run failed event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunFailed {
    pub code: Option<i32>,
    pub message: String,
}
impl AgentEvent {
    /// Returns the run ID if this event is associated with a run.
    pub fn run_id(&self) -> Option<Uuid> {
        match self {
            AgentEvent::RunStarted { run_id, .. } => Some(*run_id),
            _ => None,
        }
    }}
impl AgentEvent {
    /// Returns true if this is a terminal event (RunCompleted, RunFailed, RunCancelled).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            AgentEvent::RunCompleted { .. }
                | AgentEvent::RunFailed { .. }
                | AgentEvent::RunCancelled
        )
    }}
impl AgentEvent {
    /// Returns a redacted copy of this event with secrets masked.
    ///
    /// Secrets matching common patterns (API keys, tokens, passwords in env vars)
    /// are replaced with `[REDACTED]`.
    pub fn redact_secrets(&self) -> Self {
        match self {
            AgentEvent::Log { level, message } => AgentEvent::Log {
                level: *level,
                message: redact_string(message),
            },
            AgentEvent::TextDelta { role, text } => AgentEvent::TextDelta {
                role: *role,
                text: redact_string(text),
            },
            AgentEvent::ToolCallFinished { id, output, error } => AgentEvent::ToolCallFinished {
                id: id.clone(),
                output: output.as_ref().map(|s| redact_string(s.as_str())),
                error: error.as_ref().map(|s| redact_string(s.as_str())),
            },
            AgentEvent::RunFailed { code, message } => AgentEvent::RunFailed {
                code: *code,
                message: redact_string(message),
            },
            // Events that don't typically contain secrets pass through unchanged
            other => other.clone(),
        }
    }}
/// Redact common secret patterns from a string.
///
/// Uses simple string matching rather than regex for efficiency.
fn redact_string(s: &str) -> String {
    let mut result = s.to_string();

    // List of common secret indicator substrings (lowercased for case-insensitive matching)
    let secret_indicators = [
        "api_key",
        "apikey",
        "api_key",
        "api-token",
        "api_secret",
        "bearer ",
        "ghp_",
        "gho_",
        "ghu_",
        "ghs_",
        "ghr_",
        "secret",
        "password",
        "passwd",
        "pwd",
        "token",
        "auth",
    ];

    // For each potential secret pattern, check if the indicator appears
    // and redact the following content
    for indicator in &secret_indicators {
        let ind_lower = indicator.to_lowercase();
        let mut search_start = 0;

        while let Some(pos) = result.to_lowercase()[search_start..].find(&ind_lower) {
            let actual_pos = search_start + pos;
            // Find the start of the value (skip past the indicator and common separators)
            let mut value_start = actual_pos + indicator.len();
            while value_start < result.len() {
                let c = result.chars().nth(value_start).unwrap();
                if c == ':' || c == '=' || c == ' ' || c == '"' || c == '\'' {
                    value_start += 1;
                } else {
                    break;
                }
            }

            // Find the end of the value (until whitespace or quote)
            if value_start < result.len() {
                let mut value_end = value_start;
                let chars: Vec<char> = result.chars().collect();
                while value_end < chars.len() {
                    let c = chars[value_end];
                    if c.is_whitespace() || c == '"' || c == '\'' || c == ',' || c == '}' {
                        break;
                    }
                    value_end += 1;
                }

                // Only redact if the value is reasonably long (likely an actual secret)
                if value_end - value_start >= 8 {
                    let prefix = &result[..value_start];
                    let suffix = &result[value_end..];
                    result = format!("{}[REDACTED]{}", prefix, suffix);
                    search_start = value_start + "[REDACTED]".len();
                } else {
                    search_start = value_end;
                }
            } else {
                break;
            }
        }
    }

    result
}
/// Patterns used for secret detection.
///
/// Uses simple string matching rather than regex for efficiency.
#[allow(dead_code)]
pub struct SecretPatterns;
impl Default for SecretPatterns {
    fn default() -> Self {
        Self
    }
}
impl SecretPatterns {
    /// Redact any secrets found in the input string.
    #[allow(dead_code)]
    pub fn redact(&self, s: &str) -> String {
        redact_string(s)
    }}
/// Mode strings for logging.
pub const DANGEROUS_DEFAULT_ACTIVE: &str = "DANGEROUS_DEFAULT_ACTIVE";
pub const SAFE_MODE_ACTIVE: &str = "SAFE_MODE_ACTIVE";
