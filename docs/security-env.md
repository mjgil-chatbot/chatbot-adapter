# Security: Environment Allowlisting and Redaction

This document describes the security measures for environment variable handling and secret redaction.

## Environment Allowlisting

Providers receive only whitelisted environment variables. This prevents leaking of system-level secrets.

### How It Works

1. Provider configs specify `env_allowlist` - a list of allowed env var names
2. When building commands, only these allowed vars are passed to the child process
3. All other environment variables are cleared with `env_clear()`

### Configuration

```toml
[providers.codex]
exe = "codex"
env_allowlist = ["HOME", "USER", "PATH"]
```

### Security Properties

- **No accidental leakage**: System secrets (AWS keys, etc.) are not passed to providers
- **Explicit allowlist**: Only intentionally passed vars reach the provider
- **Isolation**: Each provider can have different allowlists

## Secret Redaction

All events emitted by adapters have secrets redacted before logging or transmission.

### Redacted Patterns

The following patterns trigger redaction:

| Pattern | Examples |
|---------|----------|
| API keys | `api_key=xxx`, `apikey=xxx` |
| Tokens | `token=xxx`, `bearer xxx` |
| Passwords | `password=xxx`, `pwd=xxx` |
| GitHub tokens | `ghp_xxx`, `gho_xxx`, `ghu_xxx` |
| Auth headers | `Authorization: Bearer xxx` |

### How Redaction Works

```rust
let event = AgentEvent::Log {
    message: "Token=ghp_example_not_a_credential"
};
let redacted = event.redact_secrets();
// Result: "Token=[REDACTED]"
```

### Implementation

The `redact_string()` function in `agent_adapter_core::events` handles redaction:

1. Scans for secret indicator substrings (case-insensitive)
2. Finds the associated value after `:`, `=`, space, quote
3. Replaces values >= 8 chars with `[REDACTED]`
4. Preserves surrounding context

### Usage in Events

- `AgentEvent::Log` - messages are redacted
- `AgentEvent::TextDelta` - text content is redacted
- `AgentEvent::ToolCallFinished` - output and error are redacted
- `AgentEvent::RunFailed` - error messages are redacted

## Raw Provider Events

Raw provider events are passed through without redaction at the transport level. The provider-specific mappers handle any necessary redaction.

## Testing

Secret redaction is tested in `agent_adapter_core::events::tests`:

- `test_redact_secrets` - Basic redaction
- `test_redact_github_token` - GitHub token patterns
- `test_redact_authorization_header` - Bearer token headers
- `test_redact_password_pattern` - Password fields
- `test_redact_multiple_secrets_in_one_message` - Multiple secrets
- `test_redact_ignores_short_values` - Short values not redacted