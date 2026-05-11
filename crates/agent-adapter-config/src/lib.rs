//! Config crate for agent-adapter.

pub mod loader;
pub mod registry;
pub mod types;

pub use loader::{ConfigError, ConfigLoader};
pub use registry::ProviderRegistry;
pub use types::*;
