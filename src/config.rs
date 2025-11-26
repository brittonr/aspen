//! Centralized application configuration
//!
//! This module provides a single source of truth for all application configuration,
//! supporting both TOML files and environment variables with sensible defaults and validation.

#![allow(dead_code)] // Config methods for future configuration patterns

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Default values for configuration
mod defaults {
    use std::path::PathBuf;

    // Network defaults
    pub fn http_port() -> u16 { 3020 }
    pub fn http_bind_addr() -> String { "0.0.0.0".to_string() }
    pub fn iroh_alpn() -> String { "iroh+h3".to_string() }

    // Storage defaults
    pub fn iroh_blobs_path() -> PathBuf { "./data/iroh-blobs".into() }
    pub fn hiqlite_data_dir() -> PathBuf { "./data/hiqlite".into() }
    pub fn vm_state_dir() -> PathBuf { "./data/vm-state".into() }
    pub fn work_dir() -> PathBuf { "/tmp/tofu-work".into() }

    // Flawless defaults
    pub fn flawless_url() -> String { "http://localhost:27288".to_string() }

    // VM defaults
    pub fn vm_flake_dir() -> PathBuf { "./microvms".into() }
    pub fn vm_state_dir_alt() -> PathBuf { "./data/firecracker-vms".into() }
    pub fn vm_default_memory_mb() -> u32 { 512 }
    pub fn vm_default_vcpus() -> u32 { 1 }
    pub fn vm_max_concurrent() -> usize { 10 }
    pub fn vm_auto_scaling() -> bool { true }
    pub fn vm_pre_warm_count() -> usize { 2 }

    // Timing defaults (seconds)
    pub fn worker_no_work_sleep_secs() -> u64 { 2 }
    pub fn worker_error_sleep_secs() -> u64 { 5 }
    pub fn worker_heartbeat_interval_secs() -> u64 { 30 }
    pub fn hiqlite_startup_delay_secs() -> u64 { 3 }
    pub fn hiqlite_check_timeout_secs() -> u64 { 60 }
    pub fn hiqlite_log_throttle_secs() -> u64 { 5 }
    pub fn hiqlite_retry_delay_secs() -> u64 { 1 }

    // Timeout defaults (seconds)
    pub fn vm_startup_timeout_secs() -> u64 { 30 }
    pub fn vm_shutdown_timeout_secs() -> u64 { 30 }
    pub fn vm_shutdown_retry_delay_millis() -> u64 { 500 }
    pub fn vm_health_check_timeout_secs() -> u64 { 5 }
    pub fn control_protocol_read_timeout_secs() -> u64 { 5 }
    pub fn control_protocol_shutdown_timeout_secs() -> u64 { 10 }
    pub fn control_protocol_connect_timeout_secs() -> u64 { 5 }
    pub fn iroh_online_timeout_secs() -> u64 { 10 }
    pub fn adapter_default_timeout_secs() -> u64 { 300 }
    pub fn adapter_wait_timeout_secs() -> u64 { 30 }
    pub fn adapter_poll_interval_millis() -> u64 { 500 }
    pub fn tofu_plan_timeout_secs() -> u64 { 600 }

    // Health check defaults
    pub fn health_check_interval_secs() -> u64 { 30 }
    pub fn health_check_timeout_secs() -> u64 { 5 }
    pub fn health_failure_threshold() -> u32 { 3 }
    pub fn health_recovery_threshold() -> u32 { 2 }
    pub fn health_circuit_breaker_enabled() -> bool { true }
    pub fn health_circuit_break_duration_secs() -> u64 { 60 }

    // Resource monitor defaults
    pub fn resource_monitor_interval_secs() -> u64 { 10 }

    // Adapter defaults
    pub fn adapter_max_concurrent() -> usize { 10 }
    pub fn adapter_max_executions() -> usize { 20 }
    pub fn adapter_execution_delay_ms() -> u64 { 100 }
    pub fn adapter_max_submission_retries() -> u32 { 3 }
    pub fn adapter_health_check_interval_secs() -> u64 { 30 }

    // Job router defaults
    pub fn job_router_max_jobs_per_vm() -> u32 { 50 }
    pub fn job_router_auto_create_vms() -> bool { true }
    pub fn job_router_prefer_service_vms() -> bool { true }

