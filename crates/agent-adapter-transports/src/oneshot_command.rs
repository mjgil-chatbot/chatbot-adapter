//! One-shot command transport implementation.
//!
//! This transport spawns a command, collects stdout as final text,
//! maps stderr to logs, and returns completion/failure by exit status.

use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use uuid::Uuid;

use agent_adapter_core::{AdapterError, AdapterResult, AgentEvent, ChatRequest, LogLevel, Role};

use super::TransportStream;

/// One-shot command transport.
pub struct OneShotCommandTransport {
    exe: String,
    args: Vec<String>,
}

impl OneShotCommandTransport {
    pub fn new(exe: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            exe: exe.into(),
            args,
        }
    }

    fn build_command(&self, request: &ChatRequest) -> Command {
        let mut cmd = Command::new(&self.exe);
        cmd.args(&self.args);

        // Set working directory
        cmd.current_dir(&request.cwd);

        // Setup environment - use only the allowed env vars
        cmd.env_clear();
        for (key, value) in &request.env {
            cmd.env(key, value);
        }

        // Pass context directories (workspace includes for sandbox)
        let context_dirs: Vec<String> = request
            .files
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        if !context_dirs.is_empty() {
            cmd.arg("--context-dirs").arg(context_dirs.join(","));
        }

        // For one-shot, we typically send prompt via stdin or arg
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Kill process group on drop
        cmd.kill_on_drop(true);

        cmd
    }

    pub async fn run_stream(&self, request: ChatRequest) -> AdapterResult<TransportStream> {
        let mut cmd = self.build_command(&request);
        let exe_name = self.exe.clone();
        let timeout_ms = request.timeout_ms;

        let mut child = cmd.spawn().map_err(|e| AdapterError::TransportError {
            message: format!("Failed to spawn {}: {}", self.exe, e),
        })?;

        // Read stdout and stderr concurrently, with optional timeout
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let (status, stdout_result, stderr_result) = if let Some(timeout_ms) = timeout_ms {
            let duration = std::time::Duration::from_millis(timeout_ms);
            tokio::time::timeout(duration, async {
                tokio::join! {
                    async { child.wait().await },
                    async {
                        if let Some(mut stdout) = stdout {
                            let mut buf = Vec::new();
                            let _ = stdout.read_to_end(&mut buf).await;
                            buf
                        } else {
                            Vec::new()
                        }
                    },
                    async {
                        if let Some(mut stderr) = stderr {
                            let mut buf = Vec::new();
                            let _ = stderr.read_to_end(&mut buf).await;
                            buf
                        } else {
                            Vec::new()
                        }
                    },
                }
            })
            .await
            .map_err(|_| AdapterError::Timeout {
                run_id: uuid::Uuid::new_v4().to_string(),
            })?
        } else {
            tokio::join! {
                async { child.wait().await },
                async {
                    if let Some(mut stdout) = stdout {
                        let mut buf = Vec::new();
                        let _ = stdout.read_to_end(&mut buf).await;
                        buf
                    } else {
                        Vec::new()
                    }
                },
                async {
                    if let Some(mut stderr) = stderr {
                        let mut buf = Vec::new();
                        let _ = stderr.read_to_end(&mut buf).await;
                        buf
                    } else {
                        Vec::new()
                    }
                },
            }
        };

        let stdout = stdout_result;
        let stderr = stderr_result;

        let run_id = Uuid::new_v4();
        let provider = request.provider.clone();

        let (tx, rx) = tokio::sync::mpsc::channel(10);
        let tx_clone = tx.clone();

        // Spawn a task to emit events
        tokio::spawn(async move {
            // Send RunStarted
            let _ = tx_clone
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

            // Parse stdout as text (one-shot returns full text)
            let stdout_text = String::from_utf8_lossy(&stdout).to_string();

            if !stdout_text.is_empty() {
                // For one-shot, we emit the full text at once or in chunks
                let chunks: Vec<&str> = stdout_text.lines().collect();
                for chunk in chunks {
                    if !chunk.is_empty() {
                        let _ = tx_clone
                            .send(Ok(AgentEvent::TextDelta {
                                role: Role::Assistant,
                                text: chunk.to_string(),
                            }))
                            .await;
                    }
                }
            }

            // Map stderr to Log events
            let stderr_text = String::from_utf8_lossy(&stderr).to_string();
            if !stderr_text.is_empty() {
                for line in stderr_text.lines() {
                    let _ = tx_clone
                        .send(Ok(AgentEvent::Log {
                            level: LogLevel::Warn,
                            message: line.to_string(),
                        }))
                        .await;
                }
            }

            // Send completion based on exit status
            match status {
                Ok(status) if status.success() => {
                    let _ = tx_clone
                        .send(Ok(AgentEvent::RunCompleted {
                            final_text: Some(stdout_text),
                            external_session: None,
                        }))
                        .await;
                }
                Ok(status) => {
                    let _ = tx_clone
                        .send(Ok(AgentEvent::RunFailed {
                            code: status.code(),
                            message: format!("{} exited with code {:?}", exe_name, status.code()),
                        }))
                        .await;
                }
                Err(e) => {
                    let _ = tx_clone
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::time::Duration;
    use tokio_stream::StreamExt;

    fn test_request() -> ChatRequest {
        ChatRequest::new("test prompt", ".", agent_adapter_core::ProviderId::Claude)
    }

    fn test_request_with_env() -> ChatRequest {
        let mut env = HashMap::new();
        env.insert("TEST_VAR".to_string(), "test_value".to_string());
        env.insert("ANOTHER_VAR".to_string(), "another_value".to_string());

        ChatRequest {
            prompt: "test prompt".to_string(),
            cwd: std::path::PathBuf::from("."),
            provider: agent_adapter_core::ProviderId::Claude,
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

    #[tokio::test]
    async fn test_oneshot_transport_build_command() {
        let transport = OneShotCommandTransport::new("echo", vec!["hello".to_string()]);
        let request = test_request();
        let _cmd = transport.build_command(&request);
    }

    #[tokio::test]
    async fn test_oneshot_transport_new() {
        let transport = OneShotCommandTransport::new("cat", vec!["-n".to_string()]);
        assert_eq!(transport.exe, "cat");
        assert_eq!(transport.args, vec!["-n"]);
    }

    #[tokio::test]
    async fn test_oneshot_stdout_emits_text_delta() {
        let transport = OneShotCommandTransport::new("echo", vec!["hello world".to_string()]);
        let request = test_request();
        let stream = transport.run_stream(request).await.unwrap();

        let events: Vec<_> = tokio::time::timeout(Duration::from_secs(5), stream.collect())
            .await
            .unwrap();

        // Find text delta events
        let text_deltas: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                Ok(AgentEvent::TextDelta { text, .. }) => Some(text.clone()),
                _ => None,
            })
            .collect();

        assert!(
            !text_deltas.is_empty(),
            "Expected at least one TextDelta event"
        );
        assert!(
            text_deltas.iter().any(|t| t.contains("hello world")),
            "Expected 'hello world' in text deltas"
        );
    }

    #[tokio::test]
    async fn test_oneshot_stderr_emits_log() {
        let transport = OneShotCommandTransport::new(
            "sh",
            vec!["-c".to_string(), "echo error >&2".to_string()],
        );
        let request = test_request();
        let stream = transport.run_stream(request).await.unwrap();

        let events: Vec<_> = tokio::time::timeout(Duration::from_secs(5), stream.collect())
            .await
            .unwrap();

        // Find log events
        let log_events: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                Ok(AgentEvent::Log { message, .. }) => Some(message.clone()),
                _ => None,
            })
            .collect();

        assert!(
            !log_events.is_empty(),
            "Expected at least one Log event for stderr"
        );
    }

    #[tokio::test]
    async fn test_oneshot_nonzero_exit_emits_run_failed() {
        let transport =
            OneShotCommandTransport::new("sh", vec!["-c".to_string(), "exit 42".to_string()]);
        let request = test_request();
        let stream = transport.run_stream(request).await.unwrap();

        let events: Vec<_> = tokio::time::timeout(Duration::from_secs(5), stream.collect())
            .await
            .unwrap();

        // Find RunFailed event
        let has_failed = events
            .iter()
            .any(|e| matches!(e, Ok(AgentEvent::RunFailed { code: Some(42), .. })));

        assert!(has_failed, "Expected RunFailed with code 42");
    }

    #[tokio::test]
    async fn test_oneshot_success_emits_run_completed() {
        let transport = OneShotCommandTransport::new("echo", vec!["success".to_string()]);
        let request = test_request();
        let stream = transport.run_stream(request).await.unwrap();

        let events: Vec<_> = tokio::time::timeout(Duration::from_secs(5), stream.collect())
            .await
            .unwrap();

        // Find RunCompleted event
        let has_completed = events
            .iter()
            .any(|e| matches!(e, Ok(AgentEvent::RunCompleted { .. })));

        assert!(has_completed, "Expected RunCompleted event on success");
    }

    #[tokio::test]
    async fn test_oneshot_run_started_first() {
        let transport = OneShotCommandTransport::new("echo", vec!["test".to_string()]);
        let request = test_request();
        let stream = transport.run_stream(request).await.unwrap();

        let events: Vec<_> = tokio::time::timeout(Duration::from_secs(5), stream.collect())
            .await
            .unwrap();

        // First event should be RunStarted
        assert!(!events.is_empty(), "Expected at least one event");
        assert!(
            matches!(&events[0], Ok(AgentEvent::RunStarted { .. })),
            "First event should be RunStarted"
        );
    }

    #[tokio::test]
    async fn test_oneshot_env_cleared_and_set() {
        let transport = OneShotCommandTransport::new(
            "sh",
            vec!["-c".to_string(), "echo $TEST_VAR".to_string()],
        );
        let request = test_request_with_env();
        let stream = transport.run_stream(request).await.unwrap();

        let events: Vec<_> = tokio::time::timeout(Duration::from_secs(5), stream.collect())
            .await
            .unwrap();

        // Should have output with env var value
        let has_text = events.iter().any(|e| match e {
            Ok(AgentEvent::TextDelta { text, .. }) => text.contains("test_value"),
            _ => false,
        });
        assert!(
            has_text,
            "Expected env var TEST_VAR to be set to 'test_value'"
        );
    }

    #[tokio::test]
    async fn test_oneshot_multiline_stdout() {
        let transport = OneShotCommandTransport::new(
            "sh",
            vec![
                "-c".to_string(),
                "printf 'line1\\nline2\\nline3'".to_string(),
            ],
        );
        let request = test_request();
        let stream = transport.run_stream(request).await.unwrap();

        let events: Vec<_> = tokio::time::timeout(Duration::from_secs(5), stream.collect())
            .await
            .unwrap();

        // Should have multiple text deltas for each line
        let text_deltas: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                Ok(AgentEvent::TextDelta { text, .. }) => Some(text.clone()),
                _ => None,
            })
            .collect();

        assert!(
            text_deltas.len() >= 2,
            "Expected multiple TextDelta events for multiline output"
        );
    }

    #[tokio::test]
    async fn test_oneshot_empty_stdout() {
        let transport = OneShotCommandTransport::new("true".to_string(), vec![]);
        let request = test_request();
        let stream = transport.run_stream(request).await.unwrap();

        let events: Vec<_> = tokio::time::timeout(Duration::from_secs(5), stream.collect())
            .await
            .unwrap();

        // Should still get RunCompleted even with empty stdout
        let has_completed = events
            .iter()
            .any(|e| matches!(e, Ok(AgentEvent::RunCompleted { .. })));
        assert!(
            has_completed,
            "Expected RunCompleted even with empty stdout"
        );
    }

    #[tokio::test]
    async fn test_oneshot_provides_run_id_and_provider() {
        let transport = OneShotCommandTransport::new("echo", vec!["test".to_string()]);
        let request = test_request();
        let stream = transport.run_stream(request).await.unwrap();

        let events: Vec<_> = tokio::time::timeout(Duration::from_secs(5), stream.collect())
            .await
            .unwrap();

        // Should have RunStarted with run_id and provider
        let has_run_started = events.iter().any(|e| match e {
            Ok(AgentEvent::RunStarted { provider, .. }) => {
                *provider == agent_adapter_core::ProviderId::Claude
            }
            _ => false,
        });
        assert!(has_run_started, "Expected RunStarted with correct provider");
    }

    #[tokio::test]
    async fn test_oneshot_stderr_lines_as_log_events() {
        let transport = OneShotCommandTransport::new(
            "sh",
            vec![
                "-c".to_string(),
                "echo line1 >&2; echo line2 >&2".to_string(),
            ],
        );
        let request = test_request();
        let stream = transport.run_stream(request).await.unwrap();

        let events: Vec<_> = tokio::time::timeout(Duration::from_secs(5), stream.collect())
            .await
            .unwrap();

        // Should have log events for each stderr line
        let log_events: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                Ok(AgentEvent::Log { message, .. }) => Some(message.clone()),
                _ => None,
            })
            .collect();

        assert!(
            log_events.len() >= 2,
            "Expected at least 2 log events for stderr lines"
        );
    }

    #[tokio::test]
    async fn test_oneshot_failed_with_code_none() {
        // Test case where spawn fails (nonexistent command)
        let transport = OneShotCommandTransport::new("nonexistent_command_12345", vec![]);
        let request = test_request();

        // The run_stream should return an error on spawn failure
        let result = transport.run_stream(request).await;
        assert!(result.is_err(), "Expected error for nonexistent command");
    }

    #[tokio::test]
    async fn test_oneshot_final_text_contains_full_output() {
        let transport = OneShotCommandTransport::new("echo", vec!["final output".to_string()]);
        let request = test_request();
        let stream = transport.run_stream(request).await.unwrap();

        let events: Vec<_> = tokio::time::timeout(Duration::from_secs(5), stream.collect())
            .await
            .unwrap();

        // Find RunCompleted with final_text
        let final_text = events.iter().find_map(|e| match e {
            Ok(AgentEvent::RunCompleted { final_text, .. }) => final_text.clone(),
            _ => None,
        });

        assert!(
            final_text.is_some(),
            "Expected RunCompleted with final_text"
        );
        assert!(
            final_text.unwrap().contains("final output"),
            "Expected final_text to contain 'final output'"
        );
    }

    // =========================================================================
    // Lifecycle invariant tests (Task 7 / Task 31)
    // =========================================================================

    #[tokio::test]
    async fn test_oneshot_exactly_one_run_started() {
        let transport = OneShotCommandTransport::new("echo", vec!["test".to_string()]);
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
    async fn test_oneshot_exactly_one_terminal_event() {
        let transport = OneShotCommandTransport::new("echo", vec!["test".to_string()]);
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
    async fn test_oneshot_run_started_before_other_events() {
        let transport = OneShotCommandTransport::new("echo", vec!["test".to_string()]);
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
    async fn test_oneshot_no_events_after_terminal() {
        let transport = OneShotCommandTransport::new("echo", vec!["test".to_string()]);
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
    async fn test_oneshot_cwd_env_respected() {
        let transport = OneShotCommandTransport::new("pwd", vec![]);
        let mut request = test_request();
        request.cwd = std::path::PathBuf::from("/tmp");
        let stream = transport.run_stream(request).await.unwrap();

        let events: Vec<_> = tokio::time::timeout(Duration::from_secs(5), stream.collect())
            .await
            .unwrap();

        // Should have output containing /tmp
        let has_tmp = events.iter().any(|e| match e {
            Ok(AgentEvent::TextDelta { text, .. }) => text.contains("/tmp"),
            _ => false,
        });
        assert!(has_tmp, "Expected pwd to show /tmp as current directory");
    }

    #[tokio::test]
    async fn test_oneshot_timeout_ms_triggers_timeout() {
        // timeout_ms handling is implemented in the transport layer
        let transport = OneShotCommandTransport::new("sleep".to_string(), vec!["10".to_string()]);
        let mut request = test_request();
        request.timeout_ms = Some(100); // 100ms timeout - should fire before sleep completes

        let result = transport.run_stream(request).await;
        // Should timeout and return error
        assert!(result.is_err(), "Expected timeout error");
        // Use err() which requires Display but not Debug
        let err = result.err().expect("Should have gotten an error");
        assert!(
            matches!(err, AdapterError::Timeout { .. }),
            "Expected Timeout error"
        );
    }
}
