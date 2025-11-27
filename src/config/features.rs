//! Feature flags configuration

use serde::{Deserialize, Serialize};
use super::ConfigError;

/// Feature flags configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturesConfig {
    /// Enable Flawless WASM backend
    #[serde(default = "default_true")]
    pub enable_flawless: bool,

    /// Enable VM manager subsystem
    #[serde(default = "default_true")]
    pub enable_vm_manager: bool,
}

fn default_true() -> bool {
    true
}

impl FeaturesConfig {
    /// Load feature flags from environment variables
    ///
    /// SKIP_FLAWLESS=1 disables Flawless
    /// SKIP_VM_MANAGER=1 disables VM manager
    pub fn load() -> Result<Self, ConfigError> {
        let enable_flawless = std::env::var("SKIP_FLAWLESS").is_err();
        let enable_vm_manager = std::env::var("SKIP_VM_MANAGER").is_err();

        Ok(Self {
            enable_flawless,
            enable_vm_manager,
        })
    }

    /// Check if Flawless backend is enabled
    pub fn is_flawless_enabled(&self) -> bool {
        self.enable_flawless
    }

    /// Check if VM manager is enabled
    pub fn is_vm_manager_enabled(&self) -> bool {
        self.enable_vm_manager
    }
}

impl Default for FeaturesConfig {
    fn default() -> Self {
        Self {
            enable_flawless: true,
            enable_vm_manager: true,
        }
    }
}
