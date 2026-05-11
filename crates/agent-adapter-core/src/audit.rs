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
    }}
impl AuditConfig {
    /// Create a new audit config with audit disabled.
    pub fn disabled() -> Self {
        Self { enabled: false }
    }}
impl AuditConfig {
    /// Returns true if audit is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }}
impl AuditConfig {
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
    }}
impl AuditConfig {
    /// Filters a sequence of events based on audit configuration.
    pub fn filter_events<I: IntoIterator<Item = AgentEvent>>(&self, events: I) -> Vec<AgentEvent> {
        events
            .into_iter()
            .filter(|e| self.filter_event(e))
            .collect()
    }}
