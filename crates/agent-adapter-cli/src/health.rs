//! Healthcheck command implementation.

use agent_adapter_config::ConfigLoader;
use agent_adapter_core::AdapterHealth;
use agent_adapter_providers::{FactoryRegistry, TransportKind};
use anyhow::Context;
use serde::Serialize;

use super::HealthcheckArgs;

#[derive(Serialize)]
struct HealthcheckOutput {
    provider: String,
    status: String,
    version: Option<String>,
    detail: String,
    transports: Vec<String>,
    safe_mode_supported: bool,
    unsafe_mode_supported: bool,
}

pub(crate) fn healthcheck_command(args: HealthcheckArgs) -> anyhow::Result<()> {
    let provider_name = args.provider.to_lowercase();
    let json_output = args.json;

    // Build config loader
    let loader = ConfigLoader::new();

    // Load configs (will use built-in defaults if no config file)
    let configs = loader.load().context("Failed to load provider config")?;
    tracing::debug!(providers = ?configs.iter().map(|c| c.name.clone()).collect::<Vec<_>>(), "Loaded provider configs");

    // Find config for the requested provider
    let config = configs
        .iter()
        .find(|c| c.name.to_lowercase() == provider_name)
        .ok_or_else(|| anyhow::anyhow!("Provider '{}' not found in config", provider_name))?;

    // Use registry to detect health status directly without building adapter
    let rt = tokio::runtime::Runtime::new()?;

    let health: AdapterHealth = rt.block_on(async {
        let registry = FactoryRegistry::new();
        if let Some(factory) = registry.get(&config.name) {
            factory
                .detect()
                .await
                .unwrap_or_else(|_| AdapterHealth::Misconfigured {
                    message: "Health check failed".to_string(),
                })
        } else {
            AdapterHealth::Misconfigured {
                message: "Factory not found".to_string(),
            }
        }
    });

    let (transports, safe_mode, unsafe_mode) = {
        let registry = FactoryRegistry::new();
        if let Some(factory) = registry.get(&config.name) {
            let t = factory.supported_transports();
            let p = factory.policy_support();
            (t, p.supports_safe, p.supports_dangerous)
        } else {
            (vec![], false, false)
        }
    };

    let transport_names: Vec<String> = transports
        .iter()
        .map(|t| match t {
            TransportKind::JsonlCommand => "jsonl".to_string(),
            TransportKind::OneShotCommand => "oneshot".to_string(),
            TransportKind::AcpStdio => "acp".to_string(),
            TransportKind::PtyFallback => "pty".to_string(),
        })
        .collect();

    if json_output {
        let (status, version, detail) = match &health {
            AdapterHealth::Ready { version } => (
                "ready".to_string(),
                version.clone(),
                "Provider binary is available".to_string(),
            ),
            AdapterHealth::NeedsAuth { login_hint } => {
                ("needs_auth".to_string(), None, login_hint.clone())
            }
            AdapterHealth::MissingBinary { install_hint } => {
                ("missing".to_string(), None, install_hint.clone())
            }
            AdapterHealth::Misconfigured { message } => {
                ("misconfigured".to_string(), None, message.clone())
            }
            AdapterHealth::Unsupported { reason } => {
                ("unsupported".to_string(), None, reason.clone())
            }
        };
        let output = HealthcheckOutput {
            provider: provider_name,
            status,
            version,
            detail,
            transports: transport_names,
            safe_mode_supported: safe_mode,
            unsafe_mode_supported: unsafe_mode,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        match &health {
            AdapterHealth::Ready { version } => {
                println!("Provider '{}' is ready", provider_name);
                if let Some(v) = version {
                    println!("Version: {}", v);
                }
                println!("Transports: {}", transport_names.join(", "));
                println!("Safe mode: supported={}", safe_mode);
                println!("Unsafe mode: supported={}", unsafe_mode);
            }
            AdapterHealth::MissingBinary { install_hint } => {
                println!("Provider '{}' is not installed", provider_name);
                println!("Install hint: {}", install_hint);
                println!("Transports: {}", transport_names.join(", "));
                println!("Safe mode: supported={}", safe_mode);
                println!("Unsafe mode: supported={}", unsafe_mode);
            }
            AdapterHealth::NeedsAuth { login_hint } => {
                println!("Provider '{}' needs authentication", provider_name);
                println!("Login hint: {}", login_hint);
                println!("Transports: {}", transport_names.join(", "));
                println!("Safe mode: supported={}", safe_mode);
                println!("Unsafe mode: supported={}", unsafe_mode);
            }
            AdapterHealth::Misconfigured { message } => {
                println!("Provider '{}' is misconfigured", provider_name);
                println!("Message: {}", message);
                println!("Transports: {}", transport_names.join(", "));
                println!("Safe mode: supported={}", safe_mode);
                println!("Unsafe mode: supported={}", unsafe_mode);
            }
            AdapterHealth::Unsupported { reason } => {
                println!("Provider '{}' is unsupported", provider_name);
                println!("Reason: {}", reason);
                println!("Transports: {}", transport_names.join(", "));
                println!("Safe mode: supported={}", safe_mode);
                println!("Unsafe mode: supported={}", unsafe_mode);
            }
        }
    }

    Ok(())
}
