//! Providers command implementation.

use agent_adapter_config::ConfigLoader;
use agent_adapter_core::{AdapterCapabilities, AdapterHealth};
use agent_adapter_providers::{FactoryRegistry, PolicySupport, TransportKind};
use anyhow::Context;
use serde::Serialize;

use super::ProvidersArgs;

#[derive(Serialize)]
struct ProviderInfo {
    name: String,
    display_name: String,
    docs_path: Option<String>,
    installed: bool,
    version: Option<String>,
    safe_mode_supported: bool,
    unsafe_mode_supported: bool,
    requires_explicit_unsafe_args: bool,
    supported_transports: Vec<String>,
    capabilities: AdapterCapabilities,
    health_status: String,
    health_detail: String,
}

#[derive(Serialize)]
struct ProvidersOutput {
    providers: Vec<ProviderInfo>,
    total: usize,
    builtin_count: usize,
    custom_count: usize,
}

pub(crate) fn providers_command(args: ProvidersArgs) -> anyhow::Result<()> {
    let json_output = args.json;

    // Build config loader
    let loader = ConfigLoader::new();

    // Load configs (will use built-in defaults if no config file)
    let configs = loader.load().context("Failed to load provider config")?;
    tracing::debug!(providers = ?configs.iter().map(|c| c.name.clone()).collect::<Vec<_>>(), "Loaded provider configs");

    // Get factory registry
    let registry = FactoryRegistry::new();
    let rt = tokio::runtime::Runtime::new()?;

    // Get all factory names from registry
    let provider_names = registry.provider_names();
    let builtin_count = provider_names.len();

    // Clone for custom provider check after iteration
    let provider_names_clone = provider_names.clone();

    let mut provider_infos = Vec::new();

    for provider_name in provider_names {
        // Get factory from registry
        let factory = registry.get(&provider_name);

        // Get health status via factory
        let health = rt
            .block_on(async {
                if let Some(f) = factory.as_ref() {
                    f.detect().await
                } else {
                    Ok(AdapterHealth::Misconfigured {
                        message: "Factory not found".to_string(),
                    })
                }
            })
            .unwrap_or(AdapterHealth::Misconfigured {
                message: "Health check failed".to_string(),
            });

        // Get docs path and policy support from factory
        let (docs_path, policy_support, supported_transports) = if let Some(f) = factory.as_ref() {
            let docs = f.docs_path();
            let docs_str = docs.to_str().map(String::from);
            let policy = f.policy_support();
            let transports = f.supported_transports();
            (docs_str, policy, transports)
        } else {
            (None, PolicySupport::neither(), vec![])
        };

        // Get capabilities by building adapter
        let (caps, installed, version, health_status_str, health_detail_str) = if let Some(config) =
            configs.iter().find(|c| c.name == provider_name)
        {
            let result = rt.block_on(registry.build_adapter(&provider_name, config.clone()));
            if let Ok(adapter) = result {
                let caps = adapter.capabilities();
                let (version, detail) = match &health {
                    AdapterHealth::Ready { version } => (version.clone(), "ready".to_string()),
                    AdapterHealth::NeedsAuth { login_hint } => (None, login_hint.clone()),
                    AdapterHealth::MissingBinary { install_hint } => (None, install_hint.clone()),
                    AdapterHealth::Misconfigured { message } => (None, message.clone()),
                    AdapterHealth::Unsupported { reason } => (None, reason.clone()),
                };
                (caps, true, version, "ready".to_string(), detail)
            } else {
                (
                    AdapterCapabilities::default(),
                    false,
                    None,
                    "build_failed".to_string(),
                    "Adapter build failed".to_string(),
                )
            }
        } else {
            (
                AdapterCapabilities::default(),
                false,
                None,
                "not_configured".to_string(),
                "Not in config".to_string(),
            )
        };

        let transport_names: Vec<String> = supported_transports
            .iter()
            .map(|t| match t {
                TransportKind::JsonlCommand => "jsonl".to_string(),
                TransportKind::OneShotCommand => "oneshot".to_string(),
                TransportKind::AcpStdio => "acp".to_string(),
                TransportKind::PtyFallback => "pty".to_string(),
            })
            .collect();

        let info = ProviderInfo {
            name: provider_name.clone(),
            display_name: provider_metadata_display(&provider_name),
            docs_path,
            installed,
            version,
            safe_mode_supported: policy_support.supports_safe,
            unsafe_mode_supported: policy_support.supports_dangerous,
            requires_explicit_unsafe_args: policy_support.requires_explicit_unsafe_args,
            supported_transports: if transport_names.is_empty() {
                vec!["unknown".to_string()]
            } else {
                transport_names
            },
            capabilities: caps,
            health_status: health_status_str,
            health_detail: health_detail_str,
        };
        provider_infos.push(info);
    }

    // Add custom providers from config that aren't in registry
    let builtin_names: std::collections::HashSet<_> = provider_names_clone.into_iter().collect();
    let mut custom_count = 0;
    for config in &configs {
        if !builtin_names.contains(&config.name) {
            custom_count += 1;
            let info = ProviderInfo {
                name: config.name.clone(),
                display_name: config.name.clone(),
                docs_path: None,
                installed: false,
                version: None,
                safe_mode_supported: false,
                unsafe_mode_supported: false,
                requires_explicit_unsafe_args: false,
                supported_transports: vec![],
                capabilities: AdapterCapabilities::default(),
                health_status: "custom".to_string(),
                health_detail: "Custom provider in config".to_string(),
            };
            provider_infos.push(info);
        }
    }

    if json_output {
        let total = provider_infos.len();
        let output = ProvidersOutput {
            providers: provider_infos,
            total,
            builtin_count,
            custom_count,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("Available providers:");
        println!();

        for info in &provider_infos {
            println!(
                "  {:12} - {} (transports: {})",
                info.name,
                info.display_name,
                info.supported_transports.join(", ")
            );
            if let Some(docs) = &info.docs_path {
                println!("              Docs: {}", docs);
            }
            if info.installed {
                println!(
                    "              Status: {} | Safe: {} | Unsafe: {} | Requires unsafe args: {}",
                    info.health_status,
                    info.safe_mode_supported,
                    info.unsafe_mode_supported,
                    info.requires_explicit_unsafe_args
                );
                if let Some(v) = &info.version {
                    println!("              Version: {}", v);
                }
            } else {
                println!("              [Not installed or not configured]");
            }
            println!();
        }

        println!("Use 'agent-adapter doctor' for detailed diagnostics.");
        println!("Use 'agent-adapter healthcheck --provider <name>' to check a specific provider.");
    }

    Ok(())
}

/// Get display name for a provider from registry metadata.
fn provider_metadata_display(name: &str) -> String {
    let metadata = [
        ("claude", "Anthropic Claude Code CLI"),
        ("codex", "OpenAI Codex CLI"),
        ("gemini", "Google Gemini CLI"),
        ("kimi", "Moonshot Kimi CLI"),
        ("opencode", "OpenCode CLI"),
        ("hermes", "Hermes Agent"),
        ("forge", "ForgeCode CLI"),
        ("claw", "Claw Code CLI"),
    ];
    metadata
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, d)| d)
        .unwrap_or(&name)
        .to_string()
}
