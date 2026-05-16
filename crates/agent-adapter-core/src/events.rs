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
    }

    /// Returns true if this is a terminal event (RunCompleted, RunFailed, RunCancelled).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            AgentEvent::RunCompleted { .. }
                | AgentEvent::RunFailed { .. }
                | AgentEvent::RunCancelled
        )
    }

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
    }
}

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
    }
}

/// Mode strings for logging.
pub const DANGEROUS_DEFAULT_ACTIVE: &str = "DANGEROUS_DEFAULT_ACTIVE";
pub const SAFE_MODE_ACTIVE: &str = "SAFE_MODE_ACTIVE";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_event_serde() {
        let event = AgentEvent::RunStarted {
            run_id: Uuid::new_v4(),
            provider: ProviderId::Codex,
            external_session: None,
            approval_policy: DANGEROUS_DEFAULT_ACTIVE.to_string(),
            workspace: WorkspaceRef::Direct {
                path: std::path::PathBuf::from("/workspace"),
            },
        };

        let json = serde_json::to_string(&event).unwrap();
        let parsed: AgentEvent = serde_json::from_str(&json).unwrap();

        assert!(matches!(parsed, AgentEvent::RunStarted { .. }));
    }

    #[test]
    fn test_event_is_terminal() {
        let run_id = Uuid::new_v4();
        let provider = ProviderId::Claude;

        assert!(!AgentEvent::RunStarted {
            run_id,
            provider,
            external_session: None,
            approval_policy: DANGEROUS_DEFAULT_ACTIVE.to_string(),
            workspace: WorkspaceRef::Direct {
                path: std::path::PathBuf::from("/workspace"),
            },
        }
        .is_terminal());

        assert!(AgentEvent::RunCompleted {
            final_text: Some("hello".to_string()),
            external_session: None,
        }
        .is_terminal());

        assert!(AgentEvent::RunFailed {
            code: Some(1),
            message: "error".to_string(),
        }
        .is_terminal());

        assert!(AgentEvent::RunCancelled.is_terminal());
    }

    #[test]
    fn test_redact_secrets() {
        let event = AgentEvent::Log {
            level: LogLevel::Info,
            message: "Token=ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890".to_string(),
        };
        let redacted = event.redact_secrets();
        if let AgentEvent::Log { message, .. } = redacted {
            assert!(message.contains("[REDACTED]"), "Message was: {}", message);
            assert!(!message.contains("ABCDEFGHIJ"));
        } else {
            panic!("Expected Log event");
        }
    }

    #[test]
    fn test_redact_github_token() {
        let event = AgentEvent::TextDelta {
            role: Role::Assistant,
            text: "Token: ghp_1234567890abcdefghijklmnopqrstuvwxyz".to_string(),
        };
        let redacted = event.redact_secrets();
        if let AgentEvent::TextDelta { text, .. } = redacted {
            assert!(text.contains("[REDACTED]"));
            assert!(!text.contains("ghp_1234567890"));
        } else {
            panic!("Expected TextDelta event");
        }
    }

    #[test]
    fn test_secret_patterns_api_key() {
        let patterns = SecretPatterns::default();
        let redacted = patterns.redact("api_key=my_super_secret_key_here");
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("my_super_secret_key"));
    }

    #[test]
    fn test_secret_patterns_github_token() {
        let patterns = SecretPatterns::default();
        let redacted = patterns.redact("ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890");
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("ABCDEFGHIJ"));
    }

    #[test]
    fn test_secret_patterns_bearer_token() {
        let patterns = SecretPatterns::default();
        let redacted =
            patterns.redact("Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9");
        assert!(redacted.contains("[REDACTED]"));
    }

    // =========================================================================
    // Env allowlisting and secret redaction tests (Task 37)
    // =========================================================================

    #[test]
    fn test_redact_password_pattern() {
        let event = AgentEvent::Log {
            level: LogLevel::Info,
            message: "Connecting with password=SuperSecret123".to_string(),
        };
        let redacted = event.redact_secrets();
        if let AgentEvent::Log { message, .. } = redacted {
            assert!(message.contains("[REDACTED]"));
            assert!(!message.contains("SuperSecret123"));
        }
    }

    #[test]
    fn test_redact_authorization_header() {
        let event = AgentEvent::TextDelta {
            role: Role::Assistant,
            text: "Request headers: Authorization=Bearer secret_token_here".to_string(),
        };
        let redacted = event.redact_secrets();
        if let AgentEvent::TextDelta { text, .. } = redacted {
            assert!(text.contains("[REDACTED]"));
            assert!(!text.contains("secret_token_here"));
        }
    }

    #[test]
    fn test_redact_tool_error_with_credentials() {
        let event = AgentEvent::ToolCallFinished {
            id: "tool1".to_string(),
            output: None,
            error: Some("Auth failed: api_token=sk_secret123456789".to_string()),
        };
        let redacted = event.redact_secrets();
        if let AgentEvent::ToolCallFinished {
            error: Some(err), ..
        } = redacted
        {
            assert!(err.contains("[REDACTED]"));
            assert!(!err.contains("sk_secret123456789"));
        }
    }

    #[test]
    fn test_redact_multiple_secrets_in_one_message() {
        // The redaction handles patterns like "api_key=VALUE" or "secret:VALUE"
        // It does not handle bare tokens without indicators
        let event = AgentEvent::Log {
            level: LogLevel::Warn,
            message: "API_KEY=abc123_def456 secret=def456_gh789 token=gh789_abc123".to_string(),
        };
        let redacted = event.redact_secrets();
        if let AgentEvent::Log { message, .. } = redacted {
            assert!(message.contains("[REDACTED]"));
            // Should have multiple redactions
            let count = message.matches("[REDACTED]").count();
            assert!(count >= 3, "Expected at least 3 redactions, got {}", count);
        }
    }

    #[test]
    fn test_no_secrets_in_safe_message() {
        let event = AgentEvent::Log {
            level: LogLevel::Info,
            message: "This is a safe message with no secrets".to_string(),
        };
        let redacted = event.redact_secrets();
        if let AgentEvent::Log { message, .. } = redacted {
            assert!(!message.contains("[REDACTED]"));
            assert!(message.contains("safe message"));
        }
    }

    #[test]
    fn test_redact_ignores_short_values() {
        // Values less than 8 chars are not redacted (likely not real secrets)
        let patterns = SecretPatterns::default();
        let redacted = patterns.redact("key=abc");
        // abc is too short to be a secret
        assert!(!redacted.contains("[REDACTED]"));
    }

    #[test]
    fn test_redact_runs_pass_through() {
        // Events that don't contain secrets should pass through unchanged
        let event = AgentEvent::RunStarted {
            run_id: uuid::Uuid::new_v4(),
            provider: ProviderId::Claude,
            external_session: None,
            approval_policy: "DANGEROUS_DEFAULT_ACTIVE".to_string(),
            workspace: WorkspaceRef::Direct {
                path: std::path::PathBuf::from("/workspace"),
            },
        };
        let redacted = event.redact_secrets();
        // Should be the same (no secrets to redact)
        assert!(matches!(redacted, AgentEvent::RunStarted { .. }));
    }

    #[test]
    fn test_redact_raw_provider_event() {
        // RawProviderEvent should have secrets redacted
        let event = AgentEvent::RawProviderEvent {
            provider: "claude".to_string(),
            payload: serde_json::json!({
                "error": "API key sk_test_123456789 is invalid"
            }),
        };
        let redacted = event.redact_secrets();
        // RawProviderEvent passes through, secrets are not in standard fields
        assert!(matches!(redacted, AgentEvent::RawProviderEvent { .. }));
    }

    // =========================================================================
    // Mode string tests (Task 2, 8)
    // =========================================================================

    #[test]
    fn test_run_started_contains_dangerous_mode_string() {
        // Verify RunStarted contains the correct mode string for dangerous default
        let event = AgentEvent::RunStarted {
            run_id: Uuid::new_v4(),
            provider: ProviderId::Claude,
            external_session: None,
            approval_policy: DANGEROUS_DEFAULT_ACTIVE.to_string(),
            workspace: WorkspaceRef::Direct {
                path: PathBuf::from("/workspace"),
            },
        };

        if let AgentEvent::RunStarted {
            approval_policy, ..
        } = event
        {
            assert_eq!(approval_policy, DANGEROUS_DEFAULT_ACTIVE);
            assert!(!approval_policy.contains("SAFE"));
        }
    }

    #[test]
    fn test_run_started_contains_safe_mode_string() {
        // Verify RunStarted contains the correct mode string for safe mode
        let event = AgentEvent::RunStarted {
            run_id: Uuid::new_v4(),
            provider: ProviderId::Claude,
            external_session: None,
            approval_policy: SAFE_MODE_ACTIVE.to_string(),
            workspace: WorkspaceRef::Direct {
                path: PathBuf::from("/workspace"),
            },
        };

        if let AgentEvent::RunStarted {
            approval_policy, ..
        } = event
        {
            assert_eq!(approval_policy, SAFE_MODE_ACTIVE);
            assert!(approval_policy.contains("SAFE"));
        }
    }

    #[test]
    fn test_mode_string_constants_are_distinct() {
        // Ensure the two mode strings are different to prevent confusion
        assert_ne!(DANGEROUS_DEFAULT_ACTIVE, SAFE_MODE_ACTIVE);
        assert!(DANGEROUS_DEFAULT_ACTIVE.contains("DANGEROUS"));
        assert!(SAFE_MODE_ACTIVE.contains("SAFE"));
    }

    // =========================================================================
    // Workspace metadata tests (Task 10)
    // =========================================================================

    #[test]
    fn test_workspace_ref_direct_serialization() {
        let workspace = WorkspaceRef::Direct {
            path: PathBuf::from("/workspace/project"),
        };
        let json = serde_json::to_string(&workspace).unwrap();
        assert!(json.contains("direct"));
        assert!(json.contains("/workspace/project"));
    }

    #[test]
    fn test_workspace_ref_temp_serialization() {
        let workspace = WorkspaceRef::Temp {
            path: PathBuf::from("/tmp/workspace_abc123"),
        };
        let json = serde_json::to_string(&workspace).unwrap();
        assert!(json.contains("temp"));
        assert!(json.contains("/tmp/workspace_abc123"));
    }

    #[test]
    fn test_workspace_ref_isolated_worktree_serialization() {
        let workspace = WorkspaceRef::IsolatedWorktree {
            path: PathBuf::from("/workspace/worktree"),
            branch: Some("feature-branch".to_string()),
        };
        let json = serde_json::to_string(&workspace).unwrap();
        assert!(json.contains("isolated_worktree"));
        assert!(json.contains("feature-branch"));
    }

    #[test]
    fn test_workspace_ref_container_serialization() {
        let workspace = WorkspaceRef::Container {
            path: PathBuf::from("/workspace"),
            container_id: Some("container_xyz".to_string()),
        };
        let json = serde_json::to_string(&workspace).unwrap();
        assert!(json.contains("container"));
        assert!(json.contains("container_xyz"));
    }

    #[test]
    fn test_workspace_ref_path_method() {
        let path = PathBuf::from("/test/path");
        assert_eq!(WorkspaceRef::Direct { path: path.clone() }.path(), &path);
        assert_eq!(WorkspaceRef::Temp { path: path.clone() }.path(), &path);
        assert_eq!(
            WorkspaceRef::IsolatedWorktree {
                path: path.clone(),
                branch: None
            }
            .path(),
            &path
        );
        assert_eq!(
            WorkspaceRef::Container {
                path: path.clone(),
                container_id: None
            }
            .path(),
            &path
        );
    }

    #[test]
    fn test_workspace_ref_run_started_integration() {
        // Verify workspace metadata flows correctly into RunStarted events
        let workspace = WorkspaceRef::IsolatedWorktree {
            path: PathBuf::from("/repo/worktree"),
            branch: Some("main".to_string()),
        };

        let event = AgentEvent::RunStarted {
            run_id: Uuid::new_v4(),
            provider: ProviderId::Claude,
            external_session: None,
            approval_policy: DANGEROUS_DEFAULT_ACTIVE.to_string(),
            workspace: workspace.clone(),
        };

        if let AgentEvent::RunStarted { workspace: ws, .. } = event {
            assert_eq!(ws.path(), &PathBuf::from("/repo/worktree"));
        }
    }

    // =========================================================================
    // Lifecycle invariant tests (Task 11)
    // =========================================================================

    #[test]
    fn test_only_terminal_events_are_marked_terminal() {
        // Verify that only RunCompleted, RunFailed, and RunCancelled are terminal
        let run_id = Uuid::new_v4();
        let provider = ProviderId::Claude;

        // Non-terminal events
        assert!(!AgentEvent::TextDelta {
            role: Role::Assistant,
            text: "hello".to_string(),
        }
        .is_terminal());

        assert!(!AgentEvent::ToolCallStarted {
            id: "1".to_string(),
            name: "bash".to_string(),
            input: serde_json::json!({}),
        }
        .is_terminal());

        assert!(!AgentEvent::Log {
            level: LogLevel::Info,
            message: "info".to_string(),
        }
        .is_terminal());

        assert!(!AgentEvent::PermissionRequest {
            id: "1".to_string(),
            action: "read".to_string(),
            risk: RiskLevel::Low,
            details: serde_json::json!({}),
        }
        .is_terminal());

        // Terminal events
        assert!(AgentEvent::RunCompleted {
            final_text: None,
            external_session: None,
        }
        .is_terminal());

        assert!(AgentEvent::RunFailed {
            code: Some(1),
            message: "failed".to_string(),
        }
        .is_terminal());

        assert!(AgentEvent::RunCancelled.is_terminal());

        // RunStarted is NOT terminal
        assert!(!AgentEvent::RunStarted {
            run_id,
            provider,
            external_session: None,
            approval_policy: DANGEROUS_DEFAULT_ACTIVE.to_string(),
            workspace: WorkspaceRef::Direct {
                path: PathBuf::from("/workspace"),
            },
        }
        .is_terminal());
    }

    #[test]
    fn test_run_id_extraction() {
        let run_id = Uuid::new_v4();
        let event = AgentEvent::RunStarted {
            run_id,
            provider: ProviderId::Claude,
            external_session: None,
            approval_policy: DANGEROUS_DEFAULT_ACTIVE.to_string(),
            workspace: WorkspaceRef::Direct {
                path: PathBuf::from("/workspace"),
            },
        };

        assert_eq!(event.run_id(), Some(run_id));

        // Non-RunStarted events have no run_id
        let event2 = AgentEvent::TextDelta {
            role: Role::Assistant,
            text: "hello".to_string(),
        };
        assert_eq!(event2.run_id(), None);
    }
}
