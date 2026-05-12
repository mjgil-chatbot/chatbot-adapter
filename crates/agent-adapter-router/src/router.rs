//! Agent router.

use agent_adapter_core::{
    AdapterError, AdapterResult, AgentAdapter, ApprovalPolicy, ChatRequest, EventStream,
    ProviderId, ResumeMode,
};
use std::collections::HashMap;

pub struct AgentRouter {
    adapters: HashMap<ProviderId, Box<dyn AgentAdapter>>,
}

impl AgentRouter {
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
        }
    }

    pub fn register(&mut self, adapter: Box<dyn AgentAdapter>) {
        self.adapters.insert(adapter.id(), adapter);
    }

    /// Check if a provider supports the requested capabilities.
    fn check_capabilities(
        &self,
        provider: &ProviderId,
        request: &ChatRequest,
    ) -> AdapterResult<()> {
        let adapter =
            self.adapters
                .get(provider)
                .ok_or_else(|| AdapterError::ProviderNotFound {
                    provider: request.provider.clone(),
                    install_hint: None,
                })?;

        let caps = adapter.capabilities();

        // Check if request needs images but provider doesn't support them
        if !request.images.is_empty() && !caps.images {
            return Err(AdapterError::UnsupportedCapability {
                capability: "images".to_string(),
                provider: provider.clone(),
            });
        }

        // Check if request needs tools but provider doesn't support them
        // DangerousDefault always needs tools (dangerous mode), Safe doesn't (safe mode)
        let needs_tools = request.approval_policy == ApprovalPolicy::DangerousDefault;
        if needs_tools && !caps.tools {
            return Err(AdapterError::UnsupportedCapability {
                capability: "tools".to_string(),
                provider: provider.clone(),
            });
        }

        // Check if request needs sessions but provider doesn't support them
        // Any non-default resume mode requires session support
        let needs_session = !matches!(
            request.resume,
            ResumeMode::None | ResumeMode::ProviderDefault
        );
        if needs_session && !caps.sessions {
            return Err(AdapterError::UnsupportedCapability {
                capability: "sessions".to_string(),
                provider: provider.clone(),
            });
        }

        Ok(())
    }

    pub async fn run(&self, request: ChatRequest) -> AdapterResult<EventStream> {
        // Check provider exists
        if !self.adapters.contains_key(&request.provider) {
            return Err(AdapterError::ProviderNotFound {
                provider: request.provider.clone(),
                install_hint: None,
            });
        }

        // Check capability compatibility
        self.check_capabilities(&request.provider, &request)?;

        let adapter = self.adapters.get(&request.provider).unwrap();
        adapter.run(request).await
    }

    pub fn get_adapter(&self, provider: &ProviderId) -> Option<&dyn AgentAdapter> {
        self.adapters.get(provider).map(|b| b.as_ref())
    }
}

impl Default for AgentRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_register() {
        // Will be expanded when providers are implemented
        let router = AgentRouter::new();
        assert!(router.adapters.is_empty());
    }
}
