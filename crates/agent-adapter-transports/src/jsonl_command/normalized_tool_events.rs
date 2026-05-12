use agent_adapter_core::AgentEvent;
use serde_json::Value;

pub(super) fn extract_text_content(content: Option<&Value>) -> String {
    content
        .and_then(|content| content.as_array())
        .map(|blocks| {
            blocks
                .iter()
                .filter_map(|block| {
                    if block.get("type").and_then(|v| v.as_str()) == Some("text") {
                        block.get("text").and_then(|v| v.as_str())
                    } else {
                        None
                    }
                })
                .collect::<String>()
        })
        .unwrap_or_default()
}

pub(super) fn extract_content_string(content: Option<&Value>) -> String {
    content
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| extract_text_content(content))
}

pub(super) fn extract_content_tool_events(content: Option<&Value>) -> Vec<AgentEvent> {
    content
        .and_then(|content| content.as_array())
        .map(|blocks| blocks.iter().filter_map(tool_started_event).collect())
        .unwrap_or_default()
}

pub(super) fn extract_content_tool_result_events(content: Option<&Value>) -> Vec<AgentEvent> {
    content
        .and_then(|content| content.as_array())
        .map(|blocks| blocks.iter().filter_map(tool_finished_event).collect())
        .unwrap_or_default()
}

pub(super) fn tool_started_event(value: &Value) -> Option<AgentEvent> {
    let block_type = value.get("type").and_then(|kind| kind.as_str())?;
    match block_type {
        "tool_use" => Some(AgentEvent::ToolCallStarted {
            id: value
                .get("id")
                .and_then(|id| id.as_str())
                .unwrap_or("tool")
                .to_string(),
            name: extract_tool_name(value).unwrap_or("tool").to_string(),
            input: value.get("input").cloned().unwrap_or(Value::Null),
        }),
        "tool_call" | "tool_started" => {
            let function = value.get("function").unwrap_or(value);
            let name = extract_tool_name(function).or_else(|| extract_tool_name(value))?;
            let input = function
                .get("arguments")
                .or_else(|| function.get("input"))
                .or_else(|| value.get("input"))
                .or_else(|| value.get("arguments"))
                .cloned()
                .unwrap_or(Value::Null);
            let id = value
                .get("id")
                .and_then(|id| id.as_str())
                .or_else(|| function.get("id").and_then(|id| id.as_str()))
                .unwrap_or(name)
                .to_string();
            Some(AgentEvent::ToolCallStarted {
                id,
                name: name.to_string(),
                input: parse_embedded_json(input),
            })
        }
        _ => None,
    }
}

pub(super) fn tool_finished_event(value: &Value) -> Option<AgentEvent> {
    let event_type = value
        .get("type")
        .or_else(|| value.get("role"))
        .and_then(|kind| kind.as_str())?;
    match event_type {
        "tool_result" => {
            let id = value
                .get("tool_use_id")
                .and_then(|id| id.as_str())
                .or_else(|| value.get("id").and_then(|id| id.as_str()))
                .or_else(|| extract_tool_name(value))
                .unwrap_or("tool")
                .to_string();
            Some(AgentEvent::ToolCallFinished {
                id,
                output: json_value_to_string(
                    value
                        .get("content")
                        .or_else(|| value.get("result"))
                        .or_else(|| value.get("output"))
                        .unwrap_or(&Value::Null),
                ),
                error: None,
            })
        }
        "tool_call_finished" | "tool_finished" => Some(AgentEvent::ToolCallFinished {
            id: value
                .get("id")
                .and_then(|id| id.as_str())
                .unwrap_or("unknown")
                .to_string(),
            output: value
                .get("output")
                .and_then(|output| output.as_str())
                .map(String::from),
            error: value
                .get("error")
                .and_then(|error| error.as_str())
                .map(String::from),
        }),
        "tool" => {
            let output = extract_content_string(
                value
                    .get("content")
                    .or_else(|| value.pointer("/message/content")),
            );
            let id = value
                .get("id")
                .and_then(|id| id.as_str())
                .or_else(|| value.get("name").and_then(|name| name.as_str()))
                .unwrap_or("tool")
                .to_string();
            Some(AgentEvent::ToolCallFinished {
                id,
                output: (!output.is_empty()).then_some(output),
                error: None,
            })
        }
        _ => None,
    }
}

