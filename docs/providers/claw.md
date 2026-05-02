# Claw Code Provider

## Provider ID

`claw`

## Transport

One-shot text command (via `claw prompt`).

## Command Template

```bash
claw prompt <prompt>
```

## Unsafe Mode (Default)

When `--safe` is NOT specified, `unsafe_args` must be configured in the provider config. If `unsafe_args` is not configured, the adapter returns `UnsafeModeUnavailable`.

The Claw Code ecosystem is fragmented with multiple forks and unstable interfaces, so this provider is treated as configurable via `unsafe_args` and `safe_args`.

## Safe Mode

When `--safe` IS specified, configured `safe_args` are used. If `safe_args` is not configured, the adapter returns `SafeModeUnavailable`.

## Configuration

Claw Code requires explicit configuration because the CLI interface varies across forks:

```toml
[claw]
exe = "claw"
unsafe_args = []  # Must be configured
safe_args = []    # Must be configured
```

## Environment Variables

The following environment variables can be whitelisted:
- Custom environment variables as configured

## Session Support

Session support varies by Claw fork. Not guaranteed.

## Health Check

```bash
which claw && claw doctor  # if available
```

## Notes

The Claw Code ecosystem appears experimental and fork-heavy. Do not hardcode assumptions about this provider's interface.
