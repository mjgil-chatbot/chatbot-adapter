# ACP (Agent Client Protocol) Stdio Transport

The ACP stdio transport communicates with agents over stdin/stdout using JSON-RPC 2.0 framing. Used by Gemini, OpenCode, Kimi, and Hermes providers.

## Usage

```rust
use agent_adapter_transports::AcpStdioTransport;

let transport = AcpStdioTransport::new("provider-binary", vec![]);
let stream = transport.run_stream(request).await?;
```

## Protocol

The transport uses JSON-RPC 2.0 with notifications (no `id` field in requests):

### Initialization

```json
{"jsonrpc":"2.0","id":"1","method":"initialize","params":{
  "protocolVersion":"2024-11-05",
  "capabilities":{"tools":true,"permissions":true},
  "clientInfo":{"name":"agent-adapter","version":"0.1.0"}
}}
```

### Prompt Submission

```json
{"jsonrpc":"2.0","id":"2","method":"prompt","params":{
  "prompt":"Your task here..."
}}
```

## Lifecycle Invariants

The transport enforces the following invariants:
- **Exactly one `RunStarted`**: Emitted exactly once as the first event
- **Exactly one terminal event**: One of `RunCompleted`, `RunFailed`, or `RunCancelled` as the final event
- **No events after terminal**: No non-log events emitted after a terminal event
- **RunStarted first**: `RunStarted` is always the first event emitted

## Command Construction

The transport builds commands using `CommandSpec`:
- Sets the working directory to `request.cwd`
- Sets environment variables from allowlist only
- Uses `kill_on_drop(true)` for process tree cleanup on cancellation
- Configures stdin/stdout/stderr piping for JSON-RPC communication

## Cancellation and Timeout

Cancellation and timeout are handled at a higher level (CLI/router layer):
- Spawns the process with `kill_on_drop(true)` so process trees are killed on drop
- Does not implement internal timeout
- Relies on the cancellation module to signal stoppage and kill child processes

## Events (Server → Client)

| Method | Description |
|--------|-------------|
| `text_delta` | Streaming text content |
| `tool_call_started` | Tool execution began |
| `tool_call_delta` | Streaming tool input |
| `tool_call_finished` | Tool execution completed |
| `permission_request` | Approval needed for action |
| `log` | Log message (trace, debug, warn, error, info) |
| `usage` | Token and cost information |

## Approval Policy

The transport embeds `approval_policy` in the `RunStarted` event using `as_mode_string()`:
- `DANGEROUS_DEFAULT_ACTIVE` - dangerous operations allowed by default
- `SAFE_MODE_ACTIVE` - safe mode, approval required

## Process Management

- stdin/stdout/stderr all captured via pipes
- Process is killed on drop (`kill_on_drop(true)`)
- JSON-RPC responses (with `id` field) are ignored (they are protocol acknowledgments)