    // Validation defaults
    pub fn validation_job_id_min_length() -> usize { 1 }
    pub fn validation_job_id_max_length() -> usize { 255 }
    pub fn validation_payload_max_size_bytes() -> usize { 1024 * 1024 } // 1MB

    // VM check defaults
    pub fn vm_check_initial_interval_secs() -> u64 { 1 }
    pub fn vm_check_max_interval_secs() -> u64 { 10 }
    pub fn vm_check_timeout_secs() -> u64 { 300 }
}

/// Network-related configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// HTTP server port for local workflows and Web UI
    #[serde(default = "defaults::http_port")]
    pub http_port: u16,
    /// HTTP server bind address
    #[serde(default = "defaults::http_bind_addr")]
    pub http_bind_addr: String,
    /// Iroh ALPN protocol identifier for HTTP/3 over QUIC
    #[serde(default = "defaults::iroh_alpn")]
    pub iroh_alpn: String,
}

impl NetworkConfig {
    /// Load network configuration from environment variables
    pub fn load() -> Result<Self, ConfigError> {
        let http_port = std::env::var("HTTP_PORT")
            .unwrap_or_else(|_| defaults::http_port().to_string())
            .parse::<u16>()
            .map_err(|e| ConfigError::InvalidValue {
                key: "HTTP_PORT".to_string(),
                value: std::env::var("HTTP_PORT").unwrap_or_default(),
                reason: format!("must be a valid port number (0-65535): {}", e),
            })?;

        Ok(Self {
            http_port,
            http_bind_addr: std::env::var("HTTP_BIND_ADDR")
                .unwrap_or_else(|_| defaults::http_bind_addr()),
            iroh_alpn: std::env::var("IROH_ALPN")
                .unwrap_or_else(|_| defaults::iroh_alpn()),
        })
    }

    /// Get default configuration (useful for testing)
    pub fn default() -> Self {
        Self {
            http_port: defaults::http_port(),
            http_bind_addr: defaults::http_bind_addr(),
            iroh_alpn: defaults::iroh_alpn(),
        }
    }

    /// Get ALPN as bytes (for backwards compatibility)
    pub fn iroh_alpn_bytes(&self) -> Vec<u8> {
        self.iroh_alpn.as_bytes().to_vec()
    }
}

/// Storage-related configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Path to iroh blob storage directory
    #[serde(default = "defaults::iroh_blobs_path")]
    pub iroh_blobs_path: PathBuf,
    /// Path to hiqlite data directory
    #[serde(default = "defaults::hiqlite_data_dir")]
    pub hiqlite_data_dir: PathBuf,
    /// Path to VM state directory (for VM lifecycle management)
    #[serde(default = "defaults::vm_state_dir")]
    pub vm_state_dir: PathBuf,
    /// Working directory for temporary files
    #[serde(default = "defaults::work_dir")]
    pub work_dir: PathBuf,
}

impl StorageConfig {
    /// Load storage configuration from environment variables
    pub fn load() -> Result<Self, ConfigError> {
        Ok(Self {
            iroh_blobs_path: std::env::var("IROH_BLOBS_PATH")
                .unwrap_or_else(|_| defaults::iroh_blobs_path().to_string_lossy().to_string())
                .into(),
            hiqlite_data_dir: std::env::var("HQL_DATA_DIR")
                .unwrap_or_else(|_| defaults::hiqlite_data_dir().to_string_lossy().to_string())
                .into(),
            vm_state_dir: std::env::var("VM_STATE_DIR")
                .unwrap_or_else(|_| defaults::vm_state_dir().to_string_lossy().to_string())
                .into(),
            work_dir: std::env::var("WORK_DIR")
                .unwrap_or_else(|_| defaults::work_dir().to_string_lossy().to_string())
                .into(),
        })
    }

    /// Get default configuration (useful for testing)
    pub fn default() -> Self {
        Self {
            iroh_blobs_path: defaults::iroh_blobs_path(),
            hiqlite_data_dir: defaults::hiqlite_data_dir(),
            vm_state_dir: defaults::vm_state_dir(),
            work_dir: defaults::work_dir(),
        }
    }
}

/// Flawless WASM runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlawlessConfig {
    /// URL of the Flawless WASM runtime server
    #[serde(default = "defaults::flawless_url")]
    pub flawless_url: String,
}

impl FlawlessConfig {
    /// Load Flawless configuration from environment variables
    pub fn load() -> Result<Self, ConfigError> {
        let flawless_url = std::env::var("FLAWLESS_URL")
            .unwrap_or_else(|_| defaults::flawless_url());

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
            flawless_url: defaults::flawless_url(),
        }
    }
}

