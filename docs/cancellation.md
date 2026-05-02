# Cancellation and Timeout

This document describes the cancellation and timeout mechanisms in the agent-adapter system.

## Cancellation Architecture

Cancellation is wired through the system using `CancellationToken`:

```rust
use agent_adapter_transports::CancellationToken;

let token = CancellationToken::new();
// Pass to operations that should be cancellable
token.cancel(); // Signal cancellation
```

### CancellationToken API

- `new()` - Create a new uncancelled token
- `cancel()` - Signal cancellation (idempotent)
- `is_cancelled()` - Check if cancellation was requested
- `wait_cancelled()` - Async wait until cancellation is requested
- `clone()` - Create an independent clone (cancellation state is independent)

### EventSender Trait

For sending cancellation events to event streams:

```rust
use agent_adapter_transports::EventSender;

let helper = EventSendHelper;
helper.send_cancelled(&tx);        // Send RunCancelled
helper.send_timeout(&tx, 5000);    // Send RunFailed with timeout message
```

## Timeout Handling

Timeout is configured via `RunOptions`:

```rust
use agent_adapter_transports::{RunOptions, run_command_with_timeout};

let options = RunOptions::new(run_id, provider)
    .with_timeout(5000); // 5 seconds

let result = run_command_with_timeout(cmd, &options).await;
```

### Timeout Behavior

1. If timeout expires, the child process is killed
2. `Cancelled::TimedOut` is returned with the timeout duration
3. `AdapterError::Timeout` is returned from `run_command_with_timeout`
4. A `RunFailed` event should be emitted with the timeout message

## Process Tree Cleanup

All transports use `kill_on_drop(true)` to ensure process trees are cleaned up:

- When the transport is dropped, the process group is killed
- This handles both normal cleanup and cancellation scenarios
- Works consistently across JSONL, one-shot, ACP, and PTY transports

## Lifecycle Invariants

Cancellation and timeout handling must preserve these invariants:

1. **Exactly one terminal event**: Each run produces exactly one of:
   - `RunCompleted` - successful completion
   - `RunFailed` - failure (including timeout)
   - `RunCancelled` - explicit cancellation

2. **Process cleanup**: On cancellation/timeout, the process tree is fully terminated

3. **No events after terminal**: No events are emitted after a terminal event

## Transport-Level Timeout Handling

Each transport implements timeout at the transport level:

### JSONL Transport

- Uses `tokio::time::timeout` to bound stdout reading
- On timeout, kills child process and emits `RunFailed`

### One-Shot Transport

- Collects stdout/stderr concurrently with `tokio::join!`
- Does not implement internal timeout (handled at higher level)
- Uses `kill_on_drop(true)` for cleanup

### ACP Transport

- Uses async stdin/stdout with timeout on read operations
- On timeout, kills child and emits `RunFailed`

### PTY Transport

- Configurable inactivity timeout via `with_timeout()` (default 5 minutes)
- Uses `try_wait()` to check process exit status
- On timeout expiration, kills child and emits `RunFailed`

## Timeout Error Handling

Timeout errors preserve the run_id for correlation:

```rust
match result.unwrap_err() {
    AdapterError::Timeout { run_id } => {
        // Log timeout with run_id for correlation
        eprintln!("Run {} timed out", run_id);
    }
}
```

## CLI-Level Cancellation

The CLI layer handles user-initiated cancellation:

1. User sends interrupt signal (Ctrl+C)
2. CLI sets cancellation flag via `CancellationToken`
3. All active runs check token and emit `RunCancelled`
4. Process trees are cleaned up

## Testing

Cancellation and timeout are tested via:

- `cancellation::tests` - Unit tests for cancellation primitives
- Transport integration tests - Verify terminal events on timeout/cancellation
- Lifecycle invariant tests - Ensure exactly one terminal event