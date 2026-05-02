# Policy: Dangerous Default with Safe Mode Opt-In

## Overview

The agent-adapter uses **dangerous default mode** as the default behavior. This means that when `--safe` is not specified, the adapter will allow dangerous operations (file modifications, shell commands, etc.) to proceed without explicit approval. The `--safe` flag provides an opt-in safe mode that requires explicit approval for dangerous operations.

## Design Principles

1. **Default to dangerous**: Without `--safe`, the adapter operates in dangerous mode by default
2. **Explicit safe opt-in**: `--safe` flag is the only standard way to enable safe mode
3. **No silent fallback**: If unsafe mode was requested implicitly, the adapter must not silently fall back to safe mode
4. **Provider-specific mapping**: Each provider maps these policies to its own flags and configurations
5. **Shared prompt policy**: Cross-provider behavior rules are injected once at the adapter policy layer before provider execution

## Shared Prompt Policy

The adapter injects a shared prompt-policy block into every provider request
before execution. For Claude-compatible providers such as Claude and MiniMax it
is merged into the effective system prompt; for providers without a dedicated
system-prompt surface it is prepended to the user prompt.

Current enforced search rules:

- Prefer the `semble` shell command first for finding code spots and nearby implementations when it is available in the provider runtime.
- Prefer the provider-facing `fff` MCP tools instead of `rg` for exact or exhaustive text matches when the `fff` server is available in the provider runtime.
- Do not assume `fff` is available as a shell command unless the runtime explicitly exposes it that way.
- If either tool is unavailable, fall back immediately to the best available provider-native search or file-inspection tools instead of retrying unavailable tool lookups.
- Do not loop on repeated availability probes for `semble` or `fff`; check once and continue with the available tools.

## CLI Usage

### Dangerous Default (Default)

```bash
# No --safe flag = dangerous default mode
agent-adapter run --provider codex --prompt "Fix the bug"
```

### Safe Mode

```bash
# Explicit --safe flag = safe mode
agent-adapter run --safe --provider codex --prompt "Fix the bug"
```

## Mode Strings

The selected policy is logged in the `RunStarted` event and throughout the run:

- **Dangerous Default**: `DANGEROUS_DEFAULT_ACTIVE`
- **Safe Mode**: `SAFE_MODE_ACTIVE`

### RunStarted Event

```json
{
  "type": "run_started",
  "run_id": "uuid",
  "provider": "codex",
  "external_session": null,
  "approval_policy": "DANGEROUS_DEFAULT_ACTIVE"
}
```

## Provider-Specific Mappings

| Provider   | Dangerous Default (no --safe)                          | Safe Mode (--safe)                                              |
| ---------- | ------------------------------------------------------ | --------------------------------------------------------------- |
| **Codex**  | Add `--dangerously-bypass-approvals-and-sandbox`        | Use `--sandbox read-only --ask-for-approval on-request`         |
| **Claude** | Add `--dangerously-skip-permissions`                    | Use `--permission-mode default` or configured safe mode         |
| **Gemini** | Add `--approval-mode=yolo` (or deprecated `--yolo`)     | Omit yolo flags; use normal approval behavior                  |
| **Kimi**   | Add `--yolo` (note: `--print` implies `--afk`)          | Avoid `--print` if it forces AFK; prefer ACP or return error    |
| **OpenCode** | Use default allow or temp config with `"permission": "allow"` | Create temp config with `"permission": "ask"`                  |
| **Hermes** | Add `--yolo`                                            | Omit `--yolo`; optionally add `--worktree` for isolation        |
| **Forge**  | Use configured `unsafe_args`; error if not available   | Use configured safe args or `forge -p`                         |
| **Claw**   | Use configured `unsafe_args`; error if not available   | Use configured safe args or PTY/custom mode                    |

## Error Handling

### UnsafeModeUnavailable

When `--safe` is NOT specified (dangerous default requested) but unsafe mode is not available for a provider:

```rust
AdapterError::UnsafeModeUnavailable {
    provider: ProviderId::Forge,
    binary_version: None,
    missing_setting: "unsafe_args not configured".to_string(),
    remediation: "Configure unsafe_args in provider config or use --safe".to_string(),
}
```

This error is returned instead of silently falling back to safe mode.

### SafeModeUnavailable

When `--safe` IS specified but safe mode cannot be honored:

```rust
AdapterError::SafeModeUnavailable {
    provider: ProviderId::Kimi,
    reason: "Print mode implies AFK behavior".to_string(),
}
```

## Approval Policy Selection

The `ApprovalPolicy` enum represents the policy selection:

```rust
pub enum ApprovalPolicy {
    DangerousDefault,  // Selected when --safe is absent
    Safe,              // Selected when --safe is present
}
```

### Selection Logic

```rust
let policy = if args.safe {
    ApprovalPolicy::Safe
} else {
    ApprovalPolicy::DangerousDefault
};
```

The CLI argument parsing never inspects raw provider flags directly. Policy is determined purely from the `--safe` flag presence.

## Capability Checks

The router validates that requested capabilities are supported:

```rust
if request.images.len() > 0 && !caps.images {
    return Err(AdapterError::UnsupportedCapability {
        provider: request.provider.clone(),
        capability: "images",
    });
}
