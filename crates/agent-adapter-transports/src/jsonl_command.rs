//! JSONL command transport implementation.
//!
//! This transport spawns a command and parses JSONL output from stdout.

mod codex_runtime;
mod normalized_tool_events;

use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use uuid::Uuid;

use agent_adapter_core::{
    AdapterError, AdapterResult, AgentEvent, ChatRequest, ExternalSessionRef, LogLevel, ProviderId,
    Role,
};

use super::TransportStream;
use normalized_tool_events::{
    extract_content_tool_events, extract_content_tool_result_events, extract_text_content,
    extract_tool_name, json_value_to_string, parse_embedded_json, tool_finished_event,
    tool_started_event,
};

/// JSONL line mapper trait.
pub trait JsonlLineMapper: Send {
    fn map_line(&mut self, line: &str) -> Vec<AdapterResult<AgentEvent>>;
    fn finish(self: Box<Self>) -> Vec<AdapterResult<AgentEvent>>;
}

/// JSONL command transport.
pub struct JsonlCommandTransport {
    exe: String,
    args: Vec<String>,
    env: std::collections::HashMap<String, String>,
    prompt_stdin: Option<String>,
}

impl JsonlCommandTransport {
    pub fn new(exe: impl Into<String>, args: Vec<String>) -> Self {
        Self::new_with_env(exe, args, std::collections::HashMap::new())
    }

    pub fn new_with_env(
        exe: impl Into<String>,
        args: Vec<String>,
        env: std::collections::HashMap<String, String>,
    ) -> Self {
        Self {
            exe: exe.into(),
            args,
            env,
            prompt_stdin: None,
        }
    }

    pub fn with_prompt_stdin(mut self, prompt: Option<String>) -> Self {
        self.prompt_stdin = prompt;
        self
    }

