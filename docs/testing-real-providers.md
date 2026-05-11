# Real Provider Testing

This guide covers testing against live provider backends (Claude Code, Codex, Gemini, etc.) using the `agent-adapter` test infrastructure.

## Overview

Real provider tests validate that the adapter works correctly with actual provider binaries. These tests are **disabled by default** because they:
- Require installed provider binaries
- May incur API costs
- Require network access
- Cannot run in parallel safely

## Environment Variables

Real provider tests are gated by environment variables. Each provider has a corresponding `RUN_REAL_<PROVIDER>=1` variable.

| Variable | Provider | Example |
|----------|----------|---------|
| `RUN_REAL_CLAUDE` | Claude | `RUN_REAL_CLAUDE=1 cargo test` |
| `RUN_REAL_CODEX` | Codex | `RUN_REAL_CODEX=1 cargo test` |
| `RUN_REAL_GEMINI` | Gemini | `RUN_REAL_GEMINI=1 cargo test` |
| `RUN_REAL_KIMI` | Kimi | `RUN_REAL_KIMI=1 cargo test` |
| `RUN_REAL_OPENCODE` | OpenCode | `RUN_REAL_OPENCODE=1 cargo test` |
| `RUN_REAL_HERMES` | Hermes | `RUN_REAL_HERMES=1 cargo test` |
| `RUN_REAL_FORGE` | Forge | `RUN_REAL_FORGE=1 cargo test` |
| `RUN_REAL_CLAW` | Claw | `RUN_REAL_CLAW=1 cargo test` |

Enable all real provider tests at once:
```bash
RUN_REAL_CLAUDE=1 RUN_REAL_CODEX=1 RUN_REAL_GEMINI=1 cargo test
```

## Real Provider Gates

Real provider tests are gated by individual `RUN_REAL_<PROVIDER>` environment variables. Each provider has a corresponding gate that must be explicitly enabled.

Example:
```bash
RUN_REAL_CLAUDE=1 cargo test real_provider_suite
```

## What Gets Tested

Real provider tests validate:

1. **Binary detection** - Adapter correctly reports `AdapterHealth::MissingBinary` when provider is not installed
2. **Session lifecycle** - Start, stream, and terminate sessions correctly
3. **Event mapping** - Provider-specific events map correctly to `AgentEvent` canonicals
4. **Tool calls** - Tool call events are properly captured and routed
5. **Error handling** - Provider errors surface correctly to callers
6. **File operations** - File change events are captured

## Running Real Provider Tests

### Full suite
```bash
RUN_REAL_CLAUDE=1 cargo test -p agent-adapter-providers --test real_providers
```

### Single provider
```bash
RUN_REAL_CLAUDE=1 cargo test -p agent-adapter-providers claude_
```

### With output
```bash
RUN_REAL_CLAUDE=1 cargo test -p agent-adapter-providers -- --nocapture
```

## CI Integration

Real provider tests are not part of the default repository CI workflow. Keep
them opt-in in local or dedicated secure environments, because they depend on
installed provider binaries, network access, and credentials.

## Security Considerations

- **Never** commit real API keys to the repository
- Use GitHub Secrets for CI credentials
- Real provider tests should use dedicated test/development API keys
- Consider using provider sandbox/dev modes when available

## Troubleshooting

### "Binary not found" errors
Ensure the provider binary is in your PATH, or set `AGENT_ADAPTER_PROVIDER_PATH` to the directory containing binaries.

### "Provider not enabled" errors
Set the appropriate `RUN_REAL_<PROVIDER>` environment variable before running tests.

### Timeout errors
Real provider tests have longer timeouts. If tests still timeout, check network connectivity and provider API status.

### Permission denied
Some providers require specific permissions or licenses. Ensure you have the appropriate entitlements before running tests.
