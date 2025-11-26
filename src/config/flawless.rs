//! Flawless WASM runtime configuration

use serde::{Deserialize, Serialize};
use super::error::ConfigError;

/// Flawless WASM runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlawlessConfig {
    /// URL of the Flawless WASM runtime server
    pub flawless_url: String,
}

impl FlawlessConfig {
    /// Load Flawless configuration from environment variables
    pub fn load() -> Result<Self, ConfigError> {
        let flawless_url = std::env::var("FLAWLESS_URL")
            .unwrap_or_else(|_| Self::default_flawless_url());

        // Basic URL validation
        if !flawless_url.starts_with("http://") && !flawless_url.starts_with("https://") {
            return Err(ConfigError::InvalidValue {
                key: "FLAWLESS_URL".to_string(),
                value: flawless_url,
                reason: "must start with http:// or https://".to_string(),
            });
        }

        Ok(Self { flawless_url })
    }

    /// Apply environment variable overrides to existing configuration
    pub fn apply_env_overrides(&mut self) -> Result<(), ConfigError> {
        if let Ok(val) = std::env::var("FLAWLESS_URL") {
            if !val.starts_with("http://") && !val.starts_with("https://") {
                return Err(ConfigError::InvalidValue {
                    key: "FLAWLESS_URL".to_string(),
                    value: val,
                    reason: "must start with http:// or https://".to_string(),
                });
            }
            self.flawless_url = val;
        }
        Ok(())
    }

    // Default value function
    fn default_flawless_url() -> String { "http://localhost:27288".to_string() }
}

impl Default for FlawlessConfig {
    fn default() -> Self {
        Self {
            flawless_url: Self::default_flawless_url(),
        }
    }
}
