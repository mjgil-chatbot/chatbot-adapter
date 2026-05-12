//! One-shot command transport implementation.
//!
//! This transport spawns a command, collects stdout as final text,
//! maps stderr to logs, and returns completion/failure by exit status.

use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use uuid::Uuid;
/// One-shot command transport.
pub struct OneShotCommandTransport {
    exe: String,
    args: Vec<String>,
}
