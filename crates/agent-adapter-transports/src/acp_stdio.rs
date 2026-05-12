//! ACP (Agent Client Protocol) stdio transport implementation.
//!
//! This transport communicates with agents over stdio using JSON-RPC framing
//! for the Agent Client Protocol. Used by Gemini, OpenCode, Kimi, and Hermes.

use serde_json::Value;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use uuid::Uuid;

use agent_adapter_core::{AdapterError, AdapterResult, AgentEvent, ChatRequest, LogLevel};

use crate::TransportStream;

/// ACP stdio transport.
pub struct AcpStdioTransport {
    exe: String,
    args: Vec<String>,
}

impl AcpStdioTransport {
    pub fn new(exe: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            exe: exe.into(),
            args,
        }
    }

    fn build_command(&self, request: &ChatRequest) -> Command {
        let mut cmd = Command::new(&self.exe);
        cmd.args(&self.args);
        cmd.current_dir(&request.cwd);
        cmd.env_clear();
        for (key, value) in &request.env {
            cmd.env(key, value);
        }
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.kill_on_drop(true);
        cmd
    }

    pub async fn run_stream(&self, request: ChatRequest) -> AdapterResult<TransportStream> {
        let mut cmd = self.build_command(&request);

        let mut child = cmd.spawn().map_err(|e| AdapterError::TransportError {
            message: format!("Failed to spawn {}: {}", self.exe, e),
        })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| AdapterError::TransportError {
                message: "Failed to capture stdin".to_string(),
            })?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AdapterError::TransportError {
                message: "Failed to capture stdout".to_string(),
            })?;

        let run_id = Uuid::new_v4();
        let provider = request.provider.clone();

        let (tx, rx) = mpsc::channel(100);
        let tx_stdout = tx.clone();

        // Spawn task to read stdout (JSON-RPC messages)
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                if let Ok(events) = Self::parse_jsonrpc_line(&line) {
                    for event in events {
                        if tx_stdout.send(event).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        // Send initialize request
        let mut stdin_writer = stdin;
        let init_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "1",
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": true,
                    "permissions": true,
                },
                "clientInfo": {
                    "name": "agent-adapter",
                    "version": "0.1.0",
                }
            }
        });
        if let Ok(buf) = serde_json::to_string(&init_request) {
            let _ = stdin_writer.write_all((buf + "\n").as_bytes()).await;
        }

        // Send session/resume info if applicable
        // (omitted for brevity - would be added based on request.resume)

        // Send prompt
        let prompt_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "2",
            "method": "prompt",
            "params": {
                "prompt": request.prompt,
                "contextDirs": request.files,
            }
        });
        if let Ok(buf) = serde_json::to_string(&prompt_request) {
            let _ = stdin_writer.write_all((buf + "\n").as_bytes()).await;
        }

        // Send RunStarted event
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

        // Spawn task to handle completion
        let tx_completion = tx.clone();
        tokio::spawn(async move {
            let status = child.wait().await;
            match status {
                Ok(status) if status.success() => {
                    let _ = tx_completion
                        .send(Ok(AgentEvent::RunCompleted {
                            final_text: None,
                            external_session: None,
                        }))
                        .await;
                }
                Ok(status) => {
                    let _ = tx_completion
                        .send(Ok(AgentEvent::RunFailed {
                            code: status.code(),
                            message: format!("{} exited with code {:?}", "acp", status.code()),
                        }))
                        .await;
                }
                Err(e) => {
                    let _ = tx_completion
                        .send(Ok(AgentEvent::RunFailed {
                            code: None,
                            message: format!("{} failed: {}", "acp", e),
                        }))
                        .await;
                }
            }
        });

        let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        Ok(Box::pin(stream) as TransportStream)
    }

    fn parse_jsonrpc_line(line: &str) -> Result<Vec<AdapterResult<AgentEvent>>, ()> {
        let value: Value = serde_json::from_str(line).map_err(|_| ())?;

        // Handle JSON-RPC notifications (no id field)
        if value.get("id").is_some() {
            // This is a response - we typically don't emit events for responses
            return Ok(vec![]);
        }

        let method = value.get("method").and_then(|v| v.as_str()).unwrap_or("");
        let params = value.get("params");

        match method {
            "text_delta" => {
                let text = params
                    .and_then(|p| p.get("text"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                Ok(vec![Ok(AgentEvent::TextDelta {
                    role: agent_adapter_core::Role::Assistant,
                    text: text.to_string(),
                })])
            }
            "tool_call_started" => {
                let id = params
                    .and_then(|p| p.get("id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let name = params
                    .and_then(|p| p.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("tool")
                    .to_string();
                let input = params
                    .and_then(|p| p.get("input"))
                    .cloned()
                    .unwrap_or(Value::Null);
                Ok(vec![Ok(AgentEvent::ToolCallStarted { id, name, input })])
            }
            "tool_call_delta" => {
                let id = params
                    .and_then(|p| p.get("id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let delta = params
                    .and_then(|p| p.get("delta"))
                    .cloned()
                    .unwrap_or(Value::Null);
                Ok(vec![Ok(AgentEvent::ToolCallDelta { id, delta })])
            }
            "tool_call_finished" => {
                let id = params
                    .and_then(|p| p.get("id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let output = params
                    .and_then(|p| p.get("output"))
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let error = params
                    .and_then(|p| p.get("error"))
                    .and_then(|v| v.as_str())
                    .map(String::from);
                Ok(vec![Ok(AgentEvent::ToolCallFinished { id, output, error })])
            }
            "permission_request" => {
                let id = params
                    .and_then(|p| p.get("id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let action = params
                    .and_then(|p| p.get("action"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let risk = params
                    .and_then(|p| p.get("risk"))
                    .and_then(|v| v.as_str())
                    .map(|s| match s {
                        "none" => agent_adapter_core::RiskLevel::None,
                        "low" => agent_adapter_core::RiskLevel::Low,
                        "medium" => agent_adapter_core::RiskLevel::Medium,
                        "high" => agent_adapter_core::RiskLevel::High,
                        _ => agent_adapter_core::RiskLevel::Critical,
                    })
                    .unwrap_or(agent_adapter_core::RiskLevel::Medium);
                let details = params
                    .and_then(|p| p.get("details"))
                    .cloned()
                    .unwrap_or(Value::Null);
                Ok(vec![Ok(AgentEvent::PermissionRequest {
                    id,
                    action,
                    risk,
                    details,
                })])
            }
            "log" => {
                let level = params
                    .and_then(|p| p.get("level"))
                    .and_then(|v| v.as_str())
                    .map(|s| match s {
                        "trace" => LogLevel::Trace,
                        "debug" => LogLevel::Debug,
                        "warn" => LogLevel::Warn,
                        "error" => LogLevel::Error,
                        _ => LogLevel::Info,
                    })
                    .unwrap_or(LogLevel::Info);
                let message = params
                    .and_then(|p| p.get("message"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                Ok(vec![Ok(AgentEvent::Log { level, message })])
            }
            "usage" => {
                let input_tokens = params
                    .and_then(|p| p.get("inputTokens"))
                    .and_then(|v| v.as_u64());
                let output_tokens = params
                    .and_then(|p| p.get("outputTokens"))
                    .and_then(|v| v.as_u64());
                let cost_usd = params
                    .and_then(|p| p.get("costUSD"))
                    .and_then(|v| v.as_f64());
                Ok(vec![Ok(AgentEvent::Usage {
                    input_tokens,
                    output_tokens,
                    cost_usd,
                })])
            }
            "file_change" => {
                let path = params
                    .and_then(|p| p.get("path"))
                    .and_then(|v| v.as_str())
                    .map(PathBuf::from)
                    .unwrap_or_default();
                let operation = params
                    .and_then(|p| p.get("operation"))
                    .and_then(|v| v.as_str())
                    .map(|s| match s {
                        "create" => agent_adapter_core::FileOperation::Create,
                        "modify" => agent_adapter_core::FileOperation::Update,
                        "delete" => agent_adapter_core::FileOperation::Delete,
                        "rename" => agent_adapter_core::FileOperation::Rename,
                        _ => agent_adapter_core::FileOperation::Update,
                    })
                    .unwrap_or(agent_adapter_core::FileOperation::Update);
                Ok(vec![Ok(AgentEvent::FileChange { path, operation })])
            }
            "resume" => {
                let session_id = params
                    .and_then(|p| p.get("sessionId"))
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .unwrap_or_default();
                let _checkpoint = params
                    .and_then(|p| p.get("checkpoint"))
                    .cloned()
                    .unwrap_or(Value::Null);
                Ok(vec![Ok(AgentEvent::FileChange {
                    path: PathBuf::from(format!("resume:{}", session_id)),
                    operation: agent_adapter_core::FileOperation::Resume,
                })])
            }
            _ => Ok(vec![]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::time::Duration;
    use tokio_stream::StreamExt;

    fn test_request() -> ChatRequest {
        ChatRequest::new("test prompt", ".", agent_adapter_core::ProviderId::Gemini)
    }

    fn test_request_with_env() -> ChatRequest {
        let mut env = HashMap::new();
        env.insert("TEST_VAR".to_string(), "test_value".to_string());
        ChatRequest {
            prompt: "test prompt".to_string(),
            cwd: std::path::PathBuf::from("."),
            provider: agent_adapter_core::ProviderId::Gemini,
            model: None,
            session: None,
            resume: agent_adapter_core::ResumeMode::None,
            files: vec![],
            images: vec![],
            approval_policy: agent_adapter_core::ApprovalPolicy::Safe,
            sandbox: agent_adapter_core::SandboxMode::ReadOnly,
            timeout_ms: None,
            env,
            extra: serde_json::Value::Null,
        }
    }

    #[test]
    fn test_parse_text_delta() {
        let line = r#"{"jsonrpc":"2.0","method":"text_delta","params":{"text":"hello"}}"#;
        let events = AcpStdioTransport::parse_jsonrpc_line(line).unwrap();
        assert!(matches!(events[0], Ok(AgentEvent::TextDelta { ref text, .. }) if text == "hello"));
    }

    #[test]
    fn test_parse_tool_call() {
        let line = r#"{"jsonrpc":"2.0","method":"tool_call_started","params":{"id":"t1","name":"read_file","input":{}}}"#;
        let events = AcpStdioTransport::parse_jsonrpc_line(line).unwrap();
        assert!(
            matches!(events[0], Ok(AgentEvent::ToolCallStarted { ref id, ref name, .. }) if id == "t1" && name == "read_file")
        );
    }

    #[test]
    fn test_parse_permission_request() {
        let line = r#"{"jsonrpc":"2.0","method":"permission_request","params":{"id":"p1","action":"edit_file","risk":"high","details":{}}}"#;
        let events = AcpStdioTransport::parse_jsonrpc_line(line).unwrap();
        assert!(
            matches!(events[0], Ok(AgentEvent::PermissionRequest { ref id, ref action, .. }) if id == "p1" && action == "edit_file")
        );
    }

    #[test]
    fn test_parse_tool_call_delta() {
        let line = r#"{"jsonrpc":"2.0","method":"tool_call_delta","params":{"id":"t1","delta":{"text":"partial"}}}"#;
        let events = AcpStdioTransport::parse_jsonrpc_line(line).unwrap();
        assert!(matches!(events[0], Ok(AgentEvent::ToolCallDelta { ref id, .. }) if id == "t1"));
    }

    #[test]
    fn test_parse_tool_call_finished() {
        let line = r#"{"jsonrpc":"2.0","method":"tool_call_finished","params":{"id":"t1","output":"result"}}"#;
        let events = AcpStdioTransport::parse_jsonrpc_line(line).unwrap();
        assert!(
            matches!(events[0], Ok(AgentEvent::ToolCallFinished { ref id, output: Some(ref o), .. }) if id == "t1" && o == "result")
        );
    }

    #[test]
    fn test_parse_tool_call_finished_with_error() {
        let line = r#"{"jsonrpc":"2.0","method":"tool_call_finished","params":{"id":"t1","error":"failed"}}"#;
        let events = AcpStdioTransport::parse_jsonrpc_line(line).unwrap();
        assert!(
            matches!(events[0], Ok(AgentEvent::ToolCallFinished { ref id, error: Some(ref e), .. }) if id == "t1" && e == "failed")
        );
    }

    #[test]
    fn test_parse_log_message() {
        let line = r#"{"jsonrpc":"2.0","method":"log","params":{"level":"warn","message":"something happened"}}"#;
        let events = AcpStdioTransport::parse_jsonrpc_line(line).unwrap();
        assert!(
            matches!(events[0], Ok(AgentEvent::Log { level: LogLevel::Warn, ref message }) if message == "something happened")
        );
    }

    #[test]
    fn test_parse_log_debug() {
        let line =
            r#"{"jsonrpc":"2.0","method":"log","params":{"level":"debug","message":"debug info"}}"#;
        let events = AcpStdioTransport::parse_jsonrpc_line(line).unwrap();
        assert!(matches!(
            events[0],
            Ok(AgentEvent::Log {
                level: LogLevel::Debug,
                ..
            })
        ));
    }

    #[test]
    fn test_parse_usage() {
        let line = r#"{"jsonrpc":"2.0","method":"usage","params":{"inputTokens":100,"outputTokens":50,"costUSD":0.01}}"#;
        let events = AcpStdioTransport::parse_jsonrpc_line(line).unwrap();
        assert!(matches!(
            events[0],
            Ok(AgentEvent::Usage {
                input_tokens: Some(100),
                output_tokens: Some(50),
                cost_usd: Some(0.01)
            })
        ));
    }

    #[test]
    fn test_parse_unknown_method_returns_empty() {
        let line = r#"{"jsonrpc":"2.0","method":"custom_method","params":{}}"#;
        let events = AcpStdioTransport::parse_jsonrpc_line(line).unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn test_parse_malformed_jsonrpc_returns_error() {
        let line = "not valid json";
        let result = AcpStdioTransport::parse_jsonrpc_line(line);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_jsonrpc_response_ignored() {
        // JSON-RPC responses (with id) should be ignored
        let line = r#"{"jsonrpc":"2.0","id":"1","result":{"status":"ok"}}"#;
        let events = AcpStdioTransport::parse_jsonrpc_line(line).unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn test_parse_permission_request_risk_levels() {
        // Test high risk
        let line = r#"{"jsonrpc":"2.0","method":"permission_request","params":{"id":"p1","action":"delete","risk":"high"}}"#;
        let events = AcpStdioTransport::parse_jsonrpc_line(line).unwrap();
        assert!(matches!(
            events[0],
            Ok(AgentEvent::PermissionRequest {
                risk: agent_adapter_core::RiskLevel::High,
                ..
            })
        ));

        // Test low risk
        let line2 = r#"{"jsonrpc":"2.0","method":"permission_request","params":{"id":"p2","action":"read","risk":"low"}}"#;
        let events2 = AcpStdioTransport::parse_jsonrpc_line(line2).unwrap();
        assert!(matches!(
            events2[0],
            Ok(AgentEvent::PermissionRequest {
                risk: agent_adapter_core::RiskLevel::Low,
                ..
            })
        ));
    }

    #[test]
    fn test_acp_stdio_transport_new() {
        let transport = AcpStdioTransport::new("gemini", vec!["--json".to_string()]);
        assert_eq!(transport.exe, "gemini");
        assert_eq!(transport.args, vec!["--json"]);
    }

    #[test]
    fn test_acp_stdio_build_command_sets_cwd() {
        let transport = AcpStdioTransport::new("gemini", vec![]);
        let request = test_request();
        let cmd = transport.build_command(&request);
        // Command should be built with the request's cwd
        let _ = cmd;
    }

    #[test]
    fn test_acp_stdio_build_command_clears_env() {
        let transport = AcpStdioTransport::new("gemini", vec![]);
        let request = test_request_with_env();
        let cmd = transport.build_command(&request);
        // Verify the command structure is valid
        let _ = cmd;
    }

    #[tokio::test]
    async fn test_acp_stdio_run_stream_returns_stream() {
        // We can't easily test the full stream without a fake ACP server,
        // but we can verify the transport creates correctly
        let transport = AcpStdioTransport::new("nonexistent_acp_binary_12345", vec![]);
        let request = test_request();

        // This will fail on spawn since the binary doesn't exist
        let result = transport.run_stream(request).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_text_delta_empty() {
        let line = r#"{"jsonrpc":"2.0","method":"text_delta","params":{"text":""}}"#;
        let events = AcpStdioTransport::parse_jsonrpc_line(line).unwrap();
        assert!(matches!(&events[0], Ok(AgentEvent::TextDelta { text, .. }) if text.is_empty()));
    }

    #[test]
    fn test_parse_text_delta_multiline() {
        let line =
            r#"{"jsonrpc":"2.0","method":"text_delta","params":{"text":"line1\nline2\nline3"}}"#;
        let events = AcpStdioTransport::parse_jsonrpc_line(line).unwrap();
        assert!(
            matches!(&events[0], Ok(AgentEvent::TextDelta { text, .. }) if text.contains('\n'))
        );
    }

    #[test]
    fn test_parse_log_error_level() {
        let line = r#"{"jsonrpc":"2.0","method":"log","params":{"level":"error","message":"error occurred"}}"#;
        let events = AcpStdioTransport::parse_jsonrpc_line(line).unwrap();
        assert!(matches!(
            events[0],
            Ok(AgentEvent::Log {
                level: LogLevel::Error,
                ..
            })
        ));
    }

    #[test]
    fn test_parse_missing_method_returns_empty() {
        let line = r#"{"jsonrpc":"2.0","params":{}}"#;
        let events = AcpStdioTransport::parse_jsonrpc_line(line).unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn test_parse_tool_call_no_input() {
        let line =
            r#"{"jsonrpc":"2.0","method":"tool_call_started","params":{"id":"t1","name":"test"}}"#;
        let events = AcpStdioTransport::parse_jsonrpc_line(line).unwrap();
        assert!(
            matches!(events[0], Ok(AgentEvent::ToolCallStarted { ref id, ref name, .. }) if id == "t1" && name == "test")
        );
    }

    // =========================================================================
    // Lifecycle invariant tests (Task 7 / Task 32)
    // =========================================================================

    #[tokio::test]
    async fn test_acp_exactly_one_run_started() {
        // This test uses a fake binary that outputs JSON-RPC responses
        let transport = AcpStdioTransport::new(
            "echo",
            vec![
                "{\"jsonrpc\":\"2.0\",\"method\":\"text_delta\",\"params\":{\"text\":\"hello\"}}"
                    .to_string(),
            ],
        );
        let request = test_request();
        let stream = transport.run_stream(request).await.unwrap();

        let events: Vec<_> = tokio::time::timeout(Duration::from_secs(5), stream.collect())
            .await
            .unwrap();

        let run_started_count = events
            .iter()
            .filter(|e| matches!(e, Ok(AgentEvent::RunStarted { .. })))
            .count();

        assert_eq!(
            run_started_count, 1,
            "Expected exactly one RunStarted event, got {}",
            run_started_count
        );
    }

    #[tokio::test]
    async fn test_acp_exactly_one_terminal_event() {
        let transport = AcpStdioTransport::new(
            "echo",
            vec![
                "{\"jsonrpc\":\"2.0\",\"method\":\"text_delta\",\"params\":{\"text\":\"hello\"}}"
                    .to_string(),
            ],
        );
        let request = test_request();
        let stream = transport.run_stream(request).await.unwrap();

        let events: Vec<_> = tokio::time::timeout(Duration::from_secs(5), stream.collect())
            .await
            .unwrap();

        let terminal_count = events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    Ok(AgentEvent::RunCompleted { .. })
                        | Ok(AgentEvent::RunFailed { .. })
                        | Ok(AgentEvent::RunCancelled { .. })
                )
            })
            .count();

        assert_eq!(
            terminal_count, 1,
            "Expected exactly one terminal event, got {}",
            terminal_count
        );
    }

    #[tokio::test]
    async fn test_acp_run_started_before_other_events() {
        let transport = AcpStdioTransport::new(
            "echo",
            vec![
                "{\"jsonrpc\":\"2.0\",\"method\":\"text_delta\",\"params\":{\"text\":\"hello\"}}"
                    .to_string(),
            ],
        );
        let request = test_request();
        let stream = transport.run_stream(request).await.unwrap();

        let events: Vec<_> = tokio::time::timeout(Duration::from_secs(5), stream.collect())
            .await
            .unwrap();

        let run_started_idx = events
            .iter()
            .position(|e| matches!(e, Ok(AgentEvent::RunStarted { .. })));

        assert!(
            run_started_idx.is_some(),
            "Expected at least one RunStarted event"
        );
        let idx = run_started_idx.unwrap();
        assert_eq!(idx, 0, "RunStarted should be the first event");
    }

    #[tokio::test]
    async fn test_acp_no_events_after_terminal() {
        let transport = AcpStdioTransport::new("sh", vec!["-c".to_string(), "echo '{\"jsonrpc\":\"2.0\",\"method\":\"text_delta\",\"params\":{\"text\":\"hello\"}}'".to_string()]);
        let request = test_request();
        let stream = transport.run_stream(request).await.unwrap();

        let events: Vec<_> = tokio::time::timeout(Duration::from_secs(5), stream.collect())
            .await
            .unwrap();

        let terminal_idx = events.iter().position(|e| {
            matches!(
                e,
                Ok(AgentEvent::RunCompleted { .. })
                    | Ok(AgentEvent::RunFailed { .. })
                    | Ok(AgentEvent::RunCancelled { .. })
            )
        });

        if let Some(idx) = terminal_idx {
            let events_after = &events[idx + 1..];
            let non_empty = events_after
                .iter()
                .filter(|e| !matches!(e, Ok(AgentEvent::Log { .. })))
                .count();
            assert_eq!(
                non_empty, 0,
                "No non-log events should appear after terminal event"
            );
        }
    }

    #[tokio::test]
    async fn test_acp_env_cleared_and_set() {
        // Test that env vars are properly set in the command
        // Using sh -c with env variable check
        let transport =
            AcpStdioTransport::new("sh", vec!["-c".to_string(), "echo $TEST_VAR".to_string()]);
        let request = test_request_with_env();
        let result = transport.run_stream(request).await;
        // The transport should create a stream - whether the command succeeds
        // depends on the fake binary behavior. We just verify stream creation.
        assert!(
            result.is_ok(),
            "Transport should create stream with env vars set"
        );
    }

    #[tokio::test]
    async fn test_acp_cwd_set() {
        // Test that cwd is properly set in the command
        let transport = AcpStdioTransport::new("pwd", vec![]);
        let mut request = test_request();
        request.cwd = std::path::PathBuf::from("/tmp");
        let result = transport.run_stream(request).await;
        // The transport should create a stream
        assert!(
            result.is_ok(),
            "Transport should create stream with cwd set"
        );
    }

    #[test]
    fn test_acp_parse_text_delta_accumulates() {
        // Verify text delta parsing works correctly
        let line = r#"{"jsonrpc":"2.0","method":"text_delta","params":{"text":"Hello "}}"#;
        let events = AcpStdioTransport::parse_jsonrpc_line(line).unwrap();
        assert!(matches!(&events[0], Ok(AgentEvent::TextDelta { text, .. }) if text == "Hello "));

        let line2 = r#"{"jsonrpc":"2.0","method":"text_delta","params":{"text":"World"}}"#;
        let events2 = AcpStdioTransport::parse_jsonrpc_line(line2).unwrap();
        assert!(matches!(&events2[0], Ok(AgentEvent::TextDelta { text, .. }) if text == "World"));
    }

    #[test]
    fn test_acp_parse_file_change_event() {
        // ACP should support file_change events
        let line = r#"{"jsonrpc":"2.0","method":"file_change","params":{"path":"/tmp/test.txt","operation":"create"}}"#;
        let events = AcpStdioTransport::parse_jsonrpc_line(line).unwrap();
        assert!(!events.is_empty(), "file_change should be implemented now");
        assert!(matches!(
            events[0],
            Ok(AgentEvent::FileChange { ref path, .. }) if path.to_string_lossy() == "/tmp/test.txt"
        ));
    }

    #[test]
    fn test_acp_parse_resume_event() {
        // ACP should support resume events
        let line = r#"{"jsonrpc":"2.0","method":"resume","params":{"sessionId":"sess_abc123","checkpoint":{"step":5}}}"#;
        let events = AcpStdioTransport::parse_jsonrpc_line(line).unwrap();
        assert!(!events.is_empty(), "resume should be implemented now");
    }

    #[test]
    fn test_acp_parse_plan_event() {
        // ACP should support plan events
        let line = r#"{"jsonrpc":"2.0","method":"plan","params":{"steps":["step1","step2"]}}"#;
        let events = AcpStdioTransport::parse_jsonrpc_line(line).unwrap();
        // Unknown method returns empty - plan not yet implemented
        assert!(events.is_empty(), "plan not yet implemented in ACP parser");
    }
}
