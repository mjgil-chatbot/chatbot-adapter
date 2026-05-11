# One-Shot Command Transport

The one-shot command transport spawns a child process and collects its stdout as the final result. It does not support streaming - the full output is returned upon completion.

## Usage

```rust
use agent_adapter_transports::OneShotCommandTransport;

let transport = OneShotCommandTransport::new("provider-binary", vec!["--once".into()]);
let stream = transport.run_stream(request).await?;
```

## Protocol

The transport waits for the process to complete, then:
1. Emits `RunStarted` with the run_id and provider (first event, exactly once)
2. Emits `TextDelta` events for each line of stdout
3. Emits `Log` events for each line of stderr
4. Emits exactly one terminal event: `RunCompleted` with the full stdout as `final_text`, or `RunFailed` on non-zero exit

## Lifecycle Invariants

The transport enforces the following invariants:
- **Exactly one `RunStarted`**: The transport emits `RunStarted` exactly once as the first event
- **Exactly one terminal event**: The transport emits exactly one of `RunCompleted`, `RunFailed`, or `RunCancelled` as the final event
- **No events after terminal**: No non-log events are emitted after a terminal event
- **RunStarted first**: `RunStarted` is always the first event emitted

## Command Construction

The transport builds commands using `CommandSpec`:
- Sets the working directory to `request.cwd`
- Sets environment variables from allowlist only
- Uses `kill_on_drop(true)` for process tree cleanup on cancellation
- Configures stdout/stderr piping for concurrent collection

## Cancellation and Timeout

Cancellation and timeout are handled at a higher level (CLI/router layer). The transport:
- Spawns the process with `kill_on_drop(true)` so process trees are killed on drop
- Does not implement internal timeout - receives `ChatRequest.timeout_ms` but does not act on it
- Relies on the cancellation module to signal stoppage and kill child processes

## Use Cases

- Providers that output complete responses without streaming
- Forge and Hermes providers that return full text in one batch
- Testing scenarios where synchronous output is expected

## Error Handling

- Non-zero exit codes produce `RunFailed` events with the exit code
- Failed spawns produce `TransportError` adapter errors
- Stderr content is always emitted as `Log` events (not errors)