//! Centralized application configuration
//!
//! This module provides a modular configuration system with support for TOML files
//! and environment variables with sensible defaults and validation.

#![allow(dead_code)] // Config methods for future configuration patterns

pub mod error;
pub mod network;
pub mod storage;
pub mod flawless;
pub mod vm;
pub mod operational;
pub mod adapter;
pub mod validation;

// Deprecated - use operational module instead
#[deprecated(since = "0.2.0", note = "Use operational module instead")]
pub mod timing;

use serde::{Deserialize, Serialize};

pub use error::ConfigError;
pub use network::NetworkConfig;
pub use storage::StorageConfig;
pub use flawless::FlawlessConfig;
pub use vm::VmConfig;
pub use operational::OperationalConfig;
pub use adapter::{AdapterConfig, JobRouterConfig};
pub use validation::ValidationConfig;

// Deprecated - use OperationalConfig instead
#[allow(deprecated)]
#[deprecated(since = "0.2.0", note = "Use OperationalConfig instead")]
pub use timing::{TimingConfig, TimeoutConfig, HealthCheckConfig, ResourceMonitorConfig, VmCheckConfig};

/// Top-level application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub network: NetworkConfig,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub flawless: FlawlessConfig,
    #[serde(default)]
    pub vm: VmConfig,
    #[serde(default)]
    pub operational: OperationalConfig,
    #[serde(default)]
    pub adapter: AdapterConfig,
    #[serde(default)]
    pub job_router: JobRouterConfig,
    #[serde(default)]
    pub validation: ValidationConfig,

    // Deprecated fields - maintained for backward compatibility
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[deprecated(since = "0.2.0", note = "Use operational instead")]
    pub timing: Option<TimingConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[deprecated(since = "0.2.0", note = "Use operational instead")]
    pub timeouts: Option<TimeoutConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[deprecated(since = "0.2.0", note = "Use operational instead")]
    pub health_check: Option<HealthCheckConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[deprecated(since = "0.2.0", note = "Use operational instead")]
    pub resource_monitor: Option<ResourceMonitorConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[deprecated(since = "0.2.0", note = "Use operational instead")]
    pub vm_check: Option<VmCheckConfig>,
}

impl AppConfig {
    /// Load complete application configuration from environment variables
    ///
    /// This validates all configuration values and returns an error if any are invalid.
    /// All optional values have sensible defaults.
    pub fn load() -> Result<Self, ConfigError> {
        Ok(Self {
            network: NetworkConfig::load()?,
            storage: StorageConfig::load()?,
            flawless: FlawlessConfig::load()?,
            vm: VmConfig::load()?,
            operational: OperationalConfig::load()?,
            adapter: AdapterConfig::load()?,
            job_router: JobRouterConfig::load()?,
            validation: ValidationConfig::load()?,
            timing: None,
            timeouts: None,
            health_check: None,
            resource_monitor: None,
            vm_check: None,
        })
    }

    /// Load configuration from a TOML file
    pub fn from_toml_file(path: impl AsRef<std::path::Path>) -> Result<Self, ConfigError> {
        let contents = std::fs::read_to_string(path.as_ref())
            .map_err(|e| ConfigError::InvalidValue {
                key: "config_file".to_string(),
                value: path.as_ref().display().to_string(),
                reason: format!("Failed to read file: {}", e),
            })?;

        toml::from_str(&contents)
            .map_err(|e| ConfigError::InvalidValue {
                key: "config_file".to_string(),
                value: path.as_ref().display().to_string(),
                reason: format!("Failed to parse TOML: {}", e),
            })
    }

    /// Load configuration with layered approach:
    /// 1. Start with defaults
    /// 2. Load from TOML file if it exists
    /// 3. Override with environment variables
    ///
    /// Configuration precedence (highest to lowest):
    /// - Environment variables (always win)
    /// - Config file specified by CONFIG_FILE env var
    /// - ./config.toml
    /// - ./config/default.toml
    /// - Hardcoded defaults
    pub fn load_with_layers() -> Result<Self, ConfigError> {
        // Start with TOML configuration if available
        let mut config = Self::load_toml_with_fallbacks()?;

        // Apply environment variable overrides
        config.apply_env_overrides()?;

        Ok(config)
    }

    /// Load TOML configuration from default locations
    /// Tries in order: CONFIG_FILE env var -> ./config.toml -> ./config/default.toml -> defaults
    fn load_toml_with_fallbacks() -> Result<Self, ConfigError> {
        // Check for explicit config file path
        if let Ok(config_path) = std::env::var("CONFIG_FILE") {
            let path = std::path::Path::new(&config_path);
            if path.exists() {
                tracing::info!("Loading configuration from CONFIG_FILE: {}", config_path);
                return Self::from_toml_file(path);
            } else {
                tracing::warn!("CONFIG_FILE specified but not found: {}", config_path);
            }
        }

        // Try ./config.toml
        let local_config = std::path::Path::new("./config.toml");
        if local_config.exists() {
            tracing::info!("Loading configuration from: ./config.toml");
            return Self::from_toml_file(local_config);
        }

        // Try ./config/default.toml
        let default_config = std::path::Path::new("./config/default.toml");
        if default_config.exists() {
            tracing::info!("Loading configuration from: ./config/default.toml");
            return Self::from_toml_file(default_config);
        }

        // No TOML file found, use hardcoded defaults
        tracing::info!("No configuration file found, using hardcoded defaults");
        Ok(Self::default())
    }

    /// Apply environment variable overrides to existing configuration
    /// This allows env vars to override TOML values
    fn apply_env_overrides(&mut self) -> Result<(), ConfigError> {
        self.network.apply_env_overrides()?;
        self.storage.apply_env_overrides()?;
        self.flawless.apply_env_overrides()?;
        self.vm.apply_env_overrides()?;
        self.operational.apply_env_overrides()?;
        self.adapter.apply_env_overrides()?;
        self.job_router.apply_env_overrides()?;
        self.validation.apply_env_overrides()?;

        Ok(())
    }

    /// Load configuration from TOML file if it exists, otherwise use environment variables
    #[deprecated(since = "0.1.0", note = "Use load_with_layers() instead for proper precedence")]
    pub fn load_with_optional_file(path: Option<impl AsRef<std::path::Path>>) -> Result<Self, ConfigError> {
        if let Some(path) = path {
            if path.as_ref().exists() {
                tracing::info!("Loading configuration from file: {}", path.as_ref().display());
                return Self::from_toml_file(path);
            }
        }

        tracing::info!("Loading configuration from environment variables");
        Self::load()
    }

    /// Save configuration to a TOML file
    pub fn save_to_file(&self, path: impl AsRef<std::path::Path>) -> Result<(), ConfigError> {
        let contents = toml::to_string_pretty(self)
            .map_err(|e| ConfigError::InvalidValue {
                key: "config".to_string(),
                value: "".to_string(),
                reason: format!("Failed to serialize config: {}", e),
            })?;

        std::fs::write(path.as_ref(), contents)
            .map_err(|e| ConfigError::InvalidValue {
                key: "config_file".to_string(),
                value: path.as_ref().display().to_string(),
                reason: format!("Failed to write file: {}", e),
            })
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            network: NetworkConfig::default(),
            storage: StorageConfig::default(),
            flawless: FlawlessConfig::default(),
            vm: VmConfig::default(),
            operational: OperationalConfig::default(),
            adapter: AdapterConfig::default(),
            job_router: JobRouterConfig::default(),
            validation: ValidationConfig::default(),
            timing: None,
            timeouts: None,
            health_check: None,
            resource_monitor: None,
            vm_check: None,
        }
    }
}
