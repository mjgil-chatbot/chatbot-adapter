//! Fake implementations for testing.

use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use uuid::Uuid;

use agent_adapter_core::{
    AdapterCapabilities, AdapterHealth, AdapterResult, AgentAdapter, AgentEvent, ChatRequest,
    EventStream, LogLevel, ProviderId, Role,
};

/// Fake adapter for testing that can simulate various provider behaviors.
pub struct FakeAdapter {
    provider: ProviderId,
    events: Vec<AgentEvent>,
    capabilities: AdapterCapabilities,
    health: AdapterHealth,
    should_fail: bool,
    fail_error: Option<String>,
    delay_ms: Option<u64>,
}

impl FakeAdapter {
    pub fn new(provider: ProviderId) -> Self {
        Self {
            provider,
            events: Vec::new(),
            capabilities: AdapterCapabilities::default(),
            health: AdapterHealth::Ready {
                version: Some("0.0.0-test".to_string()),
            },
            should_fail: false,
            fail_error: None,
            delay_ms: None,
        }
    }

    pub fn with_events(mut self, events: Vec<AgentEvent>) -> Self {
        self.events = events;
        self
    }

    pub fn with_capabilities(mut self, caps: AdapterCapabilities) -> Self {
        self.capabilities = caps;
        self
    }

    pub fn with_health(mut self, health: AdapterHealth) -> Self {
        self.health = health;
        self
    }

    pub fn with_failure(mut self, error: impl Into<String>) -> Self {
        self.should_fail = true;
        self.fail_error = Some(error.into());
        self
    }

    pub fn with_delay(mut self, ms: u64) -> Self {
        self.delay_ms = Some(ms);
        self
    }

    pub fn with_jsonl_output(mut self, output: &str) -> Self {
        self.events = parse_jsonl_lines(output);
        self
    }

    pub fn simulate_text_response(mut self, text: &str) -> Self {
        self.events = vec![
            AgentEvent::TextDelta {
                role: Role::Assistant,
                text: text.to_string(),
            },
            AgentEvent::RunCompleted {
                final_text: Some(text.to_string()),
                external_session: None,
            },
        ];
        self
    }

    pub fn simulate_tool_call(mut self, id: &str, name: &str) -> Self {
        self.events = vec![
            AgentEvent::ToolCallStarted {
                id: id.to_string(),
                name: name.to_string(),
                input: serde_json::json!({}),
            },
            AgentEvent::ToolCallFinished {
                id: id.to_string(),
                output: Some("{}".to_string()),
                error: None,
            },
            AgentEvent::RunCompleted {
                final_text: None,
                external_session: None,
            },
        ];
        self
    }
}

impl Default for FakeAdapter {
    fn default() -> Self {
        Self::new(ProviderId::Claude)
    }
}

#[async_trait]
impl AgentAdapter for FakeAdapter {
    fn id(&self) -> ProviderId {
        self.provider.clone()
    }

    fn capabilities(&self) -> AdapterCapabilities {
        self.capabilities.clone()
    }

    async fn healthcheck(&self) -> AdapterResult<AdapterHealth> {
        if let Some(ms) = self.delay_ms {
            tokio::time::sleep(tokio::time::Duration::from_millis(ms)).await;
        }
        Ok(self.health.clone())
    }

    async fn run(&self, _request: ChatRequest) -> AdapterResult<EventStream> {
        if self.should_fail {
            return Err(agent_adapter_core::AdapterError::TransportError {
                message: self
                    .fail_error
                    .clone()
                    .unwrap_or_else(|| "Fake error".to_string()),
            });
        }

        if let Some(ms) = self.delay_ms {
            tokio::time::sleep(tokio::time::Duration::from_millis(ms)).await;
        }

        let events: Vec<_> = self.events.clone();
        let stream = futures_util::stream::iter(events.into_iter().map(Ok));
        Ok(Box::pin(stream))
    }

    async fn cancel(&self, _run_id: Uuid) -> AdapterResult<()> {
        Ok(())
    }
}

