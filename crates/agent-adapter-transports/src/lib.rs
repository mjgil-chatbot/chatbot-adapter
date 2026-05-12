//! Transport crate for agent-adapter.

pub mod acp_stdio;
pub mod traits;
pub use traits::{Transport, TransportStream};
