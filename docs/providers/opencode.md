# OpenCode Provider

## Provider ID

`opencode`

## Transport

JSONL command (via `--format json`).

## Command Template

```bash
opencode run --format json --dir <cwd> <prompt>
```

## Unsafe Mode (Default)

When `--safe` is NOT specified, OpenCode uses its default allow behavior. This can be configured via a temporary config file:

```json
{"permission": "allow"}
```

## Safe Mode

When `--safe` IS specified, a temporary config with stricter permissions is used:

```json
{"permission": "ask"}
```

## Environment Variables

The following environment variables can be whitelisted:
- Custom environment variables as configured

## Session Support

OpenCode supports session resumption via `--session <ID>` and `--continue` flags.

## Health Check

```bash
which opencode && opencode --version
```

## Notes

OpenCode permissions documentation: https://open-code.ai/en/docs/permissions
