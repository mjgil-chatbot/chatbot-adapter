use super::*;
use agent_adapter_core::{ChatRequest, ProviderId};
use std::time::Duration;
use tokio_stream::StreamExt;
fn test_request() -> ChatRequest {
    ChatRequest::new("test prompt", ".", ProviderId::Claude)
}