/// Parse JSONL lines into AgentEvents for fake adapters.
pub fn parse_jsonl_lines(output: &str) -> Vec<AgentEvent> {
    let mut events = Vec::new();
    let mut final_text = String::new();

    for line in output.lines() {
        if line.is_empty() {
            continue;
        }

        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            events.push(AgentEvent::Log {
                level: LogLevel::Debug,
                message: line.to_string(),
            });
            continue;
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

                final_text.push_str(&text);
                events.push(AgentEvent::TextDelta {
                    role: Role::Assistant,
                    text,
                });
            }
            "tool_call" | "tool_started" => {
                events.push(AgentEvent::ToolCallStarted {
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
                });
            }
            "tool_call_delta" | "tool_delta" => {
                events.push(AgentEvent::ToolCallDelta {
                    id: value
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    delta: value
                        .get("delta")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null),
                });
            }
            "tool_call_finished" | "tool_finished" => {
                events.push(AgentEvent::ToolCallFinished {
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
                });
            }
            "usage" => {
                events.push(AgentEvent::Usage {
                    input_tokens: value.get("input_tokens").and_then(|v| v.as_u64()),
                    output_tokens: value.get("output_tokens").and_then(|v| v.as_u64()),
                    cost_usd: value.get("cost_usd").and_then(|v| v.as_f64()),
                });
            }
            _ => {
                events.push(AgentEvent::RawProviderEvent {
                    provider: "fake".to_string(),
                    payload: value,
                });
            }
        }
    }

    if !final_text.is_empty()
        && !events
            .iter()
            .any(|e| matches!(e, AgentEvent::RunCompleted { .. }))
    {
        events.push(AgentEvent::RunCompleted {
            final_text: Some(final_text),
            external_session: None,
        });
    }

    events
}

/// A cancellation token for fake testing.
#[derive(Clone, Default)]
pub struct FakeCancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl FakeCancellationToken {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

/// Builder for creating fake events in tests.
pub struct EventBuilder {
    events: Vec<AgentEvent>,
}

impl EventBuilder {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn with_run_started(mut self, run_id: Uuid, provider: ProviderId) -> Self {
        self.events.push(AgentEvent::RunStarted {
            run_id,
            provider,
            external_session: None,
            approval_policy: "DangerousDefault".to_string(),
            workspace: agent_adapter_core::WorkspaceRef::Direct {
                path: std::path::PathBuf::from("/fake"),
            },
        });
        self
    }

    pub fn with_text_delta(mut self, text: &str) -> Self {
        self.events.push(AgentEvent::TextDelta {
            role: Role::Assistant,
            text: text.to_string(),
        });
        self
    }

    pub fn with_tool_call(mut self, id: &str, name: &str) -> Self {
        self.events.push(AgentEvent::ToolCallStarted {
            id: id.to_string(),
            name: name.to_string(),
            input: serde_json::json!({}),
        });
        self
    }

    pub fn with_tool_finished(mut self, id: &str, output: Option<&str>) -> Self {
        self.events.push(AgentEvent::ToolCallFinished {
            id: id.to_string(),
            output: output.map(String::from),
            error: None,
        });
        self
    }

    pub fn with_run_completed(mut self, final_text: Option<&str>) -> Self {
        self.events.push(AgentEvent::RunCompleted {
            final_text: final_text.map(String::from),
            external_session: None,
        });
        self
    }

    pub fn with_run_failed(mut self, message: &str) -> Self {
        self.events.push(AgentEvent::RunFailed {
            code: None,
            message: message.to_string(),
        });
        self
    }

    pub fn with_permission_request(mut self, id: &str, action: &str) -> Self {
        self.events.push(AgentEvent::PermissionRequest {
            id: id.to_string(),
            action: action.to_string(),
            risk: agent_adapter_core::RiskLevel::High,
            details: serde_json::json!({}),
        });
        self
    }

