//! Doctor command implementation.

use agent_adapter_core::AdapterCapabilities;
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
