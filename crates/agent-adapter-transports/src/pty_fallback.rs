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
