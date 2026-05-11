//! Healthcheck command implementation.

use anyhow::Context;
use serde::Serialize;
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
