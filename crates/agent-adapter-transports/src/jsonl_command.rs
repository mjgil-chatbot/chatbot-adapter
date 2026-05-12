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
use agent_adapter_core::AdapterResult;
use agent_adapter_core::AgentEvent;
use agent_adapter_core::ChatRequest;
use agent_adapter_core::ExternalSessionRef;
use agent_adapter_core::LogLevel;
use agent_adapter_core::ProviderId;
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
    }}
impl JsonlCommandTransport {
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
    }}
impl JsonlCommandTransport {
    pub fn with_prompt_stdin(mut self, prompt: Option<String>) -> Self {
        self.prompt_stdin = prompt;
        self
    }}
impl JsonlCommandTransport {
    pub fn with_mapper<M: JsonlLineMapper + 'static>(self, _mapper: M) -> Self {
        // For now, we use a generic mapper. In the future, this could be configured.
        self
    }}
impl JsonlCommandTransport {
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
    }}
impl JsonlCommandTransport {
    fn raw_provider_event(provider: &ProviderId, payload: serde_json::Value) -> AgentEvent {
        AgentEvent::RawProviderEvent {
            provider: provider.to_string(),
            payload,
        }
    }}
impl JsonlCommandTransport {
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
    }}
impl JsonlCommandTransport {
    fn extract_external_session(line: &str, provider: &ProviderId) -> Option<ExternalSessionRef> {
        let value = serde_json::from_str::<serde_json::Value>(line).ok()?;
        let session_id = Self::extract_external_session_id(&value, provider)?;
        Some(ExternalSessionRef {
            provider: provider.clone(),
            id: session_id,
        })
    }}
impl JsonlCommandTransport {
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
    }}
impl JsonlCommandTransport {
    fn extract_session_id_value(value: &serde_json::Value) -> Option<String> {
        value
            .get("session_id")
            .or_else(|| value.get("thread_id"))
            .and_then(|id| id.as_str())
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(ToOwned::to_owned)
    }}
impl JsonlCommandTransport {
    fn extract_kimi_resume_session_id(message: &str) -> Option<String> {
        const PREFIX: &str = "To resume this session: kimi -r ";
        message
            .strip_prefix(PREFIX)
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(ToOwned::to_owned)
    }}
fn provider_supports_context_dirs(provider: &ProviderId) -> bool {
    !matches!(
        provider,
        ProviderId::Claude | ProviderId::MiniMax | ProviderId::Codex | ProviderId::Kimi
    )
}
#[cfg(test)]
mod tests;
