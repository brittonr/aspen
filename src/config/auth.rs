//! Authentication configuration

use serde::{Deserialize, Serialize};
use super::ConfigError;

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// API key for authentication (minimum 32 characters)
    pub api_key: String,
}

impl AuthConfig {
    /// Load authentication configuration from environment variables
    pub fn load() -> Result<Self, ConfigError> {
        let api_key = std::env::var("BLIXARD_API_KEY")
            .map_err(|_| ConfigError::MissingRequired {
                key: "BLIXARD_API_KEY".to_string(),
                hint: "Set this environment variable to a secure value (minimum 32 characters)".to_string(),
            })?;

        // Validate API key length
        if api_key.len() < 32 {
            return Err(ConfigError::InvalidValue {
                key: "BLIXARD_API_KEY".to_string(),
                value: "<redacted>".to_string(),
                reason: "must be at least 32 characters for security".to_string(),
            });
        }

        // Check for default key
        if api_key == "CHANGE_ME_YOUR_SECRET_API_KEY_HERE_MIN_32_CHARS" {
            return Err(ConfigError::InvalidValue {
                key: "BLIXARD_API_KEY".to_string(),
                value: "<default>".to_string(),
                reason: "please change the default API key in .env".to_string(),
            });
        }

        Ok(Self { api_key })
    }

    /// Get the configured API key
    pub fn api_key(&self) -> &str {
        &self.api_key
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            api_key: "CHANGE_ME_YOUR_SECRET_API_KEY_HERE_MIN_32_CHARS".to_string(),
        }
    }
}
