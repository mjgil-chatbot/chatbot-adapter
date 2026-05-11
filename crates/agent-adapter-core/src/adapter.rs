//! Adapter trait for agent providers.

use async_trait::async_trait;
use futures_core::Stream;
use std::pin::Pin;

use crate::capabilities::{AdapterCapabilities, AdapterHealth};
use crate::errors::AdapterResult;
use crate::events::AgentEvent;
use crate::request::ChatRequest;
use crate::types::ProviderId;

/// Event stream type.
pub type EventStream = Pin<Box<dyn Stream<Item = AdapterResult<AgentEvent>> + Send + 'static>>;

/// Core trait for agent adapters.
///
/// Implement this trait to add support for a new agent provider.
#[async_trait]
pub trait AgentAdapter: Send + Sync {
    /// Returns the provider ID for this adapter.
    fn id(&self) -> ProviderId;

    /// Returns the capabilities of this adapter.
    fn capabilities(&self) -> AdapterCapabilities;

    /// Performs a health check on the adapter.
    async fn healthcheck(&self) -> AdapterResult<AdapterHealth>;

    /// Runs the agent with the given request.
    async fn run(&self, request: ChatRequest) -> AdapterResult<EventStream>;

    /// Cancels a running agent.
    async fn cancel(&self, run_id: uuid::Uuid) -> AdapterResult<()>;
}

/// Trait for mapping raw output to events.
pub trait EventMapper: Send {
    /// Maps a line from stdout to events.
    fn map_stdout_line(&mut self, line: &str) -> Vec<AdapterResult<AgentEvent>>;

    /// Maps a line from stderr to events.
    fn map_stderr_line(&mut self, line: &str) -> Vec<AdapterResult<AgentEvent>>;

    /// Called when the process finishes.
    fn finish(&mut self, exit_code: Option<i32>) -> Vec<AdapterResult<AgentEvent>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_mapper_trait_exists() {
        // Verify the trait is properly defined.
        fn _check<M: EventMapper>() {}
    }
}
