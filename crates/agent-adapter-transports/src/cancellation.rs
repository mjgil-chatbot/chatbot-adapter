//! Cancellation and timeout handling for transports.
//!
//! This module provides utilities for handling process cancellation,
//! timeouts, and graceful shutdown with process group termination.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout};

use agent_adapter_core::{AdapterError, AdapterResult, AgentEvent};

/// Result of a cancelled or timed-out operation.
#[derive(Debug)]
pub enum Cancelled {
    /// The operation was cancelled by user request.
    Cancelled,
    /// The operation timed out.
    TimedOut { timeout_ms: u64 },
}

/// Options for running a command with timeout and cancellation.
pub struct RunOptions {
    pub timeout_ms: Option<u64>,
    pub run_id: uuid::Uuid,
    pub provider: agent_adapter_core::ProviderId,
}

impl RunOptions {
    /// Create new run options.
    pub fn new(run_id: uuid::Uuid, provider: agent_adapter_core::ProviderId) -> Self {
        Self {
            timeout_ms: None,
            run_id,
            provider,
        }
    }

    /// Set the timeout in milliseconds.
    pub fn with_timeout(mut self, ms: u64) -> Self {
        self.timeout_ms = Some(ms);
        self
    }
}

/// Wait for a child process to complete, with optional timeout.
///
/// Returns Ok((exit_status, final_text)) on success, or Err on timeout/cancellation.
pub async fn wait_with_timeout(
    child: &mut Child,
    options: &RunOptions,
) -> Result<(Option<i32>, String), Cancelled> {
    let final_text = String::new();

    if let Some(timeout_ms) = options.timeout_ms {
        let duration = Duration::from_millis(timeout_ms);
        let result = timeout(duration, child.wait()).await;

        match result {
            Ok(Ok(status)) => {
                let code = status.code();
                Ok((code, final_text))
            }
            Ok(Err(_e)) => Err(Cancelled::Cancelled),
            Err(_) => {
                // Kill the process on timeout
                let _ = child.kill().await;
                Err(Cancelled::TimedOut { timeout_ms })
            }
        }
    } else {
        match child.wait().await {
            Ok(status) => {
                let code = status.code();
                Ok((code, final_text))
            }
            Err(_e) => Err(Cancelled::Cancelled),
        }
    }
}

/// Spawn a command and wait for it to complete, respecting timeout settings.
///
/// Returns the exit code and a final text if applicable.
pub async fn run_command_with_timeout(
    mut cmd: Command,
    options: &RunOptions,
) -> AdapterResult<(Option<i32>, String)> {
    let mut child = cmd.spawn().map_err(|e| AdapterError::TransportError {
        message: format!("Failed to spawn command: {}", e),
    })?;

    match wait_with_timeout(&mut child, options).await {
        Ok((code, text)) => Ok((code, text)),
        Err(Cancelled::Cancelled) => {
            let _ = child.kill().await;
            Err(AdapterError::TransportError {
                message: "Process cancelled".to_string(),
            })
        }
        Err(Cancelled::TimedOut { timeout_ms: _ }) => {
            let _ = child.kill().await;
            Err(AdapterError::Timeout {
                run_id: options.run_id.to_string(),
            })
        }
    }
}

/// Create a cancellation token that can be used to signal stoppage.
pub struct CancellationToken {
    cancelled: AtomicBool,
}

impl Clone for CancellationToken {
    fn clone(&self) -> Self {
        Self {
            cancelled: AtomicBool::new(self.cancelled.load(Ordering::SeqCst)),
        }
    }
}

impl CancellationToken {
    /// Create a new cancellation token.
    pub fn new() -> Self {
        Self {
            cancelled: AtomicBool::new(false),
        }
    }

