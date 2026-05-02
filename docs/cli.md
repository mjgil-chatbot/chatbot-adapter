# Agent Adapter CLI

The `agent-adapter-cli` binary provides a unified interface for running AI coding assistants through a consistent transport layer.

## Commands

### run

Run an agent with a prompt.

```bash
agent-adapter run --provider <name> --prompt <text> [options]
```

**Options:**

- `--provider <name>` - Provider to use (codex, claude, gemini, kimi, opencode, hermes, forge, claw)
- `--prompt <text>` - Prompt to send to the agent
- `--safe` - Enable safe mode (requires explicit approval for dangerous operations)
- `--cwd <path>` - Working directory (default: ".")
- `--timeout <ms>` - Timeout in milliseconds
- `--model <name>` - Model to use (provider-specific)
- `--config <path>` - Explicit config file path
- `--no-env` - Skip environment variable overrides
- `--json` - Output events as JSON lines
- `--resume` - Resume a previous session
- `--session-id <id>` - Session ID to resume

### healthcheck

Check provider health status.

```bash
agent-adapter healthcheck --provider <name> [options]
```

**Options:**

- `--provider <name>` - Provider to check
- `--json` - Output as JSON

**Output states:**

- `ready` - Provider binary is available and functional
- `needs_auth` - Provider requires authentication
- `missing` - Provider binary is not installed
- `misconfigured` - Provider configuration is invalid
- `unsupported` - Provider version does not support required capabilities

### providers

List available providers.

```bash
agent-adapter providers [options]
```

**Options:**

- `--json` - Output as JSON

### doctor

Diagnose provider configuration issues.

```bash
agent-adapter doctor [options]
```

**Options:**

- `--json` - Output JSON

## Exit Codes

| Code | Meaning |
|------|---------|
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

## Policy Modes

**Safe mode (`--safe`)** - Operations require explicit approval. Providers run with `--permission-mode default` or equivalent.

**Unsafe mode (default)** - Dangerous operations are permitted without approval. Providers run with `--dangerously-skip-permissions` or equivalent.

## Session Resumption

Sessions can be resumed using `--resume` or `--session-id <id>`:

```bash
agent-adapter run --provider claude --prompt "continue" --resume
agent-adapter run --provider claude --prompt "continue" --session-id abc123
```

## JSON Output

All commands support `--json` for structured output, useful for scripting:

```bash
agent-adapter healthcheck --provider claude --json
agent-adapter providers --json
agent-adapter doctor --json
```
