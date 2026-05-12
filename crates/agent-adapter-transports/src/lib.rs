//! Transport crate for agent-adapter.

pub mod acp_stdio;
pub mod cancellation;
pub mod jsonl_command;
pub mod traits;
pub use traits::{Transport, TransportStream};