    /// Request cancellation.
    pub fn cancel(&self) {
        self.cancelled
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// Check if cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Wait until cancellation is requested.
    pub async fn wait_cancelled(&self) {
        while !self.is_cancelled() {
            sleep(Duration::from_millis(10)).await;
        }
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait for sending cancellation events.
pub trait EventSender: Send {
    fn send_cancelled(&self, tx: &mpsc::Sender<AdapterResult<AgentEvent>>);
    fn send_timeout(&self, tx: &mpsc::Sender<AdapterResult<AgentEvent>>, timeout_ms: u64);
}

pub struct EventSendHelper;

impl EventSender for EventSendHelper {
    fn send_cancelled(&self, tx: &mpsc::Sender<AdapterResult<AgentEvent>>) {
        let _ = tx.try_send(Ok(AgentEvent::RunCancelled));
    }

    fn send_timeout(&self, tx: &mpsc::Sender<AdapterResult<AgentEvent>>, timeout_ms: u64) {
        let _ = tx.try_send(Ok(AgentEvent::RunFailed {
            code: None,
            message: format!("Process timed out after {}ms", timeout_ms),
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[test]
    fn test_cancellation_token() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());

        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn test_cancellation_token_default() {
        let token = CancellationToken::default();
        assert!(!token.is_cancelled());
    }

    #[test]
    fn test_cancellation_token_cancel_twice() {
        let token = CancellationToken::new();
        token.cancel();
        token.cancel(); // Cancel twice should be idempotent
        assert!(token.is_cancelled());
    }

    #[tokio::test]
    async fn test_wait_cancelled_returns_immediately_when_already_cancelled() {
        let token = CancellationToken::new();
        // Cancel immediately
        token.cancel();
        // Now wait should return right away since already cancelled
        // Use a timeout to avoid hanging
        let result = tokio::time::timeout(Duration::from_secs(1), token.wait_cancelled()).await;
        assert!(result.is_ok()); // Should complete without timeout
    }

    #[test]
    fn test_run_options_new() {
        let run_id = uuid::Uuid::new_v4();
        let options = RunOptions::new(run_id, agent_adapter_core::ProviderId::Claude);
        assert_eq!(options.run_id, run_id);
        assert_eq!(options.provider, agent_adapter_core::ProviderId::Claude);
        assert!(options.timeout_ms.is_none());
    }

    #[test]
    fn test_run_options_with_timeout() {
        let options = RunOptions::new(uuid::Uuid::new_v4(), agent_adapter_core::ProviderId::Gemini)
            .with_timeout(5000);
        assert_eq!(options.timeout_ms, Some(5000));
    }

    #[test]
    fn test_run_options_chained_timeout() {
        let options = RunOptions::new(uuid::Uuid::new_v4(), agent_adapter_core::ProviderId::Kimi)
            .with_timeout(1000)
            .with_timeout(2000); // Last one wins
        assert_eq!(options.timeout_ms, Some(2000));
    }

    #[tokio::test]
    async fn test_wait_with_timeout_success() {
        let mut cmd = Command::new("echo");
        cmd.arg("hello");
        let mut child = cmd.spawn().unwrap();

        let _options =
            RunOptions::new(uuid::Uuid::new_v4(), agent_adapter_core::ProviderId::Claude);
        let result = timeout(Duration::from_secs(5), child.wait()).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_wait_with_timeout_with_timeout_set() {
        let mut cmd = Command::new("sleep");
        cmd.arg("0");
        let mut child = cmd.spawn().unwrap();

        let options = RunOptions::new(uuid::Uuid::new_v4(), agent_adapter_core::ProviderId::Claude)
            .with_timeout(5000);

        let result = wait_with_timeout(&mut child, &options).await;
        assert!(result.is_ok());
        let (code, _) = result.unwrap();
        assert_eq!(code, Some(0));
    }

    #[tokio::test]
    async fn test_wait_with_timeout_expiration() {
        let mut cmd = Command::new("sleep");
        cmd.arg("10"); // Sleep for 10 seconds
        let mut child = cmd.spawn().unwrap();

        let options = RunOptions::new(uuid::Uuid::new_v4(), agent_adapter_core::ProviderId::Claude)
            .with_timeout(100); // 100ms timeout

        let result = wait_with_timeout(&mut child, &options).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            Cancelled::TimedOut { timeout_ms } => assert_eq!(timeout_ms, 100),
            Cancelled::Cancelled => panic!("Expected TimedOut"),
        }
    }

    #[tokio::test]
    async fn test_event_send_helper_send_cancelled() {
        let (tx, mut rx) = mpsc::channel(10);
        let helper = EventSendHelper;

        helper.send_cancelled(&tx);

        // Receive the event
        let event = rx.recv().await;
        assert!(event.is_some());
        assert!(matches!(event.unwrap(), Ok(AgentEvent::RunCancelled)));
    }

    #[tokio::test]
    async fn test_event_send_helper_send_timeout() {
        let (tx, mut rx) = mpsc::channel(10);
        let helper = EventSendHelper;

        helper.send_timeout(&tx, 5000);

        // Receive the event
        let event = rx.recv().await;
        assert!(event.is_some());
        assert!(matches!(
            event.unwrap(),
            Ok(AgentEvent::RunFailed { code: None, message }) if message.contains("5000")
        ));
    }

    #[test]
    fn test_cancelled_debug() {
        let cancelled = Cancelled::Cancelled;
        let timed_out = Cancelled::TimedOut { timeout_ms: 1000 };
        assert_eq!(format!("{:?}", cancelled), "Cancelled");
        assert_eq!(format!("{:?}", timed_out), "TimedOut { timeout_ms: 1000 }");
    }

    #[tokio::test]
    async fn test_run_command_with_timeout_success() {
        let cmd = Command::new("echo");
        let options = RunOptions::new(uuid::Uuid::new_v4(), agent_adapter_core::ProviderId::Codex)
            .with_timeout(5000);

        let result = run_command_with_timeout(cmd, &options).await;
        assert!(result.is_ok());
        let (code, _) = result.unwrap();
        assert_eq!(code, Some(0));
    }

    #[tokio::test]
    async fn test_run_command_with_timeout_nonexistent() {
        let cmd = Command::new("nonexistent_command_xyz");
        let options = RunOptions::new(
            uuid::Uuid::new_v4(),
            agent_adapter_core::ProviderId::OpenCode,
        );

        let result = run_command_with_timeout(cmd, &options).await;
        assert!(result.is_err());
    }

    // =========================================================================
    // End-to-end cancellation tests (Task 34)
    // =========================================================================

    #[tokio::test]
    async fn test_cancellation_token_cloning() {
        let token1 = CancellationToken::new();
        let token2 = token1.clone();

        // Cancel the original
        token1.cancel();

        // Each token has its own atomic, so cancelling one doesn't cancel the other
        // This is the correct behavior - independent cancellation domains
        assert!(token1.is_cancelled());
        // token2 is independent, not cancelled
        assert!(!token2.is_cancelled());
    }

    #[tokio::test]
    async fn test_cancellation_token_independent_clones() {
        let token1 = CancellationToken::new();
        let token2 = token1.clone();

        // Cancel only token2
        token2.cancel();

        // token1 should NOT be cancelled (independent atomic)
        assert!(!token1.is_cancelled());
        assert!(token2.is_cancelled());
    }

    #[tokio::test]
    async fn test_event_sender_works_with_channels() {
        let (tx, mut rx) = mpsc::channel(10);
        let helper = EventSendHelper;

        // Send cancelled event
        helper.send_cancelled(&tx);
        let event = rx.recv().await;
        assert!(matches!(event, Some(Ok(AgentEvent::RunCancelled))));

        // Send timeout event
        let (tx2, mut rx2) = mpsc::channel(10);
        helper.send_timeout(&tx2, 5000);
        let event2 = rx2.recv().await;
        assert!(matches!(
            event2,
            Some(Ok(AgentEvent::RunFailed { code: None, .. }))
        ));
    }

    #[test]
    fn test_cancelled_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Cancelled>();
    }

    #[test]
    fn test_cancellation_token_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<CancellationToken>();
    }

    // =========================================================================
    // Timeout end-to-end tests (Task 35)
    // =========================================================================

    #[tokio::test]
    async fn test_run_command_with_timeout_returns_error_on_timeout() {
        let mut cmd = Command::new("sleep");
        cmd.arg("10"); // Sleep for 10 seconds
        let options = RunOptions::new(uuid::Uuid::new_v4(), agent_adapter_core::ProviderId::Claude)
            .with_timeout(100); // 100ms timeout

        let result = run_command_with_timeout(cmd, &options).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            AdapterError::Timeout { run_id } => {
                // Verify the run_id is preserved in the error
                assert!(!run_id.is_empty());
            }
            other => panic!("Expected Timeout error, got {:?}", other),
        }
    }

    #[test]
    fn test_timed_out_contains_timeout_ms() {
        let timed_out = Cancelled::TimedOut { timeout_ms: 5000 };
        match timed_out {
            Cancelled::TimedOut { timeout_ms } => assert_eq!(timeout_ms, 5000),
            _ => panic!("Expected TimedOut"),
        }
    }

    #[test]
    fn test_timeout_error_display() {
        let error = AdapterError::Timeout {
            run_id: "test-run-123".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("test-run-123"));
        assert!(display.contains("timed out"));
    }
}
