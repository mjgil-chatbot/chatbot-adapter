//! Run command implementation.

use crate::{
    create_request, print_events, ApprovalPolicy, DANGEROUS_DEFAULT_ACTIVE, SAFE_MODE_ACTIVE,
};
use agent_adapter_config::ConfigLoader;
use agent_adapter_core::ProviderId;
use agent_adapter_providers::FactoryRegistry;
use agent_adapter_router::AgentRouter;

pub(crate) fn run_command(args: super::RunArgs) -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async_run(args))
}

async fn async_run(args: super::RunArgs) -> anyhow::Result<()> {
    // Build config loader based on CLI flags
    let mut loader = ConfigLoader::new();

    if let Some(ref config_path) = args.config {
        // Explicit config path - skip discovery
        loader = loader.with_explicit_path(config_path);
        tracing::info!(path = %config_path, "Using explicit config path");
    }

    if args.no_env {
        loader = loader.with_env_overrides(false);
        tracing::info!("Environment variable overrides disabled");
    }

    // Load configs to get provider list
    let configs = loader.load()?;
    tracing::debug!(providers = ?configs.iter().map(|c| c.name.clone()).collect::<Vec<_>>(), "Loaded provider configs");

    let request = create_request(&args)?;
    let policy = if args.safe {
        ApprovalPolicy::Safe
    } else {
        ApprovalPolicy::DangerousDefault
    };

    // Log the startup mode
    if policy.is_safe() {
        tracing::info!(mode = %SAFE_MODE_ACTIVE, "Safe mode active");
    } else {
        tracing::info!(mode = %DANGEROUS_DEFAULT_ACTIVE, "Dangerous default mode active");
    }

    // Build router with adapters from config using FactoryRegistry
    let registry = FactoryRegistry::new();
    let mut router = AgentRouter::new();
    let registered_providers: Vec<String> = configs.iter().map(|c| c.name.clone()).collect();

    for config in &configs {
        match registry.build_adapter(&config.name, config.clone()).await {
            Ok(adapter) => {
                router.register(adapter);
                tracing::debug!(provider = %config.name, "Registered adapter from config");
            }
            Err(e) => {
                tracing::warn!(provider = %config.name, error = %e, "Failed to build adapter - skipping");
            }
        }
    }

    // Verify the requested provider is registered
    let requested_provider_id = match request.provider.clone() {
        ProviderId::Custom(ref name) => name.clone(),
        other => format!("{:?}", other).to_lowercase(),
    };

    if !registered_providers.contains(&requested_provider_id) {
        anyhow::bail!(
            "Provider '{}' not found in config. Available providers: {:?}. Use --config to specify config file.",
            requested_provider_id,
            registered_providers
        );
    }

    // Note: RunStarted event is emitted by the transport layer, not here.
    // This ensures exactly one RunStarted per run (owned by the transport).
    // The transport has full context of when the process actually started.

    let stream = router.run(request).await?;

    print_events(stream).await?;

    Ok(())
}
