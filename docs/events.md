# AgentEvent Schema

This document describes the canonical event model for agent adapters. All providers emit events through this unified interface.

## Event Overview

The `AgentEvent` enum is defined in `crates/agent-adapter-core/src/events.rs` and covers:

- Run lifecycle (RunStarted, RunCompleted, RunFailed, RunCancelled)
- Text deltas (TextDelta)
- Tool calls (ToolCallStarted, ToolCallDelta, ToolCallFinished)
- Permission requests (PermissionRequest)
- File changes (FileChange)
- Plans (PlanUpdate)
- Usage statistics (Usage)
- Logging (Log)
- Raw provider events (RawProviderEvent)

## Event Types

### RunStarted

Emitted when a run begins.

```json
{
  "type": "run_started",
  "run_id": "550e8400-e29b-41d4-a716-446655440000",
  "provider": "claude",
  "external_session": null,
  "approval_policy": "DANGEROUS_DEFAULT_ACTIVE"
}
```

Fields:
- `run_id`: UUID identifying this run
- `provider`: Provider identifier (codex, claude, gemini, etc.)
- `external_session`: Optional external session reference for resumption
- `approval_policy`: String representation of the approval policy ("DANGEROUS_DEFAULT_ACTIVE" or "SAFE_MODE_ACTIVE")

### TextDelta

Emitted when text content is received.

```json
{
  "type": "text_delta",
  "role": "assistant",
  "text": "Hello! How can I help you today?"
}
```

Fields:
- `role`: Role of the message sender (user, assistant, system)
- `text`: The text content (may be a partial delta)

### ToolCallStarted

Emitted when a tool call begins.

```json
{
  "type": "tool_call_started",
  "id": "tool_123",
  "name": "read_file",
  "input": {
    "path": "/workspace/src/main.rs"
  }
}
```

Fields:
- `id`: Unique identifier for this tool call
- `name`: Name of the tool being called
- `input`: JSON object with tool-specific input parameters

### ToolCallDelta

Emitted during streaming tool call responses.

```json
{
  "type": "tool_call_delta",
  "id": "tool_123",
  "delta": {
    "partial": "Reading file..."
  }
}
```

Fields:
- `id`: Matching identifier from ToolCallStarted
- `delta`: Partial update to the tool output

### ToolCallFinished

Emitted when a tool call completes.

```json
{
  "type": "tool_call_finished",
  "id": "tool_123",
  "output": "file contents here",
  "error": null
}
```

Fields:
- `id`: Matching identifier from ToolCallStarted
- `output`: Tool output as string, or null if error
- `error`: Error message if tool failed, null otherwise

## Tool Event Contract

`ToolCallStarted` and `ToolCallFinished` are the canonical adapter contract for
tool activity. Every JSONL-backed provider is expected to normalize its native
tool output into those events before the worker/UI layer consumes the stream.

Current non-exempt providers:
- Claude
- MiniMax
- Codex
- Gemini
- Kimi
- OpenCode
- Claw

Current explicit exemptions:
- Forge
- Hermes

Those exempt providers remain outside this contract because their current
adapter path streams passthrough CLI output instead of normalized structured
tool events.

## Startup And Status Rows

Startup markers and provider-specific status notices remain outside the
tool-call schema. They are not tool activity, so folding them into
`ToolCallStarted` / `ToolCallFinished` would blur the contract and make
downstream handling ambiguous.

If those rows need standardization later, they should move onto a separate
shared status schema rather than being added to the tool-call schema.

### FileChange

Emitted when the agent modifies a file.

```json
{
  "type": "file_change",
  "path": "/workspace/src/main.rs",
  "operation": "update"
}
```

Fields:
- `path`: Path to the affected file
- `operation`: Type of operation (create, read, update, delete, rename, copy)

### PermissionRequest

Emitted when the agent requests permission to perform a potentially dangerous action.

```json
{
  "type": "permission_request",
  "id": "perm_456",
  "action": "execute_command",
  "risk": "high",
  "details": {
    "command": "rm -rf /workspace",
    "reason": "User requested cleanup"
  }
}
```

Fields:
- `id`: Unique identifier for this permission request
- `action`: Description of the action being requested
- `risk`: Risk level (none, low, medium, high, critical)
- `details`: Additional context about the request

