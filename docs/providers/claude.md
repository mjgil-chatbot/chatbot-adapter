# Claude Code Provider

## Provider ID

`claude`

## Transport

JSONL command (via `-p --verbose --output-format stream-json`).

## Command Template

```bash
claude -p <prompt> --verbose --output-format stream-json --input-format text
```

## Unsafe Mode (Default)

When `--safe` is NOT specified, the following flag is added:

```bash
--dangerously-skip-permissions
```

## Safe Mode

When `--safe` IS specified, the following flags are used:

```bash
--permission-mode default
```

## Environment Variables

The following environment variables can be whitelisted:
- `ANTHROPIC_API_KEY` - Required for authentication
- `CLAUDE_SESSION_ID` - For session resumption

## Session Support

Claude supports session resumption via `--session-id <SESSION_ID>` or `--resume`.

## Health Check

```bash
which claude && claude --version
```

## Notes

Claude CLI currently requires `--verbose` when `--print`/`-p` is paired with
`--output-format stream-json`.

Claude CLI documentation: https://code.claude.com/docs/en/cli-reference
