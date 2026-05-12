//! Transport crate for agent-adapter.

pub mod acp_stdio;
pub mod cancellation;
pub mod jsonl_command;
pub mod oneshot_command;
pub mod pty_fallback;
pub mod traits;

pub use acp_stdio::AcpStdioTransport;
pub use cancellation::{
    run_command_with_timeout, wait_with_timeout, CancellationToken, RunOptions,
};
pub use jsonl_command::JsonlCommandTransport;
pub use oneshot_command::OneShotCommandTransport;
pub use pty_fallback::PtyFallbackTransport;
pub use traits::{Transport, TransportStream};