pub(super) fn extract_tool_name(value: &Value) -> Option<&str> {
    value
        .get("name")
        .and_then(|name| name.as_str())
        .or_else(|| value.get("tool_name").and_then(|name| name.as_str()))
        .or_else(|| value.get("toolName").and_then(|name| name.as_str()))
        .or_else(|| {
            value
                .pointer("/function/name")
                .and_then(|name| name.as_str())
        })
        .or_else(|| {
            value
                .pointer("/functionCall/name")
                .and_then(|name| name.as_str())
        })
        .or_else(|| value.pointer("/tool/name").and_then(|name| name.as_str()))
        .or_else(|| value.pointer("/tool").and_then(|name| name.as_str()))
        .or_else(|| value.pointer("/call/name").and_then(|name| name.as_str()))
        .or_else(|| {
            value
                .pointer("/call/function/name")
                .and_then(|name| name.as_str())
        })
        .or_else(|| {
            value
                .pointer("/call/tool_name")
                .and_then(|name| name.as_str())
        })
        .or_else(|| {
            value
                .pointer("/call/toolName")
                .and_then(|name| name.as_str())
        })
        .or_else(|| value.pointer("/call/tool").and_then(|name| name.as_str()))
        .or_else(|| {
            value
                .pointer("/invocation/tool")
                .and_then(|name| name.as_str())
        })
        .or_else(|| {
            value
                .pointer("/call/invocation/tool")
                .and_then(|name| name.as_str())
        })
}

pub(super) fn json_value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        Value::Array(items) => {
            let joined = items
                .iter()
                .filter_map(json_value_to_string)
                .filter(|value| !value.trim().is_empty())
                .collect::<Vec<_>>()
                .join("\n");
            (!joined.is_empty()).then_some(joined)
        }
        Value::Object(map) => {
            if let Some(text) = map.get("text").and_then(|value| value.as_str()) {
                return Some(text.to_string());
            }
            if let Some(content) = map.get("content").and_then(json_value_to_string) {
                return Some(content);
            }
            if let Some(result) = map.get("result").and_then(json_value_to_string) {
                return Some(result);
            }
            if let Some(output) = map.get("output").and_then(json_value_to_string) {
                return Some(output);
            }
            serde_json::to_string(value).ok()
        }
        _ => Some(value.to_string()),
    }
}

pub(super) fn parse_embedded_json(value: Value) -> Value {
    match value {
        Value::String(text) => serde_json::from_str::<Value>(&text).unwrap_or(Value::String(text)),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::extract_tool_name;
    use serde_json::json;

    #[test]
    fn extract_tool_name_reads_nested_call_function_name() {
        let payload = json!({
            "call": {
                "function": {
                    "name": "mcp__bash__run"
                }
            }
        });

        assert_eq!(extract_tool_name(&payload), Some("mcp__bash__run"));
    }

    #[test]
    fn extract_tool_name_reads_invocation_tool_name() {
        let payload = json!({
            "call": {
                "invocation": {
                    "tool": "search_memory"
                }
            }
        });

        assert_eq!(extract_tool_name(&payload), Some("search_memory"));
    }

    #[test]
    fn extract_tool_name_reads_function_call_casing() {
        let payload = json!({
            "functionCall": {
                "name": "mcp__lightweight_builds__run_test"
            }
        });

        assert_eq!(
            extract_tool_name(&payload),
            Some("mcp__lightweight_builds__run_test")
        );
    }

    #[test]
    fn extract_tool_name_reads_call_tool_name_alias() {
        let payload = json!({
            "call": {
                "tool_name": "read_file"
            }
        });

        assert_eq!(extract_tool_name(&payload), Some("read_file"));
    }

    #[test]
    fn extract_tool_name_reads_direct_tool_string() {
        let payload = json!({
            "tool": "search_memory"
        });

        assert_eq!(extract_tool_name(&payload), Some("search_memory"));
    }
}