    pub fn build(self) -> Vec<AgentEvent> {
        self.events
    }
}

impl Default for EventBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Assert helper for event stream validation.
pub struct EventStreamAssertions;

impl EventStreamAssertions {
    /// Assert that an event stream produces exactly one RunStarted.
    pub async fn assert_single_run_started(stream: EventStream) -> Vec<AgentEvent> {
        use futures_util::StreamExt;
        let events = stream.collect::<Vec<_>>().await;
        let run_started_count = events
            .iter()
            .filter(|e| matches!(e, Ok(AgentEvent::RunStarted { .. })))
            .count();
        assert_eq!(
            run_started_count, 1,
            "Expected exactly 1 RunStarted, got {}",
            run_started_count
        );
        events.into_iter().filter_map(|r| r.ok()).collect()
    }

    /// Assert that an event stream produces exactly one terminal event.
    pub async fn assert_single_terminal(stream: EventStream) -> Vec<AgentEvent> {
        use futures_util::StreamExt;
        let events = stream.collect::<Vec<_>>().await;
        let terminal_count = events
            .iter()
            .filter(|e| matches!(e, Ok(e) if e.is_terminal()))
            .count();
        assert_eq!(
            terminal_count, 1,
            "Expected exactly 1 terminal event, got {}",
            terminal_count
        );
        events.into_iter().filter_map(|r| r.ok()).collect()
    }

    /// Assert that an event stream produces no errors.
    pub async fn assert_no_errors(stream: EventStream) -> Vec<AgentEvent> {
        use futures_util::StreamExt;
        let events = stream.collect::<Vec<_>>().await;
        let error_count = events.iter().filter(|e| e.is_err()).count();
        assert_eq!(error_count, 0, "Expected no errors, got {}", error_count);
        events.into_iter().filter_map(|r| r.ok()).collect()
    }

    /// Collect all text deltas into a single string.
    pub async fn collect_text(stream: EventStream) -> String {
        use futures_util::StreamExt;
        let events = stream.collect::<Vec<_>>().await;
        events
            .iter()
            .filter_map(|r| r.as_ref().ok())
            .filter_map(|e| {
                if let AgentEvent::TextDelta { text, .. } = e {
                    Some(text.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fake_adapter() {
        let adapter = FakeAdapter::new(ProviderId::Claude);
        assert_eq!(adapter.id(), ProviderId::Claude);
        assert!(adapter.capabilities().tools);
    }

    #[tokio::test]
    async fn test_fake_adapter_with_events() {
        let adapter =
            FakeAdapter::new(ProviderId::Claude).with_events(vec![AgentEvent::TextDelta {
                role: Role::Assistant,
                text: "hello".to_string(),
            }]);

        let stream = adapter
            .run(ChatRequest::new("test", "/tmp", ProviderId::Claude))
            .await
            .unwrap();
        let text = EventStreamAssertions::collect_text(stream).await;
        assert_eq!(text, "hello");
    }

    #[test]
    fn test_event_builder() {
        let events = EventBuilder::new()
            .with_text_delta("Hello ")
            .with_text_delta("World")
            .with_run_completed(Some("Hello World"))
            .build();

        assert_eq!(events.len(), 3);
        assert!(matches!(events[0], AgentEvent::TextDelta { .. }));
        assert!(matches!(events[2], AgentEvent::RunCompleted { .. }));
    }

    #[test]
    fn test_parse_jsonl_lines() {
        let jsonl = r#"{"type": "text_delta", "delta": {"text": "hello"}}
{"type": "text_delta", "delta": {"text": " world"}}
{"type": "tool_call", "id": "1", "name": "read_file", "input": {}}"#;

        let events = parse_jsonl_lines(jsonl);
        assert_eq!(events.len(), 4); // 2 text deltas + 1 tool + 1 run completed
    }

    #[test]
    fn test_fake_cancellation_token() {
        let token = FakeCancellationToken::new();
        assert!(!token.is_cancelled());
        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn test_event_builder_with_tool_call() {
        let events = EventBuilder::new()
            .with_tool_call("tool1", "read_file")
            .with_tool_finished("tool1", Some("{}"))
            .build();

        assert_eq!(events.len(), 2);
    }
}
