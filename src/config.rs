//! Centralized application configuration
//!
//! This module provides a single source of truth for all application configuration,
//! supporting environment variables with sensible defaults and validation.

#![allow(dead_code)] // Config methods for future configuration patterns

use std::path::PathBuf;
use std::time::Duration;

/// Network-related configuration
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// HTTP server port for local workflows and Web UI
    pub http_port: u16,
    /// HTTP server bind address
    pub http_bind_addr: String,
    /// Iroh ALPN protocol identifier for HTTP/3 over QUIC
    pub iroh_alpn: Vec<u8>,
}

impl NetworkConfig {
    /// Load network configuration from environment variables
    pub fn load() -> Result<Self, ConfigError> {
        let http_port = std::env::var("HTTP_PORT")
            .unwrap_or_else(|_| "3020".to_string())
            .parse::<u16>()
            .map_err(|e| ConfigError::InvalidValue {
                key: "HTTP_PORT".to_string(),
                value: std::env::var("HTTP_PORT").unwrap_or_default(),
                reason: format!("must be a valid port number (0-65535): {}", e),
            })?;

        Ok(Self {
            http_port,
            http_bind_addr: "0.0.0.0".to_string(),
            iroh_alpn: b"iroh+h3".to_vec(),
        })
    }

    /// Get default configuration (useful for testing)
    pub fn default() -> Self {
        Self {
            http_port: 3020,
            http_bind_addr: "0.0.0.0".to_string(),
            iroh_alpn: b"iroh+h3".to_vec(),
        }
    }
}

/// Storage-related configuration
#[derive(Debug, Clone)]
pub struct StorageConfig {
    /// Path to iroh blob storage directory
    pub iroh_blobs_path: PathBuf,
    /// Path to hiqlite data directory
    pub hiqlite_data_dir: PathBuf,
    /// Path to VM state directory (for VM lifecycle management)
    pub vm_state_dir: PathBuf,
}

impl StorageConfig {
    /// Load storage configuration from environment variables
    pub fn load() -> Result<Self, ConfigError> {
        let iroh_blobs_path = std::env::var("IROH_BLOBS_PATH")
            .unwrap_or_else(|_| "./data/iroh-blobs".to_string())
            .into();

        let hiqlite_data_dir = std::env::var("HQL_DATA_DIR")
            .unwrap_or_else(|_| "./data/hiqlite".to_string())
            .into();

        let vm_state_dir = std::env::var("VM_STATE_DIR")
            .unwrap_or_else(|_| "./data/vm-state".to_string())
            .into();

        Ok(Self {
            iroh_blobs_path,
            hiqlite_data_dir,
            vm_state_dir,
        })
    }

    /// Get default configuration (useful for testing)
    pub fn default() -> Self {
        Self {
            iroh_blobs_path: "./data/iroh-blobs".into(),
            hiqlite_data_dir: "./data/hiqlite".into(),
            vm_state_dir: "./data/vm-state".into(),
        }
    }
}

/// Flawless WASM runtime configuration
#[derive(Debug, Clone)]
pub struct FlawlessConfig {
    /// URL of the Flawless WASM runtime server
    pub flawless_url: String,
}

