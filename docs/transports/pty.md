# PTY Fallback Transport

The PTY (pseudo-terminal) fallback transport is a **compatibility escape hatch** used when no structured transport (JSONL, ACP) is available.

## When to Use

Only use this transport when:
- The provider cannot run headlessly via JSONL or ACP
- No better option exists for the specific provider

## Limitations

- PTY output is difficult to parse reliably
- ANSI escape sequences must be stripped from output
- Completion detection is heuristic (inactivity timeout or explicit markers)
- Lower reliability compared to structured modes
- Process tree cleanup is not guaranteed on all platforms

## Usage

```rust
use agent_adapter_transports::PtyFallbackTransport;

let transport = PtyFallbackTransport::new("provider-binary", vec![])
    .with_timeout(300000); // 5 minute timeout
let stream = transport.run_stream(request).await?;
```

## Lifecycle Invariants

The transport enforces the following invariants:
- **Exactly one `RunStarted`**: Emitted exactly once as the first event
- **Exactly one terminal event**: One of `RunCompleted`, `RunFailed`, or `RunCancelled` as the final event
- **No events after terminal**: No non-log events emitted after a terminal event

## Implementation

The transport:
1. Opens a pseudo-terminal pair
2. Spawns the provider process in the PTY slave
3. Reads output from the PTY master
4. Strips ANSI escape sequences from output
5. Detects completion via process exit or timeout

## Cancellation and Timeout

Cancellation and timeout are handled at a higher level (CLI/router layer):
- The transport uses `kill_on_drop` semantics for process cleanup
- Timeout is configurable via `with_timeout()` method (default 5 minutes)
- Relies on the cancellation module to signal stoppage

## Approval Policy

The transport embeds `approval_policy` in the `RunStarted` event using `as_mode_string()`:
- `DANGEROUS_DEFAULT_ACTIVE` - dangerous operations allowed by default
- `SAFE_MODE_ACTIVE` - safe mode, approval required

## Warning

This transport is not recommended for production use with providers that support structured transports. It exists solely for providers that cannot work any other way.