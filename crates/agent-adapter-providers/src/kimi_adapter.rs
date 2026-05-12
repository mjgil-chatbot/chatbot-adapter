//! Kimi provider adapter.

use async_trait::async_trait;

use agent_adapter_core::{
    apply_prompt_policy, AdapterCapabilities, AdapterHealth, AdapterResult, AgentAdapter,
    ChatRequest, EventStream, ProviderId,
};
use agent_adapter_transports::JsonlCommandTransport;

use crate::config::ProviderConfig;
use crate::{build_command, BuildError, CommandSpec};

pub struct KimiAdapter {
    config: ProviderConfig,
    capabilities: AdapterCapabilities,
}

impl KimiAdapter {
    pub fn new(config: ProviderConfig) -> Self {
        let capabilities = AdapterCapabilities {
            sessions: true,
            tools: true,
            jsonl: true,
            acp: true,
            ..Default::default()
        };
        Self {
            config,
            capabilities,
        }
    }

    fn build_spec(&self, request: &ChatRequest) -> Result<CommandSpec, BuildError> {
        // Kimi safe mode returns error for --print mode (AFK behavior)
        if request.approval_policy.is_safe() {
            return Err(BuildError {
                message: "Kimi safe mode not fully supported for --print mode (implies AFK)"
                    .to_string(),
            });
        }

        build_command(
            ProviderId::Kimi,
            request.approval_policy,
            &self.config,
            request,
        )
    }

    async fn run_transport(&self, request: ChatRequest) -> AdapterResult<EventStream> {
        let request = apply_prompt_policy(&request);
        let spec = self
            .build_spec(&request)
            .map_err(|e| agent_adapter_core::AdapterError::InvalidConfig { message: e.message })?;
        let transport = JsonlCommandTransport::new_with_env(spec.exe, spec.args, spec.env);
        transport.run_stream(request).await
    }
}

#[async_trait]
impl AgentAdapter for KimiAdapter {
    fn id(&self) -> ProviderId {
        ProviderId::Kimi
    }

    fn capabilities(&self) -> AdapterCapabilities {
        self.capabilities.clone()
    }

    async fn healthcheck(&self) -> AdapterResult<AdapterHealth> {
        crate::check_binary(ProviderId::Kimi, &self.config.exe).await
    }

    async fn run(&self, request: ChatRequest) -> AdapterResult<EventStream> {
        self.run_transport(request).await
    }

    async fn cancel(&self, _run_id: uuid::Uuid) -> AdapterResult<()> {
        Ok(())
    }
}
