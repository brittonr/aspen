//! Network-related configuration

use serde::{Deserialize, Serialize};
use super::error::ConfigError;

/// Network-related configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// HTTP server port for local workflows and Web UI
    pub http_port: u16,
    /// HTTP server bind address
    pub http_bind_addr: String,
    /// Iroh ALPN protocol identifier for HTTP/3 over QUIC
    pub iroh_alpn: String,
}

impl NetworkConfig {
    /// Load network configuration from environment variables
    pub fn load() -> Result<Self, ConfigError> {
        let http_port = std::env::var("HTTP_PORT")
            .unwrap_or_else(|_| Self::default_http_port().to_string())
            .parse::<u16>()
            .map_err(|e| ConfigError::InvalidValue {
                key: "HTTP_PORT".to_string(),
                value: std::env::var("HTTP_PORT").unwrap_or_default(),
                reason: format!("must be a valid port number (0-65535): {}", e),
            })?;

        Ok(Self {
            http_port,
            http_bind_addr: std::env::var("HTTP_BIND_ADDR")
                .unwrap_or_else(|_| Self::default_http_bind_addr()),
            iroh_alpn: std::env::var("IROH_ALPN")
                .unwrap_or_else(|_| Self::default_iroh_alpn()),
        })
    }

    /// Apply environment variable overrides to existing configuration
    pub fn apply_env_overrides(&mut self) -> Result<(), ConfigError> {
        if let Ok(val) = std::env::var("HTTP_PORT") {
            self.http_port = val.parse().map_err(|e| ConfigError::InvalidValue {
                key: "HTTP_PORT".to_string(),
                value: val.clone(),
                reason: format!("must be a valid port number: {}", e),
            })?;
        }
        if let Ok(val) = std::env::var("HTTP_BIND_ADDR") {
            self.http_bind_addr = val;
        }
        if let Ok(val) = std::env::var("IROH_ALPN") {
            self.iroh_alpn = val;
        }
        Ok(())
    }

    // Default value functions
    fn default_http_port() -> u16 { 3020 }
    fn default_http_bind_addr() -> String { "0.0.0.0".to_string() }
    fn default_iroh_alpn() -> String { "iroh+h3".to_string() }

    /// Get ALPN as bytes (for backwards compatibility)
    pub fn iroh_alpn_bytes(&self) -> Vec<u8> {
        self.iroh_alpn.as_bytes().to_vec()
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            http_port: Self::default_http_port(),
            http_bind_addr: Self::default_http_bind_addr(),
            iroh_alpn: Self::default_iroh_alpn(),
        }
    }
}