    pub fn with_mapper<M: JsonlLineMapper + 'static>(self, _mapper: M) -> Self {
        // For now, we use a generic mapper. In the future, this could be configured.
        self
    }

    fn build_command(&self, request: &ChatRequest) -> Command {
        let mut cmd = Command::new(&self.exe);
        cmd.args(&self.args);

        // Set working directory
        cmd.current_dir(&request.cwd);

        // Inherit the parent environment so provider CLIs can still see
        // runtime config like API endpoints or executable overrides, then
        // apply explicit request/spec overrides last.
        for (key, value) in &self.env {
            cmd.env(key, value);
        }
        for (key, value) in &request.env {
            cmd.env(key, value);
        }

        // Pass context directories (workspace includes for sandbox)
        let context_dirs: Vec<String> = request
            .files
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        if provider_supports_context_dirs(&request.provider) && !context_dirs.is_empty() {
            cmd.arg("--context-dirs").arg(context_dirs.join(","));
        }

        cmd.stdin(if self.prompt_stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        });
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Kill process group on drop
        cmd.kill_on_drop(true);

        cmd
    }

    pub async fn run_stream(&self, request: ChatRequest) -> AdapterResult<TransportStream> {
        let mut cmd = self.build_command(&request);

        let mut child = cmd.spawn().map_err(|e| AdapterError::TransportError {
            message: format!("Failed to spawn {}: {}", self.exe, e),
        })?;
        if let Some(prompt) = self.prompt_stdin.as_ref() {
            let mut stdin = child
                .stdin
                .take()
                .ok_or_else(|| AdapterError::TransportError {
                    message: "Failed to capture stdin".to_string(),
                })?;
            stdin
                .write_all(prompt.as_bytes())
                .await
                .map_err(|e| AdapterError::TransportError {
                    message: format!("Failed to write prompt to stdin: {}", e),
                })?;
            let _ = stdin.write_all(b"\n").await;
        }

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AdapterError::TransportError {
                message: "Failed to capture stdout".to_string(),
            })?;

        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| AdapterError::TransportError {
                message: "Failed to capture stderr".to_string(),
            })?;

        let run_id = Uuid::new_v4();
        let provider = request.provider.clone();

        let (tx, rx) = mpsc::channel(100);

        // Send RunStarted event FIRST to ensure correct ordering
        // All subsequent events will appear after RunStarted in the stream
        let _ = tx
            .send(Ok(AgentEvent::RunStarted {
                run_id,
                provider,
                external_session: None,
                approval_policy: request.approval_policy.as_mode_string().to_string(),
                workspace: agent_adapter_core::WorkspaceRef::from_cwd(
                    request.cwd.clone(),
                    request.sandbox,
                ),
            }))
            .await;

        // Shared state to coordinate final_text between stdout reader and completion task
        use std::sync::Arc;
        use tokio::sync::Mutex;
        let final_text = Arc::new(Mutex::new(String::new()));
        let final_text_clone = final_text.clone();
        let external_session = Arc::new(Mutex::new(None));
        let external_session_clone = external_session.clone();

        // Spawn task to read stdout with generic mapper
        // This task does NOT send terminal events - completion task is the single source of truth
        let tx_stdout = tx.clone();
        let provider_for_stdout = request.provider.clone();
        let stdout_task = tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            let mut text = String::new();
            while let Ok(Some(line)) = reader.next_line().await {
                if let Some(session) = Self::extract_external_session(&line, &provider_for_stdout) {
                    *external_session_clone.lock().await = Some(session);
                }
                let events = Self::map_line(&provider_for_stdout, &line, &mut text);
                for event in events {
                    if tx_stdout.send(event).await.is_err() {
                        break;
                    }
                }
            }
            // Store final text for completion task to use
            *final_text_clone.lock().await = text;
        });

        // Spawn task to read stderr (map to Log events)
        let tx_stderr = tx.clone();
        let stderr_task = tokio::spawn(async move {
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let event = Ok(AgentEvent::Log {
                    level: LogLevel::Warn,
                    message: line,
                });
                let _ = tx_stderr.send(event).await;
            }
        });

        // Spawn task to wait for child and send the appropriate terminal event
        // This is the SINGLE source of truth for terminal events
        let tx_completion = tx.clone();
        let exe_name = self.exe.clone();
        let external_session_for_completion = external_session.clone();
        tokio::spawn(async move {
            let status = child.wait().await;
            let _ = stdout_task.await;
            let _ = stderr_task.await;
            // Wait for final_text to be populated
            let text = final_text.lock().await.clone();
            let external_session = external_session_for_completion.lock().await.clone();
            match status {
                Ok(status) if !status.success() => {
                    let _ = tx_completion
                        .send(Ok(AgentEvent::RunFailed {
                            code: status.code(),
                            message: format!("{} exited with code {:?}", exe_name, status.code()),
                        }))
                        .await;
                }
                Ok(_status) => {
                    // Success - send RunCompleted (status.success() was already checked above)
                    let _ = tx_completion
                        .send(Ok(AgentEvent::RunCompleted {
                            final_text: if text.is_empty() { None } else { Some(text) },
                            external_session,
                        }))
                        .await;
                }
                Err(e) => {
                    let _ = tx_completion
                        .send(Ok(AgentEvent::RunFailed {
                            code: None,
                            message: format!("{} failed: {}", exe_name, e),
                        }))
                        .await;
                }
            }
        });

        let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        Ok(Box::pin(stream) as TransportStream)
    }

    fn map_line(
        provider: &ProviderId,
        line: &str,
        final_text: &mut String,
    ) -> Vec<AdapterResult<AgentEvent>> {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            return vec![Ok(AgentEvent::Log {
                level: LogLevel::Debug,
                message: line.to_string(),
            })];
        };

        Self::map_value(provider, &value, final_text)
    }

    fn map_value(
        provider: &ProviderId,
        value: &serde_json::Value,
        final_text: &mut String,
    ) -> Vec<AdapterResult<AgentEvent>> {
        if matches!(provider, ProviderId::Codex) {
            if let Some(events) = Self::map_codex_runtime_envelope(value, final_text) {
                return events;
            }
        }

        if matches!(value, serde_json::Value::String(_)) {
            return vec![Ok(AgentEvent::Log {
                level: LogLevel::Debug,
                message: value.to_string(),
            })];
        }

        let event_type = value
            .get("type")
            .or_else(|| value.get("event"))
            .or_else(|| value.get("role"))
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

                final_text.push_str(&text);

                vec![Ok(AgentEvent::TextDelta {
                    role: Role::Assistant,
                    text,
                })]
            }
            "assistant" => {
                let content = value
                    .pointer("/message/content")
                    .or_else(|| value.get("content"));
                let text = extract_text_content(content);
                let tool_events = extract_content_tool_events(content);

                let mut events = Vec::new();
                if !text.is_empty() {
                    final_text.push_str(&text);
                    events.push(Ok(AgentEvent::TextDelta {
                        role: Role::Assistant,
                        text,
                    }));
                }
                events.extend(tool_events.into_iter().map(Ok));

                if events.is_empty() {
                    vec![Ok(Self::raw_provider_event(provider, value.clone()))]
                } else {
                    events
                }
            }
            "user" => {
                let content = value
                    .pointer("/message/content")
                    .or_else(|| value.get("content"));
                let events = extract_content_tool_result_events(content);
                if events.is_empty() {
                    vec![Ok(Self::raw_provider_event(provider, value.clone()))]
                } else {
                    events.into_iter().map(Ok).collect()
                }
            }
            "tool" => vec![Ok(tool_finished_event(value)
                .unwrap_or_else(|| Self::raw_provider_event(provider, value.clone())))],
            "item.started" => Self::map_item_started(value),
            "item.completed" => Self::map_item_completed(value, final_text),
            "tool_use" | "tool_call" | "tool_started" => vec![Ok(tool_started_event(value)
                .unwrap_or_else(|| Self::raw_provider_event(provider, value.clone())))],
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
            "tool_result" | "tool_call_finished" | "tool_finished" => {
                vec![Ok(tool_finished_event(value).unwrap_or_else(|| {
                    Self::raw_provider_event(provider, value.clone())
                }))]
            }
            "log" => {
                if let Some(nested) = value
                    .get("message")
                    .and_then(|message| message.as_str())
                    .and_then(|message| serde_json::from_str::<serde_json::Value>(message).ok())
                {
                    return Self::map_value(provider, &nested, final_text);
                }

                vec![Ok(AgentEvent::Log {
                    level: Self::log_level_from_value(value),
                    message: value
                        .get("message")
                        .and_then(|message| message.as_str())
                        .unwrap_or_default()
                        .to_string(),
                })]
            }
            "permission_request" => {
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
            _ => {
                vec![Ok(Self::raw_provider_event(provider, value.clone()))]
            }
        }
    }

    fn raw_provider_event(provider: &ProviderId, payload: serde_json::Value) -> AgentEvent {
        AgentEvent::RawProviderEvent {
            provider: provider.to_string(),
            payload,
        }
    }

    fn log_level_from_value(value: &serde_json::Value) -> LogLevel {
        match value
            .get("level")
            .and_then(|level| level.as_str())
            .unwrap_or("info")
            .to_ascii_lowercase()
            .as_str()
        {
            "trace" => LogLevel::Trace,
            "debug" => LogLevel::Debug,
            "warn" | "warning" => LogLevel::Warn,
            "error" => LogLevel::Error,
            _ => LogLevel::Info,
        }
    }

    fn extract_external_session(line: &str, provider: &ProviderId) -> Option<ExternalSessionRef> {
        let value = serde_json::from_str::<serde_json::Value>(line).ok()?;
        let session_id = Self::extract_external_session_id(&value, provider)?;
        Some(ExternalSessionRef {
            provider: provider.clone(),
            id: session_id,
        })
    }

    fn extract_external_session_id(
        value: &serde_json::Value,
        provider: &ProviderId,
    ) -> Option<String> {
        Self::extract_session_id_value(value)
            .or_else(|| {
                value
                    .get("message")
                    .and_then(|message| message.as_str())
                    .and_then(|message| serde_json::from_str::<serde_json::Value>(message).ok())
                    .and_then(|nested| Self::extract_session_id_value(&nested))
            })
            .or_else(|| {
                (*provider == ProviderId::Kimi).then(|| {
                    value
                        .get("message")
                        .and_then(|message| message.as_str())
                        .and_then(Self::extract_kimi_resume_session_id)
                })?
            })
    }

    fn extract_session_id_value(value: &serde_json::Value) -> Option<String> {
        value
            .get("session_id")
            .or_else(|| value.get("thread_id"))
            .and_then(|id| id.as_str())
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(ToOwned::to_owned)
    }

    fn extract_kimi_resume_session_id(message: &str) -> Option<String> {
        const PREFIX: &str = "To resume this session: kimi -r ";
        message
            .strip_prefix(PREFIX)
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(ToOwned::to_owned)
    }
}

fn provider_supports_context_dirs(provider: &ProviderId) -> bool {
    !matches!(
        provider,
        ProviderId::Claude | ProviderId::MiniMax | ProviderId::Codex | ProviderId::Kimi
    )
}

#[cfg(test)]
mod tests;
