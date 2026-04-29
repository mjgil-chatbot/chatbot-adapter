# Audit Logging

This document describes the audit controls for raw provider event retention.

## Overview

Raw provider events (`AgentEvent::RawProviderEvent`) contain unparsed, provider-specific output that may include sensitive information. Audit controls determine whether these events are retained in the event stream.

**Design principle:** "Store raw provider events only if audit logging is explicitly enabled."

## Audit Configuration

The `AuditConfig` struct in `agent-adapter-core` controls audit behavior:

```rust
pub struct AuditConfig {
    /// Whether audit logging is enabled.
    /// When false, raw provider events are filtered out.
    pub enabled: bool,
}
```

### Creating Audit Config

```rust
// Enable audit logging
let audit = AuditConfig::enabled();

// Disable audit logging (default for production)
let audit = AuditConfig::disabled();
```

## Filtering Events

Use `AuditConfig::filter_event()` to check if an event should be retained:

```rust
let audit = AuditConfig::disabled();
let event = AgentEvent::RawProviderEvent {
    provider: "claude".to_string(),
    payload: serde_json::json!({"type": "custom"}),
};

if audit.filter_event(&event) {
    // Event should be retained
} else {
    // Event should be filtered out
}
```

Filter multiple events with `filter_events()`:

```rust
let audit = AuditConfig::disabled();
let events = vec![...];
let filtered: Vec<AgentEvent> = audit.filter_events(events);
```

## RawProviderEvent

When a provider emits an event type that the mapper doesn't recognize, it is preserved as a `RawProviderEvent`:

```rust
AgentEvent::RawProviderEvent {
    provider: String,      // e.g., "claude", "codex"
    payload: serde_json::Value,  // The raw event data
}
```

### Security Consideration

Raw provider events may contain:
- API keys or tokens in error messages
- Provider-specific debugging information
- Sensitive configuration details

**Disable audit logging in production** unless you specifically need to retain raw provider events for debugging.

## Relationship to Secret Redaction

Audit controls are separate from but complementary to secret redaction:

| Control | Purpose | Default |
|---------|---------|---------|
| Secret redaction (`redact_secrets()`) | Mask sensitive values in events that ARE shown | On |
| Audit controls (`AuditConfig`) | Determine whether to retain raw events at all | Off (disabled) |

Both controls work together to protect sensitive data:
1. Redaction masks secrets in events that pass through
2. Audit controls determine whether raw events are retained at all

## Implementation

- **Module:** `agent-adapter-core/src/audit.rs`
- **Type:** `AuditConfig`
- **Exported:** Via `agent_adapter_core::AuditConfig`

## Testing

Audit controls are tested in `agent-adapter-core/src/audit.rs`:

- `test_audit_config_enabled` - Config enables correctly
- `test_audit_config_disabled` - Config disables correctly
- `test_filter_event_passes_through_when_enabled` - Normal events pass when enabled
- `test_filter_event_raw_provider_filtered_when_disabled` - Raw events filtered when disabled
- `test_filter_event_raw_provider_passes_when_enabled` - Raw events pass when enabled
- `test_filter_events_multiple` - Multiple event filtering works correctly
