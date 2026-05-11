//! Provider registry for agent-adapter.
//!
//! This module provides a registry of provider configurations
//! and factory functions that can be used to build provider adapters.

use crate::ProviderConfig;
use std::collections::HashMap;
/// A registry of provider configurations.
#[derive(Debug, Clone)]
pub struct ProviderRegistry {
    configs: HashMap<String, ProviderConfig>,
}
