//! Transport traits.

use futures_core::Stream;
use std::pin::Pin;

use agent_adapter_core::{AdapterResult, AgentEvent, ChatRequest};

pub type TransportStream = Pin<Box<dyn Stream<Item = AdapterResult<AgentEvent>> + Send + 'static>>;

pub trait Transport: Send + Sync {
    fn run(
        &self,
        request: ChatRequest,
    ) -> impl std::future::Future<Output = AdapterResult<TransportStream>> + Send;
    fn cancel(
        &self,
        run_id: uuid::Uuid,
    ) -> impl std::future::Future<Output = AdapterResult<()>> + Send;
}