/// Virtual Machine configuration (Cloud Hypervisor)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    /// Path to microvm flake directory
    #[serde(default = "defaults::vm_flake_dir")]
    pub flake_dir: PathBuf,
    /// Path to store VM state and logs
    #[serde(default = "defaults::vm_state_dir_alt")]
    pub state_dir: PathBuf,
    /// Default memory allocation for VMs (MB)
    #[serde(default = "defaults::vm_default_memory_mb")]
    pub default_memory_mb: u32,
    /// Default vCPU count for VMs
    #[serde(default = "defaults::vm_default_vcpus")]
    pub default_vcpus: u32,
    /// Maximum number of concurrent VMs
    #[serde(default = "defaults::vm_max_concurrent")]
    pub max_concurrent_vms: usize,
    /// Enable auto-scaling
    #[serde(default = "defaults::vm_auto_scaling")]
    pub auto_scaling: bool,
    /// Pre-warm this many idle VMs
    #[serde(default = "defaults::vm_pre_warm_count")]
    pub pre_warm_count: usize,
}

impl VmConfig {
    /// Load VM configuration from environment variables
    pub fn load() -> Result<Self, ConfigError> {
        Ok(Self {
            flake_dir: std::env::var("FIRECRACKER_FLAKE_DIR")
                .unwrap_or_else(|_| defaults::vm_flake_dir().to_string_lossy().to_string())
                .into(),
            state_dir: std::env::var("FIRECRACKER_STATE_DIR")
                .unwrap_or_else(|_| defaults::vm_state_dir_alt().to_string_lossy().to_string())
                .into(),
            default_memory_mb: std::env::var("FIRECRACKER_DEFAULT_MEMORY_MB")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::vm_default_memory_mb),
            default_vcpus: std::env::var("FIRECRACKER_DEFAULT_VCPUS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::vm_default_vcpus),
            max_concurrent_vms: std::env::var("FIRECRACKER_MAX_CONCURRENT_VMS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::vm_max_concurrent),
            auto_scaling: std::env::var("VM_AUTO_SCALING")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::vm_auto_scaling),
            pre_warm_count: std::env::var("VM_PRE_WARM_COUNT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::vm_pre_warm_count),
        })
    }

    /// Get default configuration (useful for testing)
    pub fn default() -> Self {
        Self {
            flake_dir: defaults::vm_flake_dir(),
            state_dir: defaults::vm_state_dir_alt(),
            default_memory_mb: defaults::vm_default_memory_mb(),
            default_vcpus: defaults::vm_default_vcpus(),
            max_concurrent_vms: defaults::vm_max_concurrent(),
            auto_scaling: defaults::vm_auto_scaling(),
            pre_warm_count: defaults::vm_pre_warm_count(),
        }
    }
}

/// Timing and timeout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingConfig {
    /// Worker sleep duration when no work is available (seconds)
    #[serde(default = "defaults::worker_no_work_sleep_secs")]
    pub worker_no_work_sleep_secs: u64,
    /// Worker sleep duration after claim_work() error (seconds)
    #[serde(default = "defaults::worker_error_sleep_secs")]
    pub worker_error_sleep_secs: u64,
    /// Worker heartbeat interval (seconds)
    #[serde(default = "defaults::worker_heartbeat_interval_secs")]
    pub worker_heartbeat_interval_secs: u64,
    /// Initial delay after starting hiqlite node (seconds)
    #[serde(default = "defaults::hiqlite_startup_delay_secs")]
    pub hiqlite_startup_delay_secs: u64,
    /// Timeout for hiqlite health checks (seconds)
    #[serde(default = "defaults::hiqlite_check_timeout_secs")]
    pub hiqlite_check_timeout_secs: u64,
    /// Throttle duration for hiqlite log messages (seconds)
    #[serde(default = "defaults::hiqlite_log_throttle_secs")]
    pub hiqlite_log_throttle_secs: u64,
    /// Delay between hiqlite retry attempts (seconds)
    #[serde(default = "defaults::hiqlite_retry_delay_secs")]
    pub hiqlite_retry_delay_secs: u64,
}

