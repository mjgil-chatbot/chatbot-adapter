# ForgeCode Provider

## Provider ID

`forge`

## Transport

One-shot text command (via `-p` flag).

## Command Template

```bash
forge -p <prompt>
```

## Unsafe Mode (Default)

When `--safe` is NOT specified, `unsafe_args` must be configured in the provider config. If `unsafe_args` is not configured, the adapter returns `UnsafeModeUnavailable`.

```toml
[forge]
unsafe_args = ["--dangerous-flag"]  # if available
```

## Safe Mode

When `--safe` IS specified, either use configured `safe_args` or default to:

```bash
forge -p <prompt>
```

## Environment Variables

The following environment variables can be whitelisted:
- Custom environment variables as configured

## Session Support

Forge does not expose a stable session resumption mechanism.

## Health Check

```bash
which forge && forge --help
```

## Notes

ForgeCode is documented as having a restricted shell mode, but does not expose a well-documented no-approval flag for automation.

GitHub: https://github.com/tailcallhq/forgecode
