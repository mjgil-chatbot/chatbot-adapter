# MiniMax

MiniMax is routed through the Claude-compatible transport in `agent-adapter`.

- Executable: `claude` by default
- Transport: JSONL command
- Mapper: `claude`
- Resume: supported
- Safe mode: `--permission-mode default`
- Dangerous mode: `--dangerously-skip-permissions`
- Stream-json note: Claude CLI requires `--verbose` with `-p` and
  `--output-format stream-json`
