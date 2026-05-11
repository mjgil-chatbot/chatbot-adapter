# Kimi CLI Provider

## Provider ID

`kimi`

## Transport

JSONL command (via `--output-format stream-json --print`).

## Command Template

```bash
kimi --print --output-format stream-json --prompt <prompt>
```

## Unsafe Mode (Default)

When `--safe` is NOT specified, the `--yolo` flag is added:

```bash
kimi --print --output-format stream-json --yolo --prompt <prompt>
```

**Important**: The `--print` mode implicitly enables `--afk` behavior, which automatically executes file modifications and shell commands.

## Safe Mode

When `--safe` IS specified, the adapter returns `SafeModeUnavailable` because `--print` mode implies `--afk` behavior which is inherently dangerous. The Kimi `--print` mode does not support safe operation.

**Note**: The Kimi CLI does have an ACP protocol mode (`kimi acp`) which supports safer operation, but this adapter implementation uses `--print` mode only.

## Environment Variables

The following environment variables can be whitelisted:
- `KIMI_API_KEY` - Required for authentication

## Session Support

Kimi supports session resumption via `--continue` flag.

## Health Check

```bash
which kimi && kimi --version
```

## Notes

The adapter accepts sandbox/workspace hints from callers, but it does not
forward them as `--context-dirs` to `kimi` because the Kimi CLI rejects that
flag.

Kimi CLI documentation: https://moonshotai.github.io/kimi-cli/en/reference/kimi-command.html
