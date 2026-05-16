//! Audit controls for event handling.
//!
//! Audit controls determine whether sensitive event types like raw provider events
//! are retained in the event stream. This is separate from secret redaction
//! (which masks sensitive values in events that ARE shown).
//!
//! Design principle: "Store raw provider events only if audit logging is explicitly enabled."

use crate::events::AgentEvent;

/// Configuration for audit controls.
#[derive(Debug, Clone, Default)]
pub struct AuditConfig {
    /// Whether audit logging is enabled.
    /// When false, raw provider events are filtered out.
    pub enabled: bool,
}

impl AuditConfig {
    /// Create a new audit config with audit enabled.
    pub fn enabled() -> Self {
        Self { enabled: true }
    }

    /// Create a new audit config with audit disabled.
    pub fn disabled() -> Self {
        Self { enabled: false }
    }

    /// Returns true if audit is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Filters an event based on audit configuration.
    ///
    /// When audit is disabled, `RawProviderEvent` variants are filtered out
    /// because they may contain sensitive provider-specific data that should
    /// not be retained unless audit logging is explicitly enabled.
    pub fn filter_event(&self, event: &AgentEvent) -> bool {
        if self.enabled {
            true
        } else {
            !matches!(event, AgentEvent::RawProviderEvent { .. })
        }
    }

    /// Filters a sequence of events based on audit configuration.
    pub fn filter_events<I: IntoIterator<Item = AgentEvent>>(&self, events: I) -> Vec<AgentEvent> {
        events
            .into_iter()
            .filter(|e| self.filter_event(e))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentEvent, ProviderId, Role, WorkspaceRef};

    #[test]
    fn test_audit_config_enabled() {
        let config = AuditConfig::enabled();
        assert!(config.is_enabled());
    }

    #[test]
    fn test_audit_config_disabled() {
        let config = AuditConfig::disabled();
        assert!(!config.is_enabled());
    }

    #[test]
    fn test_filter_event_passes_through_when_enabled() {
        let config = AuditConfig::enabled();
        let event = AgentEvent::RunStarted {
            run_id: uuid::Uuid::new_v4(),
            provider: ProviderId::Claude,
            external_session: None,
            approval_policy: "DANGEROUS_DEFAULT_ACTIVE".to_string(),
            workspace: WorkspaceRef::Direct {
                path: std::path::PathBuf::from("/tmp"),
            },
        };
        assert!(config.filter_event(&event));
    }

    #[test]
    fn test_filter_event_raw_provider_filtered_when_disabled() {
        let config = AuditConfig::disabled();
        let event = AgentEvent::RawProviderEvent {
            provider: "claude".to_string(),
            payload: serde_json::json!({"type": "custom"}),
        };
        // Should be filtered out (return false)
        assert!(!config.filter_event(&event));
    }

    #[test]
    fn test_filter_event_raw_provider_passes_when_enabled() {
        let config = AuditConfig::enabled();
        let event = AgentEvent::RawProviderEvent {
            provider: "claude".to_string(),
            payload: serde_json::json!({"type": "custom"}),
        };
        // Should pass through when audit is enabled
        assert!(config.filter_event(&event));
    }

    #[test]
    fn test_filter_events_multiple() {
        let config = AuditConfig::disabled();
        let events = vec![
            AgentEvent::RunStarted {
                run_id: uuid::Uuid::new_v4(),
                provider: ProviderId::Claude,
                external_session: None,
                approval_policy: "DANGEROUS_DEFAULT_ACTIVE".to_string(),
                workspace: WorkspaceRef::Direct {
                    path: std::path::PathBuf::from("/tmp"),
                },
            },
            AgentEvent::RawProviderEvent {
                provider: "claude".to_string(),
                payload: serde_json::json!({"type": "custom"}),
            },
            AgentEvent::TextDelta {
                role: Role::Assistant,
                text: "hello".to_string(),
            },
        ];
        let filtered = config.filter_events(events);
        assert_eq!(filtered.len(), 2);
        assert!(matches!(filtered[0], AgentEvent::RunStarted { .. }));
        assert!(matches!(filtered[1], AgentEvent::TextDelta { .. }));
    }
}
