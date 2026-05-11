# JSONL Command Transport

The JSONL (JSON Lines) command transport spawns a child process and parses newline-delimited JSON from stdout. Each line represents an event emitted by the provider.

## Usage

```rust
use agent_adapter_transports::JsonlCommandTransport;

let transport = JsonlCommandTransport::new("provider-binary", vec!["--format".into(), "jsonl".into()]);
let stream = transport.run_stream(request).await?;
```

## Protocol

The transport expects JSON lines where each line contains a JSON object with a `type` field:

```json
{"type": "text_delta", "delta": {"text": "Hello"}}
{"type": "tool_call", "id": "tool1", "name": "read_file", "input": {}}
{"type": "tool_call_finished", "id": "tool1", "output": "file contents..."}
```

### Event Types

| Type | Description |
|------|-------------|
| `text_delta`, `message_delta`, `assistant_delta` | Text content from the model |
| `tool_call`, `tool_started` | A tool call was initiated |
| `tool_call_delta`, `tool_delta` | Streaming input to a tool call |
| `tool_call_finished`, `tool_finished` | A tool call completed |

## Error Handling

- Lines that fail to parse are emitted as `Log` events at `Debug` level
- Stderr lines are emitted as `Log` events at `Warn` level
- Non-zero exit codes produce `RunFailed` events

## Process Management

- Process is killed on drop (`kill_on_drop(true)`)
- stdin is set to null for safety
- stdout/stderr are captured via pipes