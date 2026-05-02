# Codex JSONL Tool Normalization

## Scope

`crates/agent-adapter-transports` owns Codex JSONL normalization.

## Implemented Contract

`JsonlCommandTransport` now supports both Codex tool-event shapes:

- legacy `item.started` / `item.completed` records for `command_execution` and
  `mcp_tool_call`
- raw runtime envelopes emitted as top-level `response_item` / `event_msg`
  records

For the raw runtime path, the transport normalizes:

- `response_item` `function_call` into `AgentEvent::ToolCallStarted`
- `event_msg` `exec_command_end` into `AgentEvent::ToolCallFinished`
- `event_msg` `mcp_tool_call_end` into `AgentEvent::ToolCallFinished`
- `event_msg` `patch_apply_end` into `AgentEvent::ToolCallFinished`

`exec_command` starts are rewritten onto the existing shell-tool path by
deriving the tool name from the command (`bash` vs `grep`) and normalizing the
input shape to `{ "command": ... }`. MCP starts preserve the tool namespace by
emitting names like `mcp__chatbot_memory__search_memory`, which lets downstream
workers keep the actual tool identity instead of collapsing to a generic
`tool` label in the persisted transcript.
The fallback name extraction now also checks deeper nested call fields such as
`call.function.name`, `call.tool_name`, and `call.invocation.tool` before it
gives up. That narrows the remaining cases that reach downstream workers as a
bare generic `tool` event.
