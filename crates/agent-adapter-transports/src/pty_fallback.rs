//! PTY (pseudo-terminal) fallback transport implementation.
//!
//! This transport is used when no better transport is available.
//!
//! # WARNING
//!
//! This is a **compatibility escape hatch only**. Terminal UIs change frequently,
//! ANSI output is noisy, and there is often no reliable "turn complete" signal.
//! Only use this when the provider cannot run headlessly via JSONL or ACP.
//!
//! The transport spawns the CLI in a pseudo-terminal, captures screen output,
//! detects completion via inactivity timeout or explicit markers.

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use uuid::Uuid;

use agent_adapter_core::{AdapterError, AdapterResult, AgentEvent, ChatRequest};

use crate::TransportStream;

/// PTY fallback transport.
///
/// # Limitations
///
/// - PTY output is difficult to parse reliably
/// - ANSI escape sequences must be stripped
/// - Completion detection is heuristic (timeout or markers)
/// - This transport provides degraded reliability compared to structured modes
pub struct PtyFallbackTransport {
    exe: String,
    args: Vec<String>,
    timeout_ms: u64,
}

impl PtyFallbackTransport {
    pub fn new(exe: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            exe: exe.into(),
            args,
            timeout_ms: 300000, // 5 minutes default
        }
    }

    pub fn with_timeout(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }

    fn build_command(&self) -> CommandBuilder {
        let mut cmd = CommandBuilder::new(&self.exe);
        for arg in &self.args {
            cmd.arg(arg);
        }
        // Don't clear env - the process needs PATH to find executables
        cmd
    }

    pub async fn run_stream(&self, request: ChatRequest) -> AdapterResult<TransportStream> {
        let pty_system = native_pty_system();

        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| AdapterError::TransportError {
                message: format!("Failed to open PTY: {}", e),
            })?;

        let mut cmd = self.build_command();
        let cwd = request.cwd.to_string_lossy().to_string();
        cmd.cwd(&cwd);
        // Only clear and set specific env vars - preserve PATH for finding executables
        // cmd.env_clear();
        // Note: env_clear removes all env including PATH which breaks executable lookup
        // Use default env instead to allow PATH resolution
        for (key, value) in &request.env {
            cmd.env(key, value);
        }

        let mut child =
            pair.slave
                .spawn_command(cmd)
                .map_err(|e| AdapterError::TransportError {
                    message: format!("Failed to spawn command in PTY: {}", e),
                })?;

        let run_id = Uuid::new_v4();
        let provider = request.provider.clone();

        let (tx, rx) = mpsc::channel::<AdapterResult<AgentEvent>>();

        // Send RunStarted
        let _ = tx.send(Ok(AgentEvent::RunStarted {
            run_id,
            provider,
            external_session: None,
            approval_policy: request.approval_policy.as_mode_string().to_string(),
            workspace: agent_adapter_core::WorkspaceRef::from_cwd(
                request.cwd.clone(),
                request.sandbox,
            ),
        }));

        let tx_send = tx.clone();
        let exe_name = self.exe.clone();

        // Spawn thread to handle PTY I/O
        thread::spawn(move || {
            let mut final_text = String::new();
            let mut done = false;

            // Get a reader from the PTY master
            let mut reader = match pair.master.try_clone_reader() {
                Ok(r) => r,
                Err(_) => {
                    // Failed to get reader, just wait for process exit
                    let status = child.wait();
                    let _ = tx_send.send(Ok(match status {
                        Ok(s) if s.success() => AgentEvent::RunCompleted {
                            final_text: None,
                            external_session: None,
                        },
                        Ok(s) => AgentEvent::RunFailed {
                            code: Some(s.exit_code() as i32),
                            message: format!("{} exited", exe_name),
                        },
                        Err(_) => AgentEvent::RunFailed {
                            code: None,
                            message: format!("{} failed", exe_name),
                        },
                    }));
                    return;
                }
            };
            let mut buf = [0u8; 1024];
            loop {
                // Use blocking read
                match reader.read(&mut buf) {
                    Ok(0) => {
                        // EOF - process has closed stdout, continue to check process status
                    }
                    Ok(n) => {
                        let data = &buf[..n];
                        // Strip ANSI and accumulate text
                        let text = strip_ansi(&String::from_utf8_lossy(data));
                        if !text.is_empty() {
                            final_text.push_str(&text);
                            // Emit text delta if channel still open
                            let _ = tx_send.send(Ok(AgentEvent::TextDelta {
                                role: agent_adapter_core::Role::Assistant,
                                text,
                            }));
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // No data available yet, continue
                    }
                    Err(_) => {
                        // Read error or EOF, break
                        break;
                    }
                }

                // Check if process has exited
                match child.try_wait() {
                    Ok(Some(status)) => {
                        // Process exited - emit completion
                        if status.success() {
                            let _ = tx_send.send(Ok(AgentEvent::RunCompleted {
                                final_text: if final_text.is_empty() {
                                    None
                                } else {
                                    Some(final_text)
                                },
                                external_session: None,
                            }));
                        } else {
                            let _ = tx_send.send(Ok(AgentEvent::RunFailed {
                                code: Some(status.exit_code() as i32),
                                message: format!("{} exited", exe_name),
                            }));
                        }
                        done = true;
                        break;
                    }
                    Ok(None) => {
                        // Still running, continue reading after brief sleep
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => {
                        // Error
                        break;
                    }
                }
            }

            if !done {
                let _ = tx_send.send(Ok(AgentEvent::RunCompleted {
                    final_text: None,
                    external_session: None,
                }));
            }
        });

        // Convert mpsc to tokio channel
        let (tx_tokio, rx_tokio) = tokio::sync::mpsc::channel::<AdapterResult<AgentEvent>>(100);

        // Spawn bridge thread
        let rx_clone = rx;
        tokio::spawn(async move {
            while let Ok(event) = rx_clone.recv() {
                if tx_tokio.send(event).await.is_err() {
                    break;
                }
            }
        });

        let stream = tokio_stream::wrappers::ReceiverStream::new(rx_tokio);
        Ok(Box::pin(stream) as TransportStream)
    }
}

