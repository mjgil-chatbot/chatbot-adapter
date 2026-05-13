//! Doctor command implementation.

use agent_adapter_config::ConfigLoader;
use agent_adapter_core::{AdapterCapabilities, AdapterHealth};
use agent_adapter_providers::FactoryRegistry;
use anyhow::Context;
use serde::Serialize;
use std::borrow::Cow;

#[derive(Serialize)]
struct DoctorOutput<'a> {
    config_path: Cow<'a, str>,
    providers: Vec<ProviderStatus>,
    env_allowlist_status: String,
}

#[derive(Serialize)]
struct ProviderStatus {
    provider: String,
    installed: bool,
    version: Option<String>,
    safe_mode_supported: bool,
    unsafe_mode_supported: bool,
    capabilities: AdapterCapabilities,
    supported_transports: Vec<String>,
}

pub(crate) fn doctor_command(args: super::DoctorArgs) -> anyhow::Result<()> {
    let json_output = args.json;

    // Build config loader
    let loader = ConfigLoader::new();

    // Load configs (will use built-in defaults if no config file)
    let configs = loader.load().context("Failed to load provider config")?;
    tracing::debug!(providers = ?configs.iter().map(|c| c.name.clone()).collect::<Vec<_>>(), "Loaded provider configs");

    // Get factory registry
    let registry = FactoryRegistry::new();
    let rt = tokio::runtime::Runtime::new()?;

    // Check all known built-in providers via registry
    let provider_names = registry.provider_names();
    let mut provider_statuses = Vec::new();

    for provider_name in provider_names {
        // Get factory health via registry
        let health = rt
            .block_on(async {
                if let Some(factory) = registry.get(&provider_name) {
                    factory.detect().await
                } else {
                    Ok(AdapterHealth::Misconfigured {
                        message: "Factory not found".to_string(),
                    })
                }
            })
            .unwrap_or(AdapterHealth::Misconfigured {
                message: "Health check failed".to_string(),
            });

        // Get capabilities by building the adapter
        let (caps, supported_transports) =
            if let Some(config) = configs.iter().find(|c| c.name == provider_name) {
                let result = rt.block_on(registry.build_adapter(&provider_name, config.clone()));
                if let Ok(adapter) = result {
                    let caps = adapter.capabilities();
                    let transports = get_supported_transports(&caps);
                    (caps, transports)
                } else {
                    (AdapterCapabilities::default(), vec![])
                }
            } else {
                (AdapterCapabilities::default(), vec![])
            };

        let status = match &health {
            AdapterHealth::Ready { version } => ProviderStatus {
                provider: provider_name.clone(),
                installed: true,
                version: version.clone(),
                safe_mode_supported: caps.permissions,
                unsafe_mode_supported: true,
                capabilities: caps,
                supported_transports,
            },
            AdapterHealth::MissingBinary { .. } => ProviderStatus {
                provider: provider_name.clone(),
                installed: false,
                version: None,
                safe_mode_supported: false,
                unsafe_mode_supported: false,
                capabilities: AdapterCapabilities::default(),
                supported_transports: vec![],
            },
            AdapterHealth::NeedsAuth { .. } => ProviderStatus {
                provider: provider_name.clone(),
                installed: true,
                version: None,
                safe_mode_supported: caps.permissions,
                unsafe_mode_supported: true,
                capabilities: caps,
                supported_transports,
            },
            _ => ProviderStatus {
                provider: provider_name.clone(),
                installed: false,
                version: None,
                safe_mode_supported: false,
                unsafe_mode_supported: false,
                capabilities: AdapterCapabilities::default(),
                supported_transports: vec![],
            },
        };
        provider_statuses.push(status);
    }

    // Check for config file
    let config_path = dirs_next::config_dir()
        .map(|p| p.join("agent-adapter").join("config.toml"))
        .filter(|p| p.exists())
        .map(|p| Cow::Owned(p.to_string_lossy().to_string()))
        .unwrap_or(Cow::Borrowed("none"));

    // Get env allowlist status
    let env_allowlist_status = if config_path == "none" {
        "Using defaults (no config file)".to_string()
    } else {
        // Check if any provider has custom env allowlist
        let has_custom_env = configs.iter().any(|c| !c.env_allowlist.is_empty());
        if has_custom_env {
            "Custom env allowlist configured".to_string()
        } else {
            "Config loaded (using defaults for env)".to_string()
        }
    };

    if json_output {
        let output = DoctorOutput {
            config_path,
            providers: provider_statuses,
            env_allowlist_status,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("=== Agent Adapter Diagnostics ===\n");
        println!("Config: {}", config_path);
        println!("Environment: {}\n", env_allowlist_status);

        println!("Providers:");
        for status in &provider_statuses {
            if status.installed {
                println!("  [OK] {}", status.provider);
                if let Some(v) = &status.version {
                    println!("      Version: {}", v);
                }
                println!(
                    "      Transports: {}",
                    status.supported_transports.join(", ")
                );
                println!("      Safe mode: supported={}", status.safe_mode_supported);
                println!(
                    "      Unsafe mode: supported={}",
                    status.unsafe_mode_supported
                );
            } else {
                println!("  [NOT FOUND] {}", status.provider);
            }
        }

        println!("\nFor more information, see docs/cli.md");
    }

    Ok(())
}

/// Get list of supported transports from capabilities.
fn get_supported_transports(caps: &AdapterCapabilities) -> Vec<String> {
    let mut transports = Vec::new();
    if caps.jsonl {
        transports.push("jsonl".to_string());
    }
    if caps.acp {
        transports.push("acp".to_string());
    }
    if caps.oneshot {
        transports.push("oneshot".to_string());
    }
    if caps.pty {
        transports.push("pty".to_string());
    }
    transports
}
