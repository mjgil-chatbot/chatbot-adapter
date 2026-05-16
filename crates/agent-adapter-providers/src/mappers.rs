//! Event mappers for provider-specific output formats.

mod plain_text;

use agent_adapter_core::{AdapterResult, AgentEvent, FileOperation, LogLevel, Role};
use std::path::PathBuf;

pub use plain_text::PlainTextMapper;

/// Claude-specific event mapper.
pub struct ClaudeMapper {
    final_text: String,
}

impl ClaudeMapper {
    pub fn new() -> Self {
        Self {
            final_text: String::new(),
        }
    }

    pub fn map_line(&mut self, line: &str) -> Vec<AdapterResult<AgentEvent>> {
        let value = match serde_json::from_str::<serde_json::Value>(line) {
            Ok(v) => v,
            Err(_) => return vec![self.log(line)],
        };

        let event_type = value
            .get("type")
            .or_else(|| value.get("event"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        match event_type {
            "text_delta" | "message_delta" => {
                let text = value
                    .pointer("/delta/text")
                    .or_else(|| value.get("text"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();

                self.final_text.push_str(&text);

                vec![Ok(AgentEvent::TextDelta {
                    role: Role::Assistant,
                    text,
                })]
            }
            "tool_call" | "tool_started" => {
                vec![Ok(AgentEvent::ToolCallStarted {
                    id: value
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    name: value
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("tool")
                        .to_string(),
                    input: value
                        .get("input")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null),
                })]
            }
            "tool_call_delta" | "tool_delta" => {
                vec![Ok(AgentEvent::ToolCallDelta {
                    id: value
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    delta: value
                        .get("delta")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null),
                })]
            }
            "tool_call_finished" | "tool_finished" => {
                vec![Ok(AgentEvent::ToolCallFinished {
                    id: value
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    output: value
                        .get("output")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    error: value
                        .get("error")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                })]
            }
            "usage" => {
                vec![Ok(AgentEvent::Usage {
                    input_tokens: value.get("input_tokens").and_then(|v| v.as_u64()),
                    output_tokens: value.get("output_tokens").and_then(|v| v.as_u64()),
                    cost_usd: value.get("cost_usd").and_then(|v| v.as_f64()),
                })]
            }
            "permission_request" => {
                // Map provider permission requests to AgentEvent::PermissionRequest
                let risk_str = value.get("risk").and_then(|v| v.as_str()).unwrap_or("none");
                let risk = match risk_str {
                    "low" => agent_adapter_core::RiskLevel::Low,
                    "medium" => agent_adapter_core::RiskLevel::Medium,
                    "high" => agent_adapter_core::RiskLevel::High,
                    "critical" => agent_adapter_core::RiskLevel::Critical,
                    _ => agent_adapter_core::RiskLevel::None,
                };
                vec![Ok(AgentEvent::PermissionRequest {
                    id: value
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    action: value
                        .get("action")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    risk,
                    details: value
                        .get("details")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null),
                })]
            }
            "file_change" => {
                // Map file change notifications to AgentEvent::FileChange
                let path = value
                    .get("path")
                    .and_then(|v| v.as_str())
                    .map(PathBuf::from)
                    .unwrap_or_default();
                let operation_str = value
                    .get("operation")
                    .and_then(|v| v.as_str())
                    .unwrap_or("update");
                let operation = match operation_str {
                    "create" => FileOperation::Create,
                    "read" => FileOperation::Read,
                    "update" | "edit" => FileOperation::Update,
                    "delete" | "remove" => FileOperation::Delete,
                    "rename" => FileOperation::Rename,
                    "copy" => FileOperation::Copy,
                    _ => FileOperation::Update,
                };
                vec![Ok(AgentEvent::FileChange { path, operation })]
            }
            "content_block_start" | "content_block_delta" | "content_block_stop" => {
                // Claude-specific message structure events - ignore these
                vec![]
            }
            _ => {
                vec![Ok(AgentEvent::RawProviderEvent {
                    provider: "claude".to_string(),
                    payload: value,
                })]
            }
        }
    }

    pub fn map_stderr(&mut self, line: &str) -> Vec<AdapterResult<AgentEvent>> {
        vec![Ok(AgentEvent::Log {
            level: LogLevel::Warn,
            message: line.to_string(),
        })]
    }

    pub fn finish(self) -> Vec<AdapterResult<AgentEvent>> {
        if self.final_text.is_empty() {
            vec![]
        } else {
            vec![Ok(AgentEvent::RunCompleted {
                final_text: Some(self.final_text),
                external_session: None,
            })]
        }
    }

    fn log(&self, message: &str) -> AdapterResult<AgentEvent> {
        Ok(AgentEvent::Log {
            level: LogLevel::Debug,
            message: message.to_string(),
        })
    }
}

impl Default for ClaudeMapper {
    fn default() -> Self {
        Self::new()
    }
}

/// Generic JSONL mapper for providers following standard JSONL output.
pub struct JsonlMapper {
    final_text: String,
    provider: String,
}

impl JsonlMapper {
    pub fn new(provider: impl Into<String>) -> Self {
        Self {
            final_text: String::new(),
            provider: provider.into(),
        }
    }