impl TimingConfig {
    /// Load timing configuration from environment variables
    pub fn load() -> Result<Self, ConfigError> {
        Ok(Self {
            worker_no_work_sleep_secs: std::env::var("WORKER_NO_WORK_SLEEP_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::worker_no_work_sleep_secs),
            worker_error_sleep_secs: std::env::var("WORKER_ERROR_SLEEP_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::worker_error_sleep_secs),
            worker_heartbeat_interval_secs: std::env::var("WORKER_HEARTBEAT_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::worker_heartbeat_interval_secs),
            hiqlite_startup_delay_secs: std::env::var("HIQLITE_STARTUP_DELAY_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::hiqlite_startup_delay_secs),
            hiqlite_check_timeout_secs: std::env::var("HIQLITE_CHECK_TIMEOUT_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::hiqlite_check_timeout_secs),
            hiqlite_log_throttle_secs: std::env::var("HIQLITE_LOG_THROTTLE_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::hiqlite_log_throttle_secs),
            hiqlite_retry_delay_secs: std::env::var("HIQLITE_RETRY_DELAY_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::hiqlite_retry_delay_secs),
        })
    }

    /// Get default configuration (useful for testing)
    pub fn default() -> Self {
        Self {
            worker_no_work_sleep_secs: defaults::worker_no_work_sleep_secs(),
            worker_error_sleep_secs: defaults::worker_error_sleep_secs(),
            worker_heartbeat_interval_secs: defaults::worker_heartbeat_interval_secs(),
            hiqlite_startup_delay_secs: defaults::hiqlite_startup_delay_secs(),
            hiqlite_check_timeout_secs: defaults::hiqlite_check_timeout_secs(),
            hiqlite_log_throttle_secs: defaults::hiqlite_log_throttle_secs(),
            hiqlite_retry_delay_secs: defaults::hiqlite_retry_delay_secs(),
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

    /// Get worker heartbeat interval
    pub fn worker_heartbeat_interval(&self) -> Duration {
        Duration::from_secs(self.worker_heartbeat_interval_secs)
    }

    /// Get hiqlite startup delay duration
    pub fn hiqlite_startup_delay(&self) -> Duration {
        Duration::from_secs(self.hiqlite_startup_delay_secs)
    }

    /// Get hiqlite check timeout duration
    pub fn hiqlite_check_timeout(&self) -> Duration {
        Duration::from_secs(self.hiqlite_check_timeout_secs)
    }

    /// Get hiqlite log throttle duration
    pub fn hiqlite_log_throttle(&self) -> Duration {
        Duration::from_secs(self.hiqlite_log_throttle_secs)
    }

    /// Get hiqlite retry delay duration
    pub fn hiqlite_retry_delay(&self) -> Duration {
        Duration::from_secs(self.hiqlite_retry_delay_secs)
    }
}

/// Timeout configuration for various operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// VM startup timeout (seconds)
    #[serde(default = "defaults::vm_startup_timeout_secs")]
    pub vm_startup_timeout_secs: u64,
    /// VM shutdown timeout (seconds)
    #[serde(default = "defaults::vm_shutdown_timeout_secs")]
    pub vm_shutdown_timeout_secs: u64,
    /// VM shutdown retry delay (milliseconds)
    #[serde(default = "defaults::vm_shutdown_retry_delay_millis")]
    pub vm_shutdown_retry_delay_millis: u64,
    /// VM health check timeout (seconds)
    #[serde(default = "defaults::vm_health_check_timeout_secs")]
    pub vm_health_check_timeout_secs: u64,
    /// Control protocol read timeout (seconds)
    #[serde(default = "defaults::control_protocol_read_timeout_secs")]
    pub control_protocol_read_timeout_secs: u64,
    /// Control protocol shutdown timeout (seconds)
    #[serde(default = "defaults::control_protocol_shutdown_timeout_secs")]
    pub control_protocol_shutdown_timeout_secs: u64,
    /// Control protocol connect timeout (seconds)
    #[serde(default = "defaults::control_protocol_connect_timeout_secs")]
    pub control_protocol_connect_timeout_secs: u64,
    /// Iroh online timeout (seconds)
    #[serde(default = "defaults::iroh_online_timeout_secs")]
    pub iroh_online_timeout_secs: u64,
    /// Default timeout for adapter operations (seconds)
    #[serde(default = "defaults::adapter_default_timeout_secs")]
    pub adapter_default_timeout_secs: u64,
    /// Timeout for waiting on adapter operations (seconds)
    #[serde(default = "defaults::adapter_wait_timeout_secs")]
    pub adapter_wait_timeout_secs: u64,
    /// Poll interval for adapter status checks (milliseconds)
    #[serde(default = "defaults::adapter_poll_interval_millis")]
    pub adapter_poll_interval_millis: u64,
    /// Timeout for tofu plan operations (seconds)
    #[serde(default = "defaults::tofu_plan_timeout_secs")]
    pub tofu_plan_timeout_secs: u64,
}

impl TimeoutConfig {
    /// Load timeout configuration from environment variables
    pub fn load() -> Result<Self, ConfigError> {
        Ok(Self {
            vm_startup_timeout_secs: std::env::var("VM_STARTUP_TIMEOUT_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::vm_startup_timeout_secs),
            vm_shutdown_timeout_secs: std::env::var("VM_SHUTDOWN_TIMEOUT_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::vm_shutdown_timeout_secs),
            vm_shutdown_retry_delay_millis: std::env::var("VM_SHUTDOWN_RETRY_DELAY_MILLIS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::vm_shutdown_retry_delay_millis),
            vm_health_check_timeout_secs: std::env::var("VM_HEALTH_CHECK_TIMEOUT_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::vm_health_check_timeout_secs),
            control_protocol_read_timeout_secs: std::env::var("CONTROL_PROTOCOL_READ_TIMEOUT_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::control_protocol_read_timeout_secs),
            control_protocol_shutdown_timeout_secs: std::env::var("CONTROL_PROTOCOL_SHUTDOWN_TIMEOUT_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::control_protocol_shutdown_timeout_secs),
            control_protocol_connect_timeout_secs: std::env::var("CONTROL_PROTOCOL_CONNECT_TIMEOUT_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::control_protocol_connect_timeout_secs),
            iroh_online_timeout_secs: std::env::var("IROH_ONLINE_TIMEOUT_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::iroh_online_timeout_secs),
            adapter_default_timeout_secs: std::env::var("ADAPTER_DEFAULT_TIMEOUT_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::adapter_default_timeout_secs),
            adapter_wait_timeout_secs: std::env::var("ADAPTER_WAIT_TIMEOUT_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::adapter_wait_timeout_secs),
            adapter_poll_interval_millis: std::env::var("ADAPTER_POLL_INTERVAL_MILLIS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::adapter_poll_interval_millis),
            tofu_plan_timeout_secs: std::env::var("TOFU_PLAN_TIMEOUT_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::tofu_plan_timeout_secs),
        })
    }

    /// Get default configuration
    pub fn default() -> Self {
        Self {
            vm_startup_timeout_secs: defaults::vm_startup_timeout_secs(),
            vm_shutdown_timeout_secs: defaults::vm_shutdown_timeout_secs(),
            vm_shutdown_retry_delay_millis: defaults::vm_shutdown_retry_delay_millis(),
            vm_health_check_timeout_secs: defaults::vm_health_check_timeout_secs(),
            control_protocol_read_timeout_secs: defaults::control_protocol_read_timeout_secs(),
            control_protocol_shutdown_timeout_secs: defaults::control_protocol_shutdown_timeout_secs(),
            control_protocol_connect_timeout_secs: defaults::control_protocol_connect_timeout_secs(),
            iroh_online_timeout_secs: defaults::iroh_online_timeout_secs(),
            adapter_default_timeout_secs: defaults::adapter_default_timeout_secs(),
            adapter_wait_timeout_secs: defaults::adapter_wait_timeout_secs(),
            adapter_poll_interval_millis: defaults::adapter_poll_interval_millis(),
            tofu_plan_timeout_secs: defaults::tofu_plan_timeout_secs(),
        }
    }

    // Duration getters
    pub fn vm_startup_timeout(&self) -> Duration { Duration::from_secs(self.vm_startup_timeout_secs) }
    pub fn vm_shutdown_timeout(&self) -> Duration { Duration::from_secs(self.vm_shutdown_timeout_secs) }
    pub fn vm_shutdown_retry_delay(&self) -> Duration { Duration::from_millis(self.vm_shutdown_retry_delay_millis) }
    pub fn vm_health_check_timeout(&self) -> Duration { Duration::from_secs(self.vm_health_check_timeout_secs) }
    pub fn control_protocol_read_timeout(&self) -> Duration { Duration::from_secs(self.control_protocol_read_timeout_secs) }
    pub fn control_protocol_shutdown_timeout(&self) -> Duration { Duration::from_secs(self.control_protocol_shutdown_timeout_secs) }
    pub fn control_protocol_connect_timeout(&self) -> Duration { Duration::from_secs(self.control_protocol_connect_timeout_secs) }
    pub fn iroh_online_timeout(&self) -> Duration { Duration::from_secs(self.iroh_online_timeout_secs) }
    pub fn adapter_default_timeout(&self) -> Duration { Duration::from_secs(self.adapter_default_timeout_secs) }
    pub fn adapter_wait_timeout(&self) -> Duration { Duration::from_secs(self.adapter_wait_timeout_secs) }
    pub fn adapter_poll_interval(&self) -> Duration { Duration::from_millis(self.adapter_poll_interval_millis) }
    pub fn tofu_plan_timeout(&self) -> Duration { Duration::from_secs(self.tofu_plan_timeout_secs) }
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Interval between health checks (seconds)
    #[serde(default = "defaults::health_check_interval_secs")]
    pub check_interval_secs: u64,
    /// Timeout for health check response (seconds)
    #[serde(default = "defaults::health_check_timeout_secs")]
    pub check_timeout_secs: u64,
    /// Number of failures before marking unhealthy
    #[serde(default = "defaults::health_failure_threshold")]
    pub failure_threshold: u32,
    /// Number of successes to recover from unhealthy
    #[serde(default = "defaults::health_recovery_threshold")]
    pub recovery_threshold: u32,
    /// Enable circuit breaker behavior
    #[serde(default = "defaults::health_circuit_breaker_enabled")]
    pub enable_circuit_breaker: bool,
    /// Time to wait before retrying unhealthy VM (seconds)
    #[serde(default = "defaults::health_circuit_break_duration_secs")]
    pub circuit_break_duration_secs: u64,
}

impl HealthCheckConfig {
    /// Load health check configuration from environment variables
    pub fn load() -> Result<Self, ConfigError> {
        Ok(Self {
            check_interval_secs: std::env::var("HEALTH_CHECK_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::health_check_interval_secs),
            check_timeout_secs: std::env::var("HEALTH_CHECK_TIMEOUT_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::health_check_timeout_secs),
            failure_threshold: std::env::var("HEALTH_FAILURE_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::health_failure_threshold),
            recovery_threshold: std::env::var("HEALTH_RECOVERY_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::health_recovery_threshold),
            enable_circuit_breaker: std::env::var("HEALTH_CIRCUIT_BREAKER_ENABLED")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::health_circuit_breaker_enabled),
            circuit_break_duration_secs: std::env::var("HEALTH_CIRCUIT_BREAK_DURATION_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::health_circuit_break_duration_secs),
        })
    }

    /// Get default configuration
    pub fn default() -> Self {
        Self {
            check_interval_secs: defaults::health_check_interval_secs(),
            check_timeout_secs: defaults::health_check_timeout_secs(),
            failure_threshold: defaults::health_failure_threshold(),
            recovery_threshold: defaults::health_recovery_threshold(),
            enable_circuit_breaker: defaults::health_circuit_breaker_enabled(),
            circuit_break_duration_secs: defaults::health_circuit_break_duration_secs(),
        }
    }
}

/// Resource monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMonitorConfig {
    /// Interval between resource monitoring checks (seconds)
    #[serde(default = "defaults::resource_monitor_interval_secs")]
    pub monitor_interval_secs: u64,
}

impl ResourceMonitorConfig {
    /// Load resource monitor configuration from environment variables
    pub fn load() -> Result<Self, ConfigError> {
        Ok(Self {
            monitor_interval_secs: std::env::var("RESOURCE_MONITOR_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::resource_monitor_interval_secs),
        })
    }

    /// Get default configuration
    pub fn default() -> Self {
        Self {
            monitor_interval_secs: defaults::resource_monitor_interval_secs(),
        }
    }

    /// Get monitor interval duration
    pub fn monitor_interval(&self) -> Duration {
        Duration::from_secs(self.monitor_interval_secs)
    }
}

/// Adapter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterConfig {
    /// Maximum concurrent executions
    #[serde(default = "defaults::adapter_max_concurrent")]
    pub max_concurrent: usize,
    /// Maximum total executions
    #[serde(default = "defaults::adapter_max_executions")]
    pub max_executions: usize,
    /// Simulated execution delay in milliseconds (for mock adapter)
    #[serde(default = "defaults::adapter_execution_delay_ms")]
    pub execution_delay_ms: u64,
    /// Maximum submission retries
    #[serde(default = "defaults::adapter_max_submission_retries")]
    pub max_submission_retries: u32,
    /// Health check interval (seconds)
    #[serde(default = "defaults::adapter_health_check_interval_secs")]
    pub health_check_interval_secs: u64,
}

impl AdapterConfig {
    /// Load adapter configuration from environment variables
    pub fn load() -> Result<Self, ConfigError> {
        Ok(Self {
            max_concurrent: std::env::var("ADAPTER_MAX_CONCURRENT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::adapter_max_concurrent),
            max_executions: std::env::var("ADAPTER_MAX_EXECUTIONS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::adapter_max_executions),
            execution_delay_ms: std::env::var("ADAPTER_EXECUTION_DELAY_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::adapter_execution_delay_ms),
            max_submission_retries: std::env::var("ADAPTER_MAX_SUBMISSION_RETRIES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::adapter_max_submission_retries),
            health_check_interval_secs: std::env::var("ADAPTER_HEALTH_CHECK_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::adapter_health_check_interval_secs),
        })
    }

    /// Get default configuration
    pub fn default() -> Self {
        Self {
            max_concurrent: defaults::adapter_max_concurrent(),
            max_executions: defaults::adapter_max_executions(),
            execution_delay_ms: defaults::adapter_execution_delay_ms(),
            max_submission_retries: defaults::adapter_max_submission_retries(),
            health_check_interval_secs: defaults::adapter_health_check_interval_secs(),
        }
    }
}

/// Job router configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRouterConfig {
    /// Maximum jobs per service VM
    #[serde(default = "defaults::job_router_max_jobs_per_vm")]
    pub max_jobs_per_vm: u32,
    /// Enable automatic VM creation
    #[serde(default = "defaults::job_router_auto_create_vms")]
    pub auto_create_vms: bool,
    /// Prefer service VMs over ephemeral when possible
    #[serde(default = "defaults::job_router_prefer_service_vms")]
    pub prefer_service_vms: bool,
}

impl JobRouterConfig {
    /// Load job router configuration from environment variables
    pub fn load() -> Result<Self, ConfigError> {
        Ok(Self {
            max_jobs_per_vm: std::env::var("JOB_ROUTER_MAX_JOBS_PER_VM")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::job_router_max_jobs_per_vm),
            auto_create_vms: std::env::var("JOB_ROUTER_AUTO_CREATE_VMS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::job_router_auto_create_vms),
            prefer_service_vms: std::env::var("JOB_ROUTER_PREFER_SERVICE_VMS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::job_router_prefer_service_vms),
        })
    }

    /// Get default configuration
    pub fn default() -> Self {
        Self {
            max_jobs_per_vm: defaults::job_router_max_jobs_per_vm(),
            auto_create_vms: defaults::job_router_auto_create_vms(),
            prefer_service_vms: defaults::job_router_prefer_service_vms(),
        }
    }
}

/// Validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Minimum length for job IDs
    #[serde(default = "defaults::validation_job_id_min_length")]
    pub job_id_min_length: usize,
    /// Maximum length for job IDs
    #[serde(default = "defaults::validation_job_id_max_length")]
    pub job_id_max_length: usize,
    /// Maximum size for job payloads in bytes
    #[serde(default = "defaults::validation_payload_max_size_bytes")]
    pub payload_max_size_bytes: usize,
}

impl ValidationConfig {
    /// Load validation configuration from environment variables
    pub fn load() -> Result<Self, ConfigError> {
        Ok(Self {
            job_id_min_length: std::env::var("VALIDATION_JOB_ID_MIN_LENGTH")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::validation_job_id_min_length),
            job_id_max_length: std::env::var("VALIDATION_JOB_ID_MAX_LENGTH")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::validation_job_id_max_length),
            payload_max_size_bytes: std::env::var("VALIDATION_PAYLOAD_MAX_SIZE_BYTES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::validation_payload_max_size_bytes),
        })
    }

    /// Get default configuration
    pub fn default() -> Self {
        Self {
            job_id_min_length: defaults::validation_job_id_min_length(),
            job_id_max_length: defaults::validation_job_id_max_length(),
            payload_max_size_bytes: defaults::validation_payload_max_size_bytes(),
        }
    }
}

/// VM check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmCheckConfig {
    /// Initial interval for VM checks (seconds)
    #[serde(default = "defaults::vm_check_initial_interval_secs")]
    pub initial_interval_secs: u64,
    /// Maximum interval for VM checks (seconds)
    #[serde(default = "defaults::vm_check_max_interval_secs")]
    pub max_interval_secs: u64,
    /// Timeout for VM checks (seconds)
    #[serde(default = "defaults::vm_check_timeout_secs")]
    pub timeout_secs: u64,
}

impl VmCheckConfig {
    /// Load VM check configuration from environment variables
    pub fn load() -> Result<Self, ConfigError> {
        Ok(Self {
            initial_interval_secs: std::env::var("VM_CHECK_INITIAL_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::vm_check_initial_interval_secs),
            max_interval_secs: std::env::var("VM_CHECK_MAX_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::vm_check_max_interval_secs),
            timeout_secs: std::env::var("VM_CHECK_TIMEOUT_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(defaults::vm_check_timeout_secs),
        })
    }

    /// Get default configuration
    pub fn default() -> Self {
        Self {
            initial_interval_secs: defaults::vm_check_initial_interval_secs(),
            max_interval_secs: defaults::vm_check_max_interval_secs(),
            timeout_secs: defaults::vm_check_timeout_secs(),
        }
    }

    // Duration getters
    pub fn initial_interval(&self) -> Duration { Duration::from_secs(self.initial_interval_secs) }
    pub fn max_interval(&self) -> Duration { Duration::from_secs(self.max_interval_secs) }
    pub fn timeout(&self) -> Duration { Duration::from_secs(self.timeout_secs) }
}

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
    pub timing: TimingConfig,
    #[serde(default)]
    pub timeouts: TimeoutConfig,
    #[serde(default)]
    pub health_check: HealthCheckConfig,
    #[serde(default)]
    pub resource_monitor: ResourceMonitorConfig,
    #[serde(default)]
    pub adapter: AdapterConfig,
    #[serde(default)]
    pub job_router: JobRouterConfig,
    #[serde(default)]
    pub validation: ValidationConfig,
    #[serde(default)]
    pub vm_check: VmCheckConfig,
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
            timeouts: TimeoutConfig::load()?,
            health_check: HealthCheckConfig::load()?,
            resource_monitor: ResourceMonitorConfig::load()?,
            adapter: AdapterConfig::load()?,
            job_router: JobRouterConfig::load()?,
            validation: ValidationConfig::load()?,
            vm_check: VmCheckConfig::load()?,
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

    /// Load configuration from TOML file if it exists, otherwise use environment variables
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

    /// Get default configuration (useful for testing)
    pub fn default() -> Self {
        Self {
            network: NetworkConfig::default(),
            storage: StorageConfig::default(),
            flawless: FlawlessConfig::default(),
            vm: VmConfig::default(),
            timing: TimingConfig::default(),
            timeouts: TimeoutConfig::default(),
            health_check: HealthCheckConfig::default(),
            resource_monitor: ResourceMonitorConfig::default(),
            adapter: AdapterConfig::default(),
            job_router: JobRouterConfig::default(),
            validation: ValidationConfig::default(),
            vm_check: VmCheckConfig::default(),
        }
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

impl Default for NetworkConfig {
    fn default() -> Self {
        NetworkConfig::default()
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        StorageConfig::default()
    }
}

impl Default for FlawlessConfig {
    fn default() -> Self {
        FlawlessConfig::default()
    }
}

impl Default for VmConfig {
    fn default() -> Self {
        VmConfig::default()
    }
}

impl Default for TimingConfig {
    fn default() -> Self {
        TimingConfig::default()
    }
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        TimeoutConfig::default()
    }
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        HealthCheckConfig::default()
    }
}

impl Default for ResourceMonitorConfig {
    fn default() -> Self {
        ResourceMonitorConfig::default()
    }
}

impl Default for AdapterConfig {
    fn default() -> Self {
        AdapterConfig::default()
    }
}

impl Default for JobRouterConfig {
    fn default() -> Self {
        JobRouterConfig::default()
    }
}

impl Default for ValidationConfig {
    fn default() -> Self {
        ValidationConfig::default()
    }
}

impl Default for VmCheckConfig {
    fn default() -> Self {
        VmCheckConfig::default()
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