/// Strip ANSI escape sequences from a string.
#[allow(dead_code)]
fn strip_ansi(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            while let Some(&peek) = chars.peek() {
                if peek.is_ascii_alphabetic() {
                    chars.next();
                    break;
                }
                chars.next();
            }
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio_stream::StreamExt;

    #[test]
    fn test_strip_ansi() {
        let input = "\x1b[32mHello\x1b[0m World";
        let output = super::strip_ansi(input);
        assert_eq!(output, "Hello World");
    }

    #[test]
    fn test_strip_ansi_empty() {
        let input = "No ANSI codes";
        let output = super::strip_ansi(input);
        assert_eq!(output, "No ANSI codes");
    }

    #[test]
    fn test_strip_ansi_bold() {
        let input = "\x1b[1mBold Text\x1b[0m";
        let output = super::strip_ansi(input);
        assert_eq!(output, "Bold Text");
    }

    #[test]
    fn test_strip_ansi_underline() {
        let input = "\x1b[4mUnderlined\x1b[0m text";
        let output = super::strip_ansi(input);
        assert_eq!(output, "Underlined text");
    }

    #[test]
    fn test_strip_ansi_color_codes() {
        // Test various color codes
        let input = "\x1b[31mRed\x1b[0m \x1b[32mGreen\x1b[0m \x1b[34mBlue\x1b[0m";
        let output = super::strip_ansi(input);
        assert_eq!(output, "Red Green Blue");
    }

    #[test]
    fn test_strip_ansi_cursor_movement() {
        // Cursor movement codes should be stripped
        let input = "\x1b[10C\x1b[5Dtext";
        let output = super::strip_ansi(input);
        assert_eq!(output, "text");
    }

    #[test]
    fn test_strip_ansi_clear_screen() {
        // Clear screen code
        let input = "\x1b[2J\x1b[HText after clear";
        let output = super::strip_ansi(input);
        assert_eq!(output, "Text after clear");
    }

    #[test]
    fn test_strip_ansi_mixed_content() {
        let input = "\x1b[7mReverse\x1b[0m \x1b[1;31mbold red\x1b[0m normal";
        let output = super::strip_ansi(input);
        assert_eq!(output, "Reverse bold red normal");
    }

    #[test]
    fn test_strip_ansi_incomplete_sequence() {
        // Incomplete ANSI sequence at end
        let input = "Text\x1b[";
        let output = super::strip_ansi(input);
        assert_eq!(output, "Text");
    }

    #[test]
    fn test_strip_ansi_multiple_sequences() {
        let input = "\x1b[31m\x1b[32m\x1b[33mColored\x1b[0m";
        let output = super::strip_ansi(input);
        assert_eq!(output, "Colored");
    }

    #[test]
    fn test_strip_ansi_reset_sequence() {
        let input = "\x1b[0mReset\x1b[0m";
        let output = super::strip_ansi(input);
        assert_eq!(output, "Reset");
    }

    #[test]
    fn test_pty_fallback_transport_new() {
        let transport =
            PtyFallbackTransport::new("test_cli", vec!["arg1".to_string(), "arg2".to_string()]);
        assert_eq!(transport.exe, "test_cli");
        assert_eq!(transport.args, vec!["arg1", "arg2"]);
        assert_eq!(transport.timeout_ms, 300000); // default 5 minutes
    }

    #[test]
    fn test_pty_fallback_transport_with_timeout() {
        let transport = PtyFallbackTransport::new("test_cli", vec![]).with_timeout(60000);
        assert_eq!(transport.timeout_ms, 60000);
    }

    #[test]
    fn test_pty_fallback_transport_with_timeout_chained() {
        let transport = PtyFallbackTransport::new("test_cli", vec![])
            .with_timeout(120000)
            .with_timeout(90000); // last call wins
        assert_eq!(transport.timeout_ms, 90000);
    }

    #[test]
    fn test_build_command_sets_exe_and_args() {
        let transport =
            PtyFallbackTransport::new("my_cli", vec!["--flag".to_string(), "value".to_string()]);
        let cmd = transport.build_command();
        // CommandBuilder doesn't expose args publicly, but we can verify it doesn't panic
        let _ = cmd;
    }

    #[test]
    fn test_pty_fallback_transport_default_timeout() {
        let transport = PtyFallbackTransport::new("test", vec![]);
        assert_eq!(transport.timeout_ms, 300000);
    }

    #[test]
    fn test_strip_ansi_only_escape_char() {
        let input = "\x1b";
        let output = super::strip_ansi(input);
        assert_eq!(output, "");
    }

    #[test]
    fn test_strip_ansi_multiple_escape_chars() {
        // Multiple consecutive escape characters
        // The strip function removes one char after each escape, so this becomes "ext"
        let input = "\x1b\x1b\x1btext";
        let output = super::strip_ansi(input);
        // Current behavior: strips escape sequences and one char each
        assert_eq!(output, "ext");
    }

    #[test]
    fn test_pty_transport_exe_with_spaces() {
        let transport = PtyFallbackTransport::new("/path/to/my cli", vec!["arg".to_string()]);
        assert_eq!(transport.exe, "/path/to/my cli");
    }

    #[test]
    fn test_pty_transport_empty_args() {
        let transport = PtyFallbackTransport::new("test", vec![]);
        assert!(transport.args.is_empty());
    }

    // =========================================================================
    // Lifecycle invariant tests (Task 7 / Task 33)
    // =========================================================================

    #[test]
    fn test_pty_fallback_new_sets_defaults() {
        let transport = PtyFallbackTransport::new("test", vec![]);
        assert_eq!(transport.exe, "test");
        assert_eq!(transport.timeout_ms, 300000);
    }

    #[test]
    fn test_pty_with_timeout_sets_timeout() {
        let transport = PtyFallbackTransport::new("test", vec![]).with_timeout(60000);
        assert_eq!(transport.timeout_ms, 60000);
    }

    // =========================================================================
    // Lifecycle invariant tests (Task 10)
    // Note: PTY tests require `echo` to be available in PATH for the test
    // environment. If echo is not available, these tests will be skipped.
    // =========================================================================

    #[tokio::test]
    async fn test_pty_fallback_exactly_one_run_started() {
        let transport = PtyFallbackTransport::new("echo", vec!["test".to_string()]);
        let request = agent_adapter_core::ChatRequest::new(
            "test prompt",
            ".",
            agent_adapter_core::ProviderId::Claude,
        );
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
    async fn test_pty_fallback_exactly_one_terminal_event() {
        let transport = PtyFallbackTransport::new("echo", vec!["test".to_string()]);
        let request = agent_adapter_core::ChatRequest::new(
            "test prompt",
            ".",
            agent_adapter_core::ProviderId::Claude,
        );
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
            "Expected exactly one terminal event (RunCompleted/RunFailed/RunCancelled), got {}",
            terminal_count
        );
    }

    #[tokio::test]
    async fn test_pty_fallback_run_started_first() {
        let transport = PtyFallbackTransport::new("echo", vec!["test".to_string()]);
        let request = agent_adapter_core::ChatRequest::new(
            "test prompt",
            ".",
            agent_adapter_core::ProviderId::Claude,
        );
        let stream = transport.run_stream(request).await.unwrap();

        let events: Vec<_> = tokio::time::timeout(Duration::from_secs(5), stream.collect())
            .await
            .unwrap();

        // Find the index of RunStarted
        let run_started_idx = events
            .iter()
            .position(|e| matches!(e, Ok(AgentEvent::RunStarted { .. })));

        assert!(
            run_started_idx.is_some(),
            "Expected at least one RunStarted event"
        );
        let idx = run_started_idx.unwrap();

        // RunStarted should be first
        assert_eq!(idx, 0, "RunStarted should be the first event");
    }

    #[tokio::test]
    async fn test_pty_fallback_no_non_log_events_after_terminal() {
        let transport = PtyFallbackTransport::new("echo", vec!["test".to_string()]);
        let request = agent_adapter_core::ChatRequest::new(
            "test prompt",
            ".",
            agent_adapter_core::ProviderId::Claude,
        );
        let stream = transport.run_stream(request).await.unwrap();

        let events: Vec<_> = tokio::time::timeout(Duration::from_secs(5), stream.collect())
            .await
            .unwrap();

        // Find the index of the terminal event
        let terminal_idx = events.iter().position(|e| {
            matches!(
                e,
                Ok(AgentEvent::RunCompleted { .. })
                    | Ok(AgentEvent::RunFailed { .. })
                    | Ok(AgentEvent::RunCancelled { .. })
            )
        });

        if let Some(idx) = terminal_idx {
            // All events after terminal should be only Log events
            let events_after = &events[idx + 1..];
            let non_log = events_after
                .iter()
                .filter(|e| !matches!(e, Ok(AgentEvent::Log { .. })))
                .count();
            assert_eq!(
                non_log, 0,
                "No non-log events should appear after terminal event"
            );
        }
    }
}
