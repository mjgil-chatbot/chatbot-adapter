# Workspaces

This document describes the workspace model for agent adapters.

## Workspace Types

The adapter supports several workspace modes:

| Kind | Description | Use Case |
|------|-------------|----------|
| `DirectCwd` | Use the provided working directory directly | Default, for local development |
| `TempWorkspace` | Use a temporary directory that is cleaned up after the run | Isolation for untrusted code |
| `IsolatedWorktree` | Use an isolated git worktree | Full isolation with git history |
| `Container` | Run in a container with metadata | Sandboxed execution environment |

## WorkspaceRef

A `WorkspaceRef` identifies a workspace:

```rust
pub enum WorkspaceRef {
    DirectCwd(PathBuf),
    TempWorkspace(PathBuf),
    IsolatedWorktree {
        worktree_path: PathBuf,
        main_repo_path: PathBuf,
    },
    Container {
        container_id: String,
        workspace_path: PathBuf,
    },
}
```

## Workspace Selection

The workspace is determined by:
1. CLI `--workspace` flag
2. `sandbox` field in `ChatRequest`
3. Provider defaults

## Workspace in RunStarted

The `RunStarted` event should include workspace identity:

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

## Cleanup

Workspaces are cleaned up:
- `TempWorkspace`: Automatically on drop
- `IsolatedWorktree`: Git worktree removed on drop
- `Container`: Container lifecycle managed externally

## Path Safety

All workspace paths are validated:
- No path traversal attacks (`../` escapes)
- No symlink traversal
- Paths must be absolute or resolvable

## Testing

Use `TempWorkspace` and `WorktreeWorkspace` helpers from `agent-adapter-testkit`:

```rust
use agent_adapter_testkit::{TempWorkspace, WorktreeWorkspace};

#[test]
fn test_with_temp_workspace() {
    let ws = TempWorkspace::new().unwrap();
    // Use ws.path() for isolated operations
}
```

## Related Documentation

- [events.md](events.md) - Workspace in RunStarted events
- [policy.md](policy.md) - Sandbox modes and approval policies
