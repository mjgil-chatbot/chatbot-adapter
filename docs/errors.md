# Error and Exit Code Model

This document describes the structured error types and exit codes used by the agent adapter.

## AdapterError Enum

All adapter operations return `AdapterResult<T>` which is `Result<T, AdapterError>`. The `AdapterError` enum is defined in `crates/agent-adapter-core/src/errors.rs`.

### Error Variants

| Variant | Description | Remediation |
|---------|-------------|-------------|
| `ProviderNotFound` | Provider binary not found | Install the provider binary |
| `ProviderNotFunctional` | Provider binary exists but not working | Check provider installation |
| `NeedsAuth` | Provider requires authentication | Complete provider auth setup |
| `Misconfigured` | Provider is misconfigured | Fix provider configuration |
| `UnsafeModeUnavailable` | Unsafe mode not available | Configure unsafe mode settings |
| `SafeModeUnavailable` | Safe mode not available | Use unsafe mode or different provider |
| `UnsupportedCapability` | Provider doesn't support capability | Use a different provider |
| `TransportError` | Transport layer error | Check system resources |
| `ParseError` | JSON parsing failed | Check provider output format |
| `Timeout` | Operation timed out | Increase timeout or check provider |
| `Cancelled` | Operation was cancelled | - |
| `SessionNotFound` | Session doesn't exist | Check session ID |
| `InvalidConfig` | Configuration is invalid | Fix configuration |

## Error Display

Each error variant has a human-readable display format:

```
ProviderNotFound: Provider 'claude' not found. Install from https://.../
ProviderNotFunctional: Provider 'codex' not functional: connection refused
NeedsAuth: Provider 'claude' needs auth: Run `claude auth login`
Misconfigured: Provider 'forge' misconfigured: missing unsafe_args
UnsafeModeUnavailable: Unsafe mode unavailable for 'forge': unsafe_args not configured (Set unsafe_args in config)
SafeModeUnavailable: Safe mode unavailable for 'kimi': --print mode doesn't support safe mode
UnsupportedCapability: Provider 'claude' does not support capability: image_input
TransportError: Transport error: Broken pipe
ParseError: Parse error: Unexpected token (raw: "invalid json{")
Timeout: Operation timed out: run-123
Cancelled: Operation cancelled: run-456
SessionNotFound: Session 'abc123' not found for provider 'claude'
InvalidConfig: Invalid configuration: missing provider exe
```

## Exit Codes

The CLI returns exit codes based on the error type:

| Exit Code | Meaning |
|-----------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Provider not found |
| 3 | Authentication required |
| 4 | Configuration error |
| 5 | Unsafe mode unavailable |
| 6 | Safe mode unavailable |
| 7 | Capability not supported |
| 8 | Transport error |
| 9 | Parse error |
| 10 | Timeout |
| 11 | Cancelled |
| 12 | Session not found |

### Exit Code Mapping

```rust
impl AdapterError {
    pub fn exit_code(&self) -> i32 {
        match self {
            AdapterError::ProviderNotFound { .. } => 2,
            AdapterError::NeedsAuth { .. } => 3,
            AdapterError::Misconfigured { .. } => 4,
            AdapterError::UnsafeModeUnavailable { .. } => 5,
            AdapterError::SafeModeUnavailable { .. } => 6,
            AdapterError::UnsupportedCapability { .. } => 7,
            AdapterError::TransportError { .. } => 8,
            AdapterError::ParseError { .. } => 9,
            AdapterError::Timeout { .. } => 10,
            AdapterError::Cancelled { .. } => 11,
            AdapterError::SessionNotFound { .. } => 12,
            AdapterError::ProviderNotFunctional { .. } => 1,
            AdapterError::InvalidConfig { .. } => 4,
        }
    }
}
```

## Error Serialization

Errors serialize to JSON with the variant name and fields:

```json
{
  "ProviderNotFound": {
    "provider": "claude",
    "install_hint": "Install from https://claude.ai/code"
  }
}
```

## Testing

Run error tests:

```bash
cargo test --release --package agent-adapter-core -- errors
```

## Related Documentation

- [events.md](events.md) - RunFailed and RunCancelled events
- [policy.md](policy.md) - Safe/unsafe mode details
- [cli.md](cli.md) - CLI exit codes