### PlanUpdate

Emitted when the agent provides a plan or checklist.

```json
{
  "type": "plan_update",
  "items": [
    {"id": "1", "description": "Read project structure", "status": "completed"},
    {"id": "2", "description": "Implement feature", "status": "in_progress"},
    {"id": "3", "description": "Write tests", "status": "pending"}
  ]
}
```

Fields:
- `items`: Array of plan items with id, description, and status

### Usage

Emitted with token usage statistics.

```json
{
  "type": "usage",
  "input_tokens": 1250,
  "output_tokens": 342,
  "cost_usd": 0.023
}
```

Fields:
- `input_tokens`: Number of input tokens consumed
- `output_tokens`: Number of output tokens generated
- `cost_usd`: Estimated cost in USD

### Log

Emitted for log messages from the provider.

```json
{
  "type": "log",
  "level": "info",
  "message": "Processing request..."
}
```

Fields:
- `level`: Log level (trace, debug, info, warn, error)
- `message`: Log message content

### RunCompleted

Emitted when a run completes successfully.

```json
{
  "type": "run_completed",
  "final_text": "Task completed successfully.",
  "external_session": null
}
```

Fields:
- `final_text`: Final text output from the agent
- `external_session`: Session reference if a new session was created

### RunFailed

Emitted when a run fails.

```json
{
  "type": "run_failed",
  "code": 1,
  "message": "Provider returned error: connection timeout"
}
```

Fields:
- `code`: Exit code if available, null otherwise
- `message`: Human-readable error message

### RunCancelled

Emitted when a run is cancelled.

```json
{
  "type": "run_cancelled"
}
```

No additional fields.

### RawProviderEvent

Emitted for provider-specific events that don't fit the standard schema.

```json
{
  "type": "raw_provider_event",
  "provider": "codex",
  "payload": {
    "custom_field": "custom_value"
  }
}
```

Fields:
- `provider`: Name of the provider
- `payload`: Raw JSON payload from the provider

## Lifecycle Invariants

The event stream must follow these invariants:

1. **Exactly one RunStarted** per run
2. **Exactly one terminal event** (RunCompleted, RunFailed, or RunCancelled) per run
3. **No events after terminal event** - any events received after a terminal event indicate a bug

## Lifecycle Ownership

Lifecycle events (RunStarted, RunCompleted, RunFailed, RunCancelled) are **owned by the transport layer**. This design ensures:

- **Single source of truth**: The transport has full context of when the process actually started and ended
- **No duplicate events**: CLI/router layers do not emit lifecycle events, preventing duplicates
- **Consistent ordering**: Transport sends RunStarted before any other events

The transport layer responsibilities:
- `JsonlCommandTransport`: Owns lifecycle for JSONL command execution
- `OneShotCommandTransport`: Owns lifecycle for one-shot command execution
- `AcpStdioTransport`: Owns lifecycle for ACP stdio communication
- `PtyFallbackTransport`: Owns lifecycle for PTY fallback mode

## Workspace Events

Workspace identity should be included in `RunStarted`:

```json
{
  "type": "run_started",
  "run_id": "550e8400-e29b-41d4-a716-446655440000",
  "provider": "claude",
  "external_session": null,
  "approval_policy": "DANGEROUS_DEFAULT_ACTIVE",
  "workspace": {
    "kind": "isolated_worktree",
    "path": "/workspace/session-123"
  }
}
```

Workspace kinds:
- `direct_cwd`: Using the provided working directory directly
- `temp_workspace`: Using a temporary directory
- `isolated_worktree`: Using an isolated git worktree
- `container`: Running in a container with metadata

## Policy Metadata

The `approval_policy` field in `RunStarted` indicates which policy mode is active:

- `DANGEROUS_DEFAULT_ACTIVE`: Default dangerous behavior, allow dangerous operations without prompting
- `SAFE_MODE_ACTIVE`: Safe mode, require explicit approval for dangerous operations

## Testing

Run the event stream invariant tests:

```bash
cargo test --release --package agent-adapter-core -- events::tests
```

## Related Documentation

- [design.md](design.md) - Architecture overview
- [policy.md](policy.md) - Approval policy details
- [transports/](transports/) - Transport-specific event handling