impl FlawlessConfig {
    /// Load Flawless configuration from environment variables
    pub fn load() -> Result<Self, ConfigError> {
        let flawless_url = std::env::var("FLAWLESS_URL")
            .unwrap_or_else(|_| "http://localhost:27288".to_string());

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

    /// Get default configuration (useful for testing)
    pub fn default() -> Self {
        Self {
            flawless_url: "http://localhost:27288".to_string(),
        }
    }
}

/// Virtual Machine configuration (Cloud Hypervisor)
#[derive(Debug, Clone)]
pub struct VmConfig {
    /// Path to microvm flake directory
    pub flake_dir: std::path::PathBuf,
    /// Path to store VM state and logs
    pub state_dir: std::path::PathBuf,
    /// Default memory allocation for VMs (MB)
    pub default_memory_mb: u32,
    /// Default vCPU count for VMs
    pub default_vcpus: u32,
    /// Maximum number of concurrent VMs
    pub max_concurrent_vms: usize,
}

impl VmConfig {
    /// Load VM configuration from environment variables
    pub fn load() -> Result<Self, ConfigError> {
        let flake_dir = std::env::var("FIRECRACKER_FLAKE_DIR")
            .unwrap_or_else(|_| "./microvms".to_string())
            .into();

        let state_dir = std::env::var("FIRECRACKER_STATE_DIR")
            .unwrap_or_else(|_| "./data/firecracker-vms".to_string())
            .into();

        let default_memory_mb = std::env::var("FIRECRACKER_DEFAULT_MEMORY_MB")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(512);

        let default_vcpus = std::env::var("FIRECRACKER_DEFAULT_VCPUS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);

        let max_concurrent_vms = std::env::var("FIRECRACKER_MAX_CONCURRENT_VMS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);

        Ok(Self {
            flake_dir,
            state_dir,
            default_memory_mb,
            default_vcpus,
            max_concurrent_vms,
        })
    }

    /// Get default configuration (useful for testing)
    pub fn default() -> Self {
        Self {
            flake_dir: "./microvms".into(),
            state_dir: "./data/firecracker-vms".into(),
            default_memory_mb: 512,
            default_vcpus: 1,
            max_concurrent_vms: 10,
        }
    }
}

/// Timing and timeout configuration
#[derive(Debug, Clone)]
pub struct TimingConfig {
    /// Worker sleep duration when no work is available (seconds)
    pub worker_no_work_sleep_secs: u64,
    /// Worker sleep duration after claim_work() error (seconds)
    pub worker_error_sleep_secs: u64,
    /// Initial delay after starting hiqlite node (seconds)
    pub hiqlite_startup_delay_secs: u64,
}

impl TimingConfig {
    /// Load timing configuration from environment variables
    pub fn load() -> Result<Self, ConfigError> {
        let worker_no_work_sleep_secs = std::env::var("WORKER_NO_WORK_SLEEP_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(2);

        let worker_error_sleep_secs = std::env::var("WORKER_ERROR_SLEEP_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5);

        let hiqlite_startup_delay_secs = std::env::var("HIQLITE_STARTUP_DELAY_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3);

        Ok(Self {
            worker_no_work_sleep_secs,
            worker_error_sleep_secs,
            hiqlite_startup_delay_secs,
        })
    }

    /// Get default configuration (useful for testing)
    pub fn default() -> Self {
        Self {
            worker_no_work_sleep_secs: 2,
            worker_error_sleep_secs: 5,
            hiqlite_startup_delay_secs: 3,
        }
    }

    /// Get worker no-work sleep duration
    pub fn worker_no_work_sleep(&self) -> Duration {
        Duration::from_secs(self.worker_no_work_sleep_secs)
    }

    /// Get worker error sleep duration
    pub fn worker_error_sleep(&self) -> Duration {
        Duration::from_secs(self.worker_error_sleep_secs)
    }

    /// Get hiqlite startup delay duration
    pub fn hiqlite_startup_delay(&self) -> Duration {
        Duration::from_secs(self.hiqlite_startup_delay_secs)
    }
}

/// Top-level application configuration
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub network: NetworkConfig,
    pub storage: StorageConfig,
    pub flawless: FlawlessConfig,
    pub vm: VmConfig,
    pub timing: TimingConfig,
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
            timing: TimingConfig::load()?,
        })
    }

    /// Get default configuration (useful for testing)
    pub fn default() -> Self {
        Self {
            network: NetworkConfig::default(),
            storage: StorageConfig::default(),
            flawless: FlawlessConfig::default(),
            vm: VmConfig::default(),
            timing: TimingConfig::default(),
        }
    }
}

/// Configuration error types
#[derive(Debug)]
pub enum ConfigError {
    /// A configuration value is invalid
    InvalidValue {
        key: String,
        value: String,
        reason: String,
    },
    /// A required configuration value is missing
    MissingRequired {
        key: String,
        hint: String,
    },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::InvalidValue { key, value, reason } => {
                write!(f, "Invalid configuration for {}: '{}' ({})", key, value, reason)
            }
            ConfigError::MissingRequired { key, hint } => {
                write!(f, "Missing required configuration: {} ({})", key, hint)
            }
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.network.http_port, 3020);
        assert_eq!(config.network.http_bind_addr, "0.0.0.0");
        assert_eq!(config.network.iroh_alpn, b"iroh+h3");
        assert_eq!(config.storage.iroh_blobs_path, PathBuf::from("./data/iroh-blobs"));
        assert_eq!(config.flawless.flawless_url, "http://localhost:27288");
        assert_eq!(config.timing.worker_no_work_sleep_secs, 2);
    }

    #[test]
    fn test_timing_durations() {
        let timing = TimingConfig::default();
        assert_eq!(timing.worker_no_work_sleep(), Duration::from_secs(2));
        assert_eq!(timing.worker_error_sleep(), Duration::from_secs(5));
        assert_eq!(timing.hiqlite_startup_delay(), Duration::from_secs(3));
    }
}