    pub fn map_line(&mut self, line: &str) -> Vec<AdapterResult<AgentEvent>> {
        let value = match serde_json::from_str::<serde_json::Value>(line) {
            Ok(v) => v,
            Err(_) => return vec![self.log(line)],
        };

        let event_type = value
            .get("type")
            .or_else(|| value.get("event"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        match event_type {
            "text_delta" | "message_delta" | "assistant_delta" => {
                let text = value
                    .pointer("/delta/text")
                    .or_else(|| value.get("text"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();

                self.final_text.push_str(&text);

                vec![Ok(AgentEvent::TextDelta {
                    role: Role::Assistant,
                    text,
                })]
            }
            "tool_call" | "tool_started" => {
                vec![Ok(AgentEvent::ToolCallStarted {
                    id: value
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    name: value
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("tool")
                        .to_string(),
                    input: value
                        .get("input")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null),
                })]
            }
            "tool_call_delta" | "tool_delta" => {
                vec![Ok(AgentEvent::ToolCallDelta {
                    id: value
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    delta: value
                        .get("delta")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null),
                })]
            }
            "tool_call_finished" | "tool_finished" => {
                vec![Ok(AgentEvent::ToolCallFinished {
                    id: value
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    output: value
                        .get("output")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    error: value
                        .get("error")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                })]
            }
            "file_change" => {
                // Map file change notifications for generic JSONL providers
                let path = value
                    .get("path")
                    .and_then(|v| v.as_str())
                    .map(PathBuf::from)
                    .unwrap_or_default();
                let operation_str = value
                    .get("operation")
                    .and_then(|v| v.as_str())
                    .unwrap_or("update");
                let operation = match operation_str {
                    "create" => FileOperation::Create,
                    "read" => FileOperation::Read,
                    "update" | "edit" => FileOperation::Update,
                    "delete" | "remove" => FileOperation::Delete,
                    "rename" => FileOperation::Rename,
                    "copy" => FileOperation::Copy,
                    _ => FileOperation::Update,
                };
                vec![Ok(AgentEvent::FileChange { path, operation })]
            }
            _ => {
                vec![Ok(AgentEvent::RawProviderEvent {
                    provider: self.provider.clone(),
                    payload: value,
                })]
            }
        }
    }

    pub fn finish(self) -> Vec<AdapterResult<AgentEvent>> {
        if self.final_text.is_empty() {
            vec![]
        } else {
            vec![Ok(AgentEvent::RunCompleted {
                final_text: Some(self.final_text),
                external_session: None,
            })]
        }
    }

    fn log(&self, message: &str) -> AdapterResult<AgentEvent> {
        Ok(AgentEvent::Log {
            level: LogLevel::Debug,
            message: message.to_string(),
        })
    }
}

/// Codex event mapper for OpenAI Codex JSONL output.
pub struct CodexMapper {
    final_text: String,
}

impl CodexMapper {
    pub fn new() -> Self {
        Self {
            final_text: String::new(),
        }
    }

    pub fn map_line(&mut self, line: &str) -> Vec<AdapterResult<AgentEvent>> {
        let value = match serde_json::from_str::<serde_json::Value>(line) {
            Ok(v) => v,
            Err(_) => return vec![self.log(line)],
        };

        let event_type = value
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        match event_type {
            "text_delta" | "message_delta" => {
                let text = value
                    .pointer("/delta/text")
                    .or_else(|| value.get("text"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();

                self.final_text.push_str(&text);

                vec![Ok(AgentEvent::TextDelta {
                    role: Role::Assistant,
                    text,
                })]
            }
            "tool_call" => {
                vec![Ok(AgentEvent::ToolCallStarted {
                    id: value
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    name: value
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("tool")
                        .to_string(),
                    input: value
                        .get("input")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null),
                })]
            }
            "tool_call_finished" => {
                vec![Ok(AgentEvent::ToolCallFinished {
                    id: value
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    output: value
                        .get("output")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    error: value
                        .get("error")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                })]
            }
            "file_change" => {
                let path = value
                    .get("path")
                    .and_then(|v| v.as_str())
                    .map(PathBuf::from)
                    .unwrap_or_default();
                let operation_str = value
                    .get("operation")
                    .and_then(|v| v.as_str())
                    .unwrap_or("update");
                let operation = match operation_str {
                    "create" => FileOperation::Create,
                    "read" => FileOperation::Read,
                    "update" | "edit" => FileOperation::Update,
                    "delete" | "remove" => FileOperation::Delete,
                    "rename" => FileOperation::Rename,
                    "copy" => FileOperation::Copy,
                    _ => FileOperation::Update,
                };
                vec![Ok(AgentEvent::FileChange { path, operation })]
            }
            _ => {
                vec![Ok(AgentEvent::RawProviderEvent {
                    provider: "codex".to_string(),
                    payload: value,
                })]
            }
        }
    }

    pub fn map_stderr(&mut self, line: &str) -> Vec<AdapterResult<AgentEvent>> {
        vec![Ok(AgentEvent::Log {
            level: LogLevel::Warn,
            message: line.to_string(),
        })]
    }

    pub fn finish(self) -> Vec<AdapterResult<AgentEvent>> {
        if self.final_text.is_empty() {
            vec![]
        } else {
            vec![Ok(AgentEvent::RunCompleted {
                final_text: Some(self.final_text),
                external_session: None,
            })]
        }
    }

    fn log(&self, message: &str) -> AdapterResult<AgentEvent> {
        Ok(AgentEvent::Log {
            level: LogLevel::Debug,
            message: message.to_string(),
        })
    }
}

impl Default for CodexMapper {
    fn default() -> Self {
        Self::new()
    }
}

/// Hermes event mapper for Hermes ACP JSONL output.
pub struct HermesMapper {
    final_text: String,
}

impl HermesMapper {
    pub fn new() -> Self {
        Self {
            final_text: String::new(),
        }
    }

    pub fn map_line(&mut self, line: &str) -> Vec<AdapterResult<AgentEvent>> {
        let value = match serde_json::from_str::<serde_json::Value>(line) {
            Ok(v) => v,
            Err(_) => return vec![self.log(line)],
        };

        let event_type = value
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        match event_type {
            "content" => {
                let text = value
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();

                self.final_text.push_str(&text);

                vec![Ok(AgentEvent::TextDelta {
                    role: Role::Assistant,
                    text,
                })]
            }
            "text" => {
                // Some providers use {"type": "text", "text": "..."}
                let text = value
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();

                self.final_text.push_str(&text);

                vec![Ok(AgentEvent::TextDelta {
                    role: Role::Assistant,
                    text,
                })]
            }
            "tool" => {
                vec![Ok(AgentEvent::ToolCallStarted {
                    id: value
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    name: value
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("tool")
                        .to_string(),
                    input: value
                        .get("input")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null),
                })]
            }
            "result" => {
                vec![Ok(AgentEvent::ToolCallFinished {
                    id: value
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    output: value
                        .get("output")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    error: None,
                })]
            }
            "done" => {
                vec![Ok(AgentEvent::RunCompleted {
                    final_text: Some(self.final_text.clone()),
                    external_session: None,
                })]
            }
            "file_change" => {
                let path = value
                    .get("path")
                    .and_then(|v| v.as_str())
                    .map(PathBuf::from)
                    .unwrap_or_default();
                let operation_str = value
                    .get("operation")
                    .and_then(|v| v.as_str())
                    .unwrap_or("update");
                let operation = match operation_str {
                    "create" => FileOperation::Create,
                    "read" => FileOperation::Read,
                    "update" | "edit" => FileOperation::Update,
                    "delete" | "remove" => FileOperation::Delete,
                    "rename" => FileOperation::Rename,
                    "copy" => FileOperation::Copy,
                    _ => FileOperation::Update,
                };
                vec![Ok(AgentEvent::FileChange { path, operation })]
            }
            _ => {
                vec![Ok(AgentEvent::RawProviderEvent {
                    provider: "hermes".to_string(),
                    payload: value,
                })]
            }
        }
    }

    pub fn map_stderr(&mut self, line: &str) -> Vec<AdapterResult<AgentEvent>> {
        vec![Ok(AgentEvent::Log {
            level: LogLevel::Warn,
            message: line.to_string(),
        })]
    }

    pub fn finish(self) -> Vec<AdapterResult<AgentEvent>> {
        if self.final_text.is_empty() {
            vec![]
        } else {
            vec![Ok(AgentEvent::RunCompleted {
                final_text: Some(self.final_text),
                external_session: None,
            })]
        }
    }

    fn log(&self, message: &str) -> AdapterResult<AgentEvent> {
        Ok(AgentEvent::Log {
            level: LogLevel::Debug,
            message: message.to_string(),
        })
    }
}

impl Default for HermesMapper {
    fn default() -> Self {
        Self::new()
    }
}

/// Claw event mapper for Claw custom JSONL output.
pub struct ClawMapper {
    final_text: String,
}

impl ClawMapper {
    pub fn new() -> Self {
        Self {
            final_text: String::new(),
        }
    }

    pub fn map_line(&mut self, line: &str) -> Vec<AdapterResult<AgentEvent>> {
        let value = match serde_json::from_str::<serde_json::Value>(line) {
            Ok(v) => v,
            Err(_) => return vec![self.log(line)],
        };

        let event_type = value
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        match event_type {
            "text_delta" => {
                let text = value
                    .pointer("/delta/text")
                    .or_else(|| value.get("text"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();

                self.final_text.push_str(&text);

                vec![Ok(AgentEvent::TextDelta {
                    role: Role::Assistant,
                    text,
                })]
            }
            "tool" => {
                vec![Ok(AgentEvent::ToolCallStarted {
                    id: value
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    name: value
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("tool")
                        .to_string(),
                    input: value
                        .get("input")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null),
                })]
            }
            "tool_result" => {
                vec![Ok(AgentEvent::ToolCallFinished {
                    id: value
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    output: value
                        .get("result")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    error: None,
                })]
            }
            "finish" => {
                vec![Ok(AgentEvent::RunCompleted {
                    final_text: Some(self.final_text.clone()),
                    external_session: None,
                })]
            }
            "file_change" => {
                let path = value
                    .get("path")
                    .and_then(|v| v.as_str())
                    .map(PathBuf::from)
                    .unwrap_or_default();
                let operation_str = value
                    .get("operation")
                    .and_then(|v| v.as_str())
                    .unwrap_or("update");
                let operation = match operation_str {
                    "create" => FileOperation::Create,
                    "read" => FileOperation::Read,
                    "update" | "edit" => FileOperation::Update,
                    "delete" | "remove" => FileOperation::Delete,
                    "rename" => FileOperation::Rename,
                    "copy" => FileOperation::Copy,
                    _ => FileOperation::Update,
                };
                vec![Ok(AgentEvent::FileChange { path, operation })]
            }
            _ => {
                vec![Ok(AgentEvent::RawProviderEvent {
                    provider: "claw".to_string(),
                    payload: value,
                })]
            }
        }
    }

    pub fn map_stderr(&mut self, line: &str) -> Vec<AdapterResult<AgentEvent>> {
        vec![Ok(AgentEvent::Log {
            level: LogLevel::Warn,
            message: line.to_string(),
        })]
    }

    pub fn finish(self) -> Vec<AdapterResult<AgentEvent>> {
        if self.final_text.is_empty() {
            vec![]
        } else {
            vec![Ok(AgentEvent::RunCompleted {
                final_text: Some(self.final_text),
                external_session: None,
            })]
        }
    }

    fn log(&self, message: &str) -> AdapterResult<AgentEvent> {
        Ok(AgentEvent::Log {
            level: LogLevel::Debug,
            message: message.to_string(),
        })
    }
}

impl Default for ClawMapper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
