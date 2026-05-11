//! Config types.

use serde::{Deserialize, Serialize};
/// Provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub kind: ProviderKind,
    #[serde(default)]
    pub exe: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub prompt_mode: PromptMode,
    #[serde(default)]
    pub mapper: String,
    #[serde(default)]
    pub unsafe_args: Vec<String>,
    #[serde(default)]
    pub safe_args: Vec<String>,
    #[serde(default)]
    pub env_allowlist: Vec<String>,
    #[serde(default)]
    pub cwd_behavior: CwdBehavior,
    #[serde(default)]
    pub schema_version: Option<i32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    #[default]
    JsonlCommand,
    OneShotCommand,
    AcpStdio,
    PtyFallback,
    Custom,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PromptMode {
    Stdin,
    #[default]
    Arg,
    File,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CwdBehavior {
    #[default]
    UseProvided,
    CreateTemp,
    CreateWorktree,
}
