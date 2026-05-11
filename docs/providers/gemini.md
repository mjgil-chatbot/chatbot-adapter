# Gemini CLI Provider

## Provider ID

`gemini`

## Transport

JSONL command (via `--output-format stream-json`).

## Command Template

```bash
gemini -p <prompt> --output-format stream-json
```

## Unsafe Mode (Default)

When `--safe` is NOT specified, the following flags are added:

```bash
--approval-mode=yolo
```

For older versions, the deprecated `--yolo` flag may be used instead.

## Safe Mode

When `--safe` IS specified, no yolo flags are added. Normal approval behavior is used.

## Environment Variables

The following environment variables can be whitelisted:
- `GEMINI_API_KEY` - Required for authentication
- `GOOGLE_API_KEY` - Alternative authentication

## Session Support

Gemini supports session resumption via `--continue` flag.

## Health Check

```bash
which gemini && gemini --version
```

## Notes

Gemini CLI documentation: https://geminicli.com/docs/cli/acp-mode/
