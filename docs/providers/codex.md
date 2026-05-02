# Codex Provider

## Provider ID

`codex`

## Transport

JSONL command (via `--json` flag).

## Command Template

```bash
codex exec --json --cd <cwd> -
```

## Unsafe Mode (Default)

When `--safe` is NOT specified, the following flags are added:

```bash
--dangerously-bypass-approvals-and-sandbox
```

Or alternatively:
```bash
--yolo
```

## Safe Mode

When `--safe` IS specified, the following flags are used:

```bash
--sandbox read-only
--ask-for-approval on-request
```

## Environment Variables

The following environment variables can be whitelisted:
- `OPENAI_API_KEY` - Required for authentication
- `OPENAI_ORGANIZATION` - Optional organization ID

## Session Support

Codex supports session resumption via `codex exec resume <SESSION_ID>`.

## Health Check

```bash
which codex && codex --version
```

## Notes

The adapter accepts sandbox/workspace hints from callers, but it does not
forward them as `--context-dirs` to `codex exec` because the Codex CLI rejects
that flag.

The adapter also prepends its shared prompt-policy block before execution, so
Codex runs inherit the enforced code-search guidance to use `semble` for code
spot discovery and `fff` instead of `rg` for text matches.

Codex CLI documentation: https://developers.openai.com/codex/cli/reference
