use super::*;

impl JsonlCommandTransport {
    pub(super) fn map_codex_runtime_envelope(
        value: &serde_json::Value,
        final_text: &mut String,
    ) -> Option<Vec<AdapterResult<AgentEvent>>> {
        let envelope_type = value.get("type").and_then(|kind| kind.as_str())?;
        let payload = value.get("payload")?;

        match envelope_type {
            "response_item" => Self::map_codex_response_item(payload).map(|event| vec![Ok(event)]),
            "event_msg" => {
                Self::map_codex_event_msg(payload, final_text).map(|event| vec![Ok(event)])
            }
            _ => None,
        }
    }

    fn map_codex_response_item(payload: &serde_json::Value) -> Option<AgentEvent> {
        match payload.get("type").and_then(|kind| kind.as_str())? {
            "function_call" => Self::codex_response_item_function_call(payload),
            "custom_tool_call" => {
                let call_id = payload
                    .get("call_id")
                    .and_then(|id| id.as_str())
                    .unwrap_or("tool")
                    .to_string();
                let name = payload
                    .get("name")
                    .and_then(|name| name.as_str())
                    .unwrap_or("tool")
                    .to_string();
                Some(AgentEvent::ToolCallStarted {
                    id: call_id,
                    name,
                    input: parse_embedded_json(
                        payload
                            .get("input")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null),
                    ),
                })
            }
            _ => None,
        }
    }

    fn codex_response_item_function_call(payload: &serde_json::Value) -> Option<AgentEvent> {
        let call_id = payload
            .get("call_id")
            .and_then(|id| id.as_str())
            .unwrap_or("tool")
            .to_string();
        let raw_name = payload.get("name").and_then(|name| name.as_str())?;
        let input = parse_embedded_json(
            payload
                .get("arguments")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        );

        let (name, input) = match raw_name {
            "exec_command" => {
                let command = input.get("cmd").and_then(|cmd| cmd.as_str())?;
                (
                    Self::shell_tool_name(command).to_string(),
                    serde_json::json!({ "command": command }),
                )
            }
            other => (
                payload
                    .get("namespace")
                    .and_then(|namespace| namespace.as_str())
                    .and_then(|namespace| {
                        namespace
                            .strip_prefix("mcp__")
                            .and_then(|server| server.strip_suffix("__"))
                    })
                    .map(|server| format!("mcp__{server}__{other}"))
                    .unwrap_or_else(|| other.to_string()),
                input,
            ),
        };

        Some(AgentEvent::ToolCallStarted {
            id: call_id,
            name,
            input,
        })
    }

    fn map_codex_event_msg(
        payload: &serde_json::Value,
        final_text: &mut String,
    ) -> Option<AgentEvent> {
        match payload.get("type").and_then(|kind| kind.as_str())? {
            "agent_message" => {
                let text = payload
                    .get("message")
                    .and_then(|message| message.as_str())
                    .unwrap_or_default()
                    .to_string();
                if text.is_empty() {
                    None
                } else {
                    final_text.clear();
                    final_text.push_str(&text);
                    Some(AgentEvent::TextDelta {
                        role: Role::Assistant,
                        text,
                    })
                }
            }
            "exec_command_end" => Some(AgentEvent::ToolCallFinished {
                id: payload
                    .get("call_id")
                    .and_then(|id| id.as_str())
                    .unwrap_or("exec_command")
                    .to_string(),
                output: Self::codex_exec_command_output(payload),
                error: Self::codex_exec_command_error(payload),
            }),
            "mcp_tool_call_end" => Some(AgentEvent::ToolCallFinished {
                id: payload
                    .get("call_id")
                    .and_then(|id| id.as_str())
                    .unwrap_or("tool")
                    .to_string(),
                output: Self::codex_mcp_tool_output(payload),
                error: Self::codex_mcp_tool_error(payload),
            }),
            "patch_apply_end" => Some(AgentEvent::ToolCallFinished {
                id: payload
                    .get("call_id")
                    .and_then(|id| id.as_str())
                    .unwrap_or("apply_patch")
                    .to_string(),
                output: payload
                    .get("stdout")
                    .and_then(|output| output.as_str())
                    .map(str::to_string),
                error: payload
                    .get("stderr")
                    .and_then(|error| error.as_str())
                    .filter(|value| !value.trim().is_empty())
                    .map(str::to_string),
            }),
            _ => None,
        }
    }

    pub(super) fn map_item_started(value: &serde_json::Value) -> Vec<AdapterResult<AgentEvent>> {
        let item = match value.get("item") {
            Some(item) => item,
            None => {
                return vec![Ok(AgentEvent::RawProviderEvent {
                    provider: ProviderId::Codex.to_string(),
                    payload: value.clone(),
                })]
            }
        };

        match item.get("type").and_then(|kind| kind.as_str()) {
            Some("command_execution") => {
                let command = Self::codex_item_command(item);
                let Some(command) = command else {
                    return vec![Ok(AgentEvent::RawProviderEvent {
                        provider: ProviderId::Codex.to_string(),
                        payload: value.clone(),
                    })];
                };
                let tool_name = Self::shell_tool_name(&command).to_string();
                vec![Ok(AgentEvent::ToolCallStarted {
                    id: item
                        .get("id")
                        .and_then(|id| id.as_str())
                        .unwrap_or("command_execution")
                        .to_string(),
                    name: tool_name,
                    input: serde_json::json!({ "command": command }),
                })]
            }
            Some("mcp_tool_call") => {
                let name = extract_tool_name(item).unwrap_or("tool");
                let input = item
                    .pointer("/call/arguments")
                    .or_else(|| item.pointer("/call/input"))
                    .or_else(|| item.get("arguments"))
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                vec![Ok(AgentEvent::ToolCallStarted {
                    id: item
                        .get("id")
                        .and_then(|id| id.as_str())
                        .or_else(|| item.pointer("/call/id").and_then(|id| id.as_str()))
                        .unwrap_or(name)
                        .to_string(),
                    name: name.to_string(),
                    input: parse_embedded_json(input),
                })]
            }
            _ => vec![Ok(AgentEvent::RawProviderEvent {
                provider: ProviderId::Codex.to_string(),
                payload: value.clone(),
            })],
        }
    }

    pub(super) fn map_item_completed(
        value: &serde_json::Value,
        final_text: &mut String,
    ) -> Vec<AdapterResult<AgentEvent>> {
        let item = match value.get("item") {
            Some(item) => item,
            None => {
                return vec![Ok(AgentEvent::RawProviderEvent {
                    provider: ProviderId::Codex.to_string(),
                    payload: value.clone(),
                })]
            }
        };

        match item.get("type").and_then(|kind| kind.as_str()) {
            Some("agent_message") => {
                let text = item
                    .get("text")
                    .and_then(|text| text.as_str())
                    .unwrap_or_default()
                    .to_string();
                if text.is_empty() {
                    vec![Ok(AgentEvent::RawProviderEvent {
                        provider: ProviderId::Codex.to_string(),
                        payload: value.clone(),
                    })]
                } else {
                    final_text.clear();
                    final_text.push_str(&text);
                    vec![Ok(AgentEvent::TextDelta {
                        role: Role::Assistant,
                        text,
                    })]
                }
            }
            Some("command_execution") => {
                vec![Ok(AgentEvent::ToolCallFinished {
                    id: item
                        .get("id")
                        .and_then(|id| id.as_str())
                        .unwrap_or("command_execution")
                        .to_string(),
                    output: Self::codex_item_output(item),
                    error: Self::codex_item_error(item),
                })]
            }
            Some("mcp_tool_call") => {
                let name = extract_tool_name(item).unwrap_or("tool");
                vec![Ok(AgentEvent::ToolCallFinished {
                    id: item
                        .get("id")
                        .and_then(|id| id.as_str())
                        .or_else(|| item.pointer("/call/id").and_then(|id| id.as_str()))
                        .unwrap_or(name)
                        .to_string(),
                    output: Self::codex_item_output(item),
                    error: Self::codex_item_error(item),
                })]
            }
            _ => vec![Ok(AgentEvent::RawProviderEvent {
                provider: ProviderId::Codex.to_string(),
                payload: value.clone(),
            })],
        }
    }

    fn codex_item_command(item: &serde_json::Value) -> Option<String> {
        if let Some(command) = item.get("command").and_then(|command| command.as_str()) {
            return Some(command.to_string());
        }

        let arguments = Self::codex_item_arguments(item)?;
        if let Some(command_line) = arguments
            .get("command_line")
            .and_then(|command| command.as_str())
        {
            return Some(command_line.to_string());
        }

        let command = arguments
            .get("command")
            .and_then(|command| command.as_str())?;
        let args = arguments
            .get("args")
            .and_then(|args| args.as_array())
            .map(|args| {
                args.iter()
                    .filter_map(|arg| arg.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_default();
        if args.is_empty() {
            Some(command.to_string())
        } else {
            Some(format!("{command} {args}"))
        }
    }

    fn codex_item_arguments(item: &serde_json::Value) -> Option<serde_json::Value> {
        if let Some(value) = [
            item.get("input"),
            item.get("arguments"),
            item.get("args"),
            item.get("params"),
            item.pointer("/call/arguments"),
            item.pointer("/call/args"),
            item.pointer("/call/input"),
            item.pointer("/function/arguments"),
            item.pointer("/functionCall/args"),
        ]
        .into_iter()
        .flatten()
        .next()
        {
            return Some(parse_embedded_json(value.clone()));
        }
        None
    }

    fn codex_item_output(item: &serde_json::Value) -> Option<String> {
        for candidate in [
            item.get("aggregated_output"),
            item.get("output"),
            item.pointer("/call/output"),
            item.pointer("/call/result"),
            item.pointer("/result/output"),
            item.pointer("/result/content"),
            item.get("result"),
            item.get("content"),
        ] {
            if let Some(output) = candidate.and_then(json_value_to_string) {
                if !output.trim().is_empty() {
                    return Some(output);
                }
            }
        }
        None
    }

    fn codex_item_error(item: &serde_json::Value) -> Option<String> {
        for candidate in [
            item.get("error"),
            item.get("message"),
            item.pointer("/result/error"),
            item.pointer("/call/error"),
        ] {
            if let Some(error) = candidate.and_then(json_value_to_string) {
                if !error.trim().is_empty() {
                    return Some(error);
                }
            }
        }

        let exit_code = item.get("exit_code").and_then(|code| code.as_i64())?;
        (exit_code != 0).then(|| format!("command exited with code {exit_code}"))
    }

    fn codex_exec_command_output(payload: &serde_json::Value) -> Option<String> {
        for candidate in [
            payload.get("aggregated_output"),
            payload.get("formatted_output"),
            payload.get("stdout"),
            payload.get("stderr"),
        ] {
            if let Some(output) = candidate.and_then(|value| value.as_str()) {
                if !output.trim().is_empty() {
                    return Some(output.to_string());
                }
            }
        }
        None
    }

    fn codex_exec_command_error(payload: &serde_json::Value) -> Option<String> {
        let exit_code = payload.get("exit_code").and_then(|code| code.as_i64())?;
        (exit_code != 0).then(|| format!("command exited with code {exit_code}"))
    }

    fn codex_mcp_tool_output(payload: &serde_json::Value) -> Option<String> {
        if let Some(output) = payload
            .pointer("/result/Ok/content")
            .and_then(json_value_to_string)
            .filter(|value| !value.trim().is_empty())
        {
            return Some(output);
        }

        payload
            .pointer("/result/Err")
            .and_then(|error| error.as_str())
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string)
    }

    fn codex_mcp_tool_error(payload: &serde_json::Value) -> Option<String> {
        payload
            .pointer("/result/Err")
            .and_then(|error| error.as_str())
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string)
    }

    fn shell_tool_name(command: &str) -> &'static str {
        match command
            .split_whitespace()
            .next()
            .map(|token| token.trim_matches('\'').trim_matches('"'))
        {
            Some("rg") | Some("grep") => "grep",
            _ => "bash",
        }
    }
}
