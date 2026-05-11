# Hermes Agent Provider

## Provider ID

`hermes`

## Transport

One-shot text command (via `-z` flag for final text output).

**Note**: The Hermes CLI also supports ACP mode (`hermes acp`) for richer session control, but this adapter implementation uses one-shot mode only.

## Command Template (One-shot)

```bash
hermes -z <prompt>
```

## Unsafe Mode (Default)

When `--safe` is NOT specified, the `--yolo` flag is added:

```bash
hermes -z <prompt> --yolo
```

## Safe Mode

When `--safe` IS specified, the `--yolo` flag is omitted. Optionally, `--worktree` can be added for workspace isolation.

## Environment Variables

The following environment variables can be whitelisted:
- Custom environment variables as configured

## Session Support

Hermes supports session resumption via `--resume` and `--continue` flags.

## Health Check

```bash
which hermes && hermes version
```

## Notes

Hermes documentation: https://hermes-agent.nousresearch.com/docs/reference/cli-commands
