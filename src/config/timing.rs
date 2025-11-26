//! Timing and timeout configuration

use serde::{Deserialize, Serialize};
use std::time::Duration;
use super::error::ConfigError;

/// Timing configuration for periodic operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingConfig {
    /// Worker sleep duration when no work is available (seconds)
    pub worker_no_work_sleep_secs: u64,
    /// Worker sleep duration after claim_work() error (seconds)
    pub worker_error_sleep_secs: u64,
    /// Worker heartbeat interval (seconds)
    pub worker_heartbeat_interval_secs: u64,
    /// Initial delay after starting hiqlite node (seconds)
    pub hiqlite_startup_delay_secs: u64,
    /// Timeout for hiqlite health checks (seconds)
    pub hiqlite_check_timeout_secs: u64,
    /// Throttle duration for hiqlite log messages (seconds)
    pub hiqlite_log_throttle_secs: u64,
    /// Delay between hiqlite retry attempts (seconds)
    pub hiqlite_retry_delay_secs: u64,
}

impl TimingConfig {
    /// Load timing configuration from environment variables
    pub fn load() -> Result<Self, ConfigError> {
        Ok(Self {
            worker_no_work_sleep_secs: Self::parse_env("WORKER_NO_WORK_SLEEP_SECS")
                .unwrap_or_else(Self::default_worker_no_work_sleep_secs),
            worker_error_sleep_secs: Self::parse_env("WORKER_ERROR_SLEEP_SECS")
                .unwrap_or_else(Self::default_worker_error_sleep_secs),
            worker_heartbeat_interval_secs: Self::parse_env("WORKER_HEARTBEAT_INTERVAL_SECS")
                .unwrap_or_else(Self::default_worker_heartbeat_interval_secs),
            hiqlite_startup_delay_secs: Self::parse_env("HIQLITE_STARTUP_DELAY_SECS")
                .unwrap_or_else(Self::default_hiqlite_startup_delay_secs),
            hiqlite_check_timeout_secs: Self::parse_env("HIQLITE_CHECK_TIMEOUT_SECS")
                .unwrap_or_else(Self::default_hiqlite_check_timeout_secs),
            hiqlite_log_throttle_secs: Self::parse_env("HIQLITE_LOG_THROTTLE_SECS")
                .unwrap_or_else(Self::default_hiqlite_log_throttle_secs),
            hiqlite_retry_delay_secs: Self::parse_env("HIQLITE_RETRY_DELAY_SECS")
                .unwrap_or_else(Self::default_hiqlite_retry_delay_secs),
        })
    }

    /// Apply environment variable overrides to existing configuration
    pub fn apply_env_overrides(&mut self) -> Result<(), ConfigError> {
        if let Some(val) = Self::parse_env("WORKER_NO_WORK_SLEEP_SECS") {
            self.worker_no_work_sleep_secs = val;
        }
        if let Some(val) = Self::parse_env("WORKER_ERROR_SLEEP_SECS") {
            self.worker_error_sleep_secs = val;
        }
        if let Some(val) = Self::parse_env("WORKER_HEARTBEAT_INTERVAL_SECS") {
            self.worker_heartbeat_interval_secs = val;
        }
        if let Some(val) = Self::parse_env("HIQLITE_STARTUP_DELAY_SECS") {
            self.hiqlite_startup_delay_secs = val;
        }
        if let Some(val) = Self::parse_env("HIQLITE_CHECK_TIMEOUT_SECS") {
            self.hiqlite_check_timeout_secs = val;
        }
        if let Some(val) = Self::parse_env("HIQLITE_LOG_THROTTLE_SECS") {
            self.hiqlite_log_throttle_secs = val;
        }
        if let Some(val) = Self::parse_env("HIQLITE_RETRY_DELAY_SECS") {
            self.hiqlite_retry_delay_secs = val;
        }
        Ok(())
    }

    // Helper to parse env vars
    fn parse_env(key: &str) -> Option<u64> {
        std::env::var(key).ok().and_then(|v| v.parse().ok())
    }

    // Default value functions
    fn default_worker_no_work_sleep_secs() -> u64 { 2 }
    fn default_worker_error_sleep_secs() -> u64 { 5 }
    fn default_worker_heartbeat_interval_secs() -> u64 { 30 }
    fn default_hiqlite_startup_delay_secs() -> u64 { 3 }
    fn default_hiqlite_check_timeout_secs() -> u64 { 60 }
    fn default_hiqlite_log_throttle_secs() -> u64 { 5 }
    fn default_hiqlite_retry_delay_secs() -> u64 { 1 }

    // Duration getters
    pub fn worker_no_work_sleep(&self) -> Duration {
        Duration::from_secs(self.worker_no_work_sleep_secs)
    }

    pub fn worker_error_sleep(&self) -> Duration {
        Duration::from_secs(self.worker_error_sleep_secs)
    }

    pub fn worker_heartbeat_interval(&self) -> Duration {
        Duration::from_secs(self.worker_heartbeat_interval_secs)
    }

    pub fn hiqlite_startup_delay(&self) -> Duration {
        Duration::from_secs(self.hiqlite_startup_delay_secs)
    }

    pub fn hiqlite_check_timeout(&self) -> Duration {
        Duration::from_secs(self.hiqlite_check_timeout_secs)
    }

    pub fn hiqlite_log_throttle(&self) -> Duration {
        Duration::from_secs(self.hiqlite_log_throttle_secs)
    }

    pub fn hiqlite_retry_delay(&self) -> Duration {
        Duration::from_secs(self.hiqlite_retry_delay_secs)
    }
}

impl Default for TimingConfig {
    fn default() -> Self {
        Self {
            worker_no_work_sleep_secs: Self::default_worker_no_work_sleep_secs(),
            worker_error_sleep_secs: Self::default_worker_error_sleep_secs(),
            worker_heartbeat_interval_secs: Self::default_worker_heartbeat_interval_secs(),
            hiqlite_startup_delay_secs: Self::default_hiqlite_startup_delay_secs(),
            hiqlite_check_timeout_secs: Self::default_hiqlite_check_timeout_secs(),
            hiqlite_log_throttle_secs: Self::default_hiqlite_log_throttle_secs(),
            hiqlite_retry_delay_secs: Self::default_hiqlite_retry_delay_secs(),
        }
    }
}

/// Timeout configuration for various operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// VM startup timeout (seconds)
    pub vm_startup_timeout_secs: u64,
    /// VM shutdown timeout (seconds)
    pub vm_shutdown_timeout_secs: u64,
    /// VM shutdown retry delay (milliseconds)
    pub vm_shutdown_retry_delay_millis: u64,
    /// VM health check timeout (seconds)
    pub vm_health_check_timeout_secs: u64,
    /// Control protocol read timeout (seconds)
    pub control_protocol_read_timeout_secs: u64,
    /// Control protocol shutdown timeout (seconds)
    pub control_protocol_shutdown_timeout_secs: u64,
    /// Control protocol connect timeout (seconds)
    pub control_protocol_connect_timeout_secs: u64,
    /// Iroh online timeout (seconds)
    pub iroh_online_timeout_secs: u64,
    /// Default timeout for adapter operations (seconds)
    pub adapter_default_timeout_secs: u64,
    /// Timeout for waiting on adapter operations (seconds)
    pub adapter_wait_timeout_secs: u64,
    /// Poll interval for adapter status checks (milliseconds)
    pub adapter_poll_interval_millis: u64,
    /// Timeout for tofu plan operations (seconds)
    pub tofu_plan_timeout_secs: u64,
}

impl TimeoutConfig {
    /// Load timeout configuration from environment variables
    pub fn load() -> Result<Self, ConfigError> {
        Ok(Self {
            vm_startup_timeout_secs: Self::parse_env("VM_STARTUP_TIMEOUT_SECS")
                .unwrap_or_else(Self::default_vm_startup_timeout_secs),
            vm_shutdown_timeout_secs: Self::parse_env("VM_SHUTDOWN_TIMEOUT_SECS")
                .unwrap_or_else(Self::default_vm_shutdown_timeout_secs),
            vm_shutdown_retry_delay_millis: Self::parse_env("VM_SHUTDOWN_RETRY_DELAY_MILLIS")
                .unwrap_or_else(Self::default_vm_shutdown_retry_delay_millis),
            vm_health_check_timeout_secs: Self::parse_env("VM_HEALTH_CHECK_TIMEOUT_SECS")
                .unwrap_or_else(Self::default_vm_health_check_timeout_secs),
            control_protocol_read_timeout_secs: Self::parse_env("CONTROL_PROTOCOL_READ_TIMEOUT_SECS")
                .unwrap_or_else(Self::default_control_protocol_read_timeout_secs),
            control_protocol_shutdown_timeout_secs: Self::parse_env("CONTROL_PROTOCOL_SHUTDOWN_TIMEOUT_SECS")
                .unwrap_or_else(Self::default_control_protocol_shutdown_timeout_secs),
            control_protocol_connect_timeout_secs: Self::parse_env("CONTROL_PROTOCOL_CONNECT_TIMEOUT_SECS")
                .unwrap_or_else(Self::default_control_protocol_connect_timeout_secs),
            iroh_online_timeout_secs: Self::parse_env("IROH_ONLINE_TIMEOUT_SECS")
                .unwrap_or_else(Self::default_iroh_online_timeout_secs),
            adapter_default_timeout_secs: Self::parse_env("ADAPTER_DEFAULT_TIMEOUT_SECS")
                .unwrap_or_else(Self::default_adapter_default_timeout_secs),
            adapter_wait_timeout_secs: Self::parse_env("ADAPTER_WAIT_TIMEOUT_SECS")
                .unwrap_or_else(Self::default_adapter_wait_timeout_secs),
            adapter_poll_interval_millis: Self::parse_env("ADAPTER_POLL_INTERVAL_MILLIS")
                .unwrap_or_else(Self::default_adapter_poll_interval_millis),
            tofu_plan_timeout_secs: Self::parse_env("TOFU_PLAN_TIMEOUT_SECS")
                .unwrap_or_else(Self::default_tofu_plan_timeout_secs),
        })
    }

    /// Apply environment variable overrides to existing configuration
    pub fn apply_env_overrides(&mut self) -> Result<(), ConfigError> {
        if let Some(val) = Self::parse_env("VM_STARTUP_TIMEOUT_SECS") {
            self.vm_startup_timeout_secs = val;
        }
        if let Some(val) = Self::parse_env("VM_SHUTDOWN_TIMEOUT_SECS") {
            self.vm_shutdown_timeout_secs = val;
        }
        if let Some(val) = Self::parse_env("VM_SHUTDOWN_RETRY_DELAY_MILLIS") {
            self.vm_shutdown_retry_delay_millis = val;
        }
        if let Some(val) = Self::parse_env("VM_HEALTH_CHECK_TIMEOUT_SECS") {
            self.vm_health_check_timeout_secs = val;
        }
        if let Some(val) = Self::parse_env("CONTROL_PROTOCOL_READ_TIMEOUT_SECS") {
            self.control_protocol_read_timeout_secs = val;
        }
        if let Some(val) = Self::parse_env("CONTROL_PROTOCOL_SHUTDOWN_TIMEOUT_SECS") {
            self.control_protocol_shutdown_timeout_secs = val;
        }
        if let Some(val) = Self::parse_env("CONTROL_PROTOCOL_CONNECT_TIMEOUT_SECS") {
            self.control_protocol_connect_timeout_secs = val;
        }
        if let Some(val) = Self::parse_env("IROH_ONLINE_TIMEOUT_SECS") {
            self.iroh_online_timeout_secs = val;
        }
        if let Some(val) = Self::parse_env("ADAPTER_DEFAULT_TIMEOUT_SECS") {
            self.adapter_default_timeout_secs = val;
        }
        if let Some(val) = Self::parse_env("ADAPTER_WAIT_TIMEOUT_SECS") {
            self.adapter_wait_timeout_secs = val;
        }
        if let Some(val) = Self::parse_env("ADAPTER_POLL_INTERVAL_MILLIS") {
            self.adapter_poll_interval_millis = val;
        }
        if let Some(val) = Self::parse_env("TOFU_PLAN_TIMEOUT_SECS") {
            self.tofu_plan_timeout_secs = val;
        }
        Ok(())
    }

    // Helper to parse env vars
    fn parse_env(key: &str) -> Option<u64> {
        std::env::var(key).ok().and_then(|v| v.parse().ok())
    }

    // Default value functions
    fn default_vm_startup_timeout_secs() -> u64 { 30 }
    fn default_vm_shutdown_timeout_secs() -> u64 { 30 }
    fn default_vm_shutdown_retry_delay_millis() -> u64 { 500 }
    fn default_vm_health_check_timeout_secs() -> u64 { 5 }
    fn default_control_protocol_read_timeout_secs() -> u64 { 5 }
    fn default_control_protocol_shutdown_timeout_secs() -> u64 { 10 }
    fn default_control_protocol_connect_timeout_secs() -> u64 { 5 }
    fn default_iroh_online_timeout_secs() -> u64 { 10 }
    fn default_adapter_default_timeout_secs() -> u64 { 300 }
    fn default_adapter_wait_timeout_secs() -> u64 { 30 }
    fn default_adapter_poll_interval_millis() -> u64 { 500 }
    fn default_tofu_plan_timeout_secs() -> u64 { 600 }

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

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            vm_startup_timeout_secs: Self::default_vm_startup_timeout_secs(),
            vm_shutdown_timeout_secs: Self::default_vm_shutdown_timeout_secs(),
            vm_shutdown_retry_delay_millis: Self::default_vm_shutdown_retry_delay_millis(),
            vm_health_check_timeout_secs: Self::default_vm_health_check_timeout_secs(),
            control_protocol_read_timeout_secs: Self::default_control_protocol_read_timeout_secs(),
            control_protocol_shutdown_timeout_secs: Self::default_control_protocol_shutdown_timeout_secs(),
            control_protocol_connect_timeout_secs: Self::default_control_protocol_connect_timeout_secs(),
            iroh_online_timeout_secs: Self::default_iroh_online_timeout_secs(),
            adapter_default_timeout_secs: Self::default_adapter_default_timeout_secs(),
            adapter_wait_timeout_secs: Self::default_adapter_wait_timeout_secs(),
            adapter_poll_interval_millis: Self::default_adapter_poll_interval_millis(),
            tofu_plan_timeout_secs: Self::default_tofu_plan_timeout_secs(),
        }
    }
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Interval between health checks (seconds)
    pub check_interval_secs: u64,
    /// Timeout for health check response (seconds)
    pub check_timeout_secs: u64,
    /// Number of failures before marking unhealthy
    pub failure_threshold: u32,
    /// Number of successes to recover from unhealthy
    pub recovery_threshold: u32,
    /// Enable circuit breaker behavior
    pub enable_circuit_breaker: bool,
    /// Time to wait before retrying unhealthy VM (seconds)
    pub circuit_break_duration_secs: u64,
}

impl HealthCheckConfig {
    /// Load health check configuration from environment variables
    pub fn load() -> Result<Self, ConfigError> {
        Ok(Self {
            check_interval_secs: Self::parse_env_u64("HEALTH_CHECK_INTERVAL_SECS")
                .unwrap_or_else(Self::default_check_interval_secs),
            check_timeout_secs: Self::parse_env_u64("HEALTH_CHECK_TIMEOUT_SECS")
                .unwrap_or_else(Self::default_check_timeout_secs),
            failure_threshold: Self::parse_env_u32("HEALTH_FAILURE_THRESHOLD")
                .unwrap_or_else(Self::default_failure_threshold),
            recovery_threshold: Self::parse_env_u32("HEALTH_RECOVERY_THRESHOLD")
                .unwrap_or_else(Self::default_recovery_threshold),
            enable_circuit_breaker: Self::parse_env_bool("HEALTH_CIRCUIT_BREAKER_ENABLED")
                .unwrap_or_else(Self::default_enable_circuit_breaker),
            circuit_break_duration_secs: Self::parse_env_u64("HEALTH_CIRCUIT_BREAK_DURATION_SECS")
                .unwrap_or_else(Self::default_circuit_break_duration_secs),
        })
    }

    /// Apply environment variable overrides to existing configuration
    pub fn apply_env_overrides(&mut self) -> Result<(), ConfigError> {
        if let Some(val) = Self::parse_env_u64("HEALTH_CHECK_INTERVAL_SECS") {
            self.check_interval_secs = val;
        }
        if let Some(val) = Self::parse_env_u64("HEALTH_CHECK_TIMEOUT_SECS") {
            self.check_timeout_secs = val;
        }
        if let Some(val) = Self::parse_env_u32("HEALTH_FAILURE_THRESHOLD") {
            self.failure_threshold = val;
        }
        if let Some(val) = Self::parse_env_u32("HEALTH_RECOVERY_THRESHOLD") {
            self.recovery_threshold = val;
        }
        if let Some(val) = Self::parse_env_bool("HEALTH_CIRCUIT_BREAKER_ENABLED") {
            self.enable_circuit_breaker = val;
        }
        if let Some(val) = Self::parse_env_u64("HEALTH_CIRCUIT_BREAK_DURATION_SECS") {
            self.circuit_break_duration_secs = val;
        }
        Ok(())
    }

    // Helper to parse env vars
    fn parse_env_u64(key: &str) -> Option<u64> {
        std::env::var(key).ok().and_then(|v| v.parse().ok())
    }
    fn parse_env_u32(key: &str) -> Option<u32> {
        std::env::var(key).ok().and_then(|v| v.parse().ok())
    }
    fn parse_env_bool(key: &str) -> Option<bool> {
        std::env::var(key).ok().and_then(|v| v.parse().ok())
    }

    // Default value functions
    fn default_check_interval_secs() -> u64 { 30 }
    fn default_check_timeout_secs() -> u64 { 5 }
    fn default_failure_threshold() -> u32 { 3 }
    fn default_recovery_threshold() -> u32 { 2 }
    fn default_enable_circuit_breaker() -> bool { true }
    fn default_circuit_break_duration_secs() -> u64 { 60 }
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: Self::default_check_interval_secs(),
            check_timeout_secs: Self::default_check_timeout_secs(),
            failure_threshold: Self::default_failure_threshold(),
            recovery_threshold: Self::default_recovery_threshold(),
            enable_circuit_breaker: Self::default_enable_circuit_breaker(),
            circuit_break_duration_secs: Self::default_circuit_break_duration_secs(),
        }
    }
}

/// Resource monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMonitorConfig {
    /// Interval between resource monitoring checks (seconds)
    pub monitor_interval_secs: u64,
}

impl ResourceMonitorConfig {
    /// Load resource monitor configuration from environment variables
    pub fn load() -> Result<Self, ConfigError> {
        Ok(Self {
            monitor_interval_secs: std::env::var("RESOURCE_MONITOR_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(Self::default_monitor_interval_secs),
        })
    }

    /// Apply environment variable overrides to existing configuration
    pub fn apply_env_overrides(&mut self) -> Result<(), ConfigError> {
        if let Ok(val) = std::env::var("RESOURCE_MONITOR_INTERVAL_SECS") {
            self.monitor_interval_secs = val.parse().ok().unwrap_or(self.monitor_interval_secs);
        }
        Ok(())
    }

    // Default value function
    fn default_monitor_interval_secs() -> u64 { 10 }

    /// Get monitor interval duration
    pub fn monitor_interval(&self) -> Duration {
        Duration::from_secs(self.monitor_interval_secs)
    }
}

impl Default for ResourceMonitorConfig {
    fn default() -> Self {
        Self {
            monitor_interval_secs: Self::default_monitor_interval_secs(),
        }
    }
}

/// VM check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmCheckConfig {
    /// Initial interval for VM checks (seconds)
    pub initial_interval_secs: u64,
    /// Maximum interval for VM checks (seconds)
    pub max_interval_secs: u64,
    /// Timeout for VM checks (seconds)
    pub timeout_secs: u64,
}

impl VmCheckConfig {
    /// Load VM check configuration from environment variables
    pub fn load() -> Result<Self, ConfigError> {
        Ok(Self {
            initial_interval_secs: Self::parse_env("VM_CHECK_INITIAL_INTERVAL_SECS")
                .unwrap_or_else(Self::default_initial_interval_secs),
            max_interval_secs: Self::parse_env("VM_CHECK_MAX_INTERVAL_SECS")
                .unwrap_or_else(Self::default_max_interval_secs),
            timeout_secs: Self::parse_env("VM_CHECK_TIMEOUT_SECS")
                .unwrap_or_else(Self::default_timeout_secs),
        })
    }

    /// Apply environment variable overrides to existing configuration
    pub fn apply_env_overrides(&mut self) -> Result<(), ConfigError> {
        if let Some(val) = Self::parse_env("VM_CHECK_INITIAL_INTERVAL_SECS") {
            self.initial_interval_secs = val;
        }
        if let Some(val) = Self::parse_env("VM_CHECK_MAX_INTERVAL_SECS") {
            self.max_interval_secs = val;
        }
        if let Some(val) = Self::parse_env("VM_CHECK_TIMEOUT_SECS") {
            self.timeout_secs = val;
        }
        Ok(())
    }

    // Helper to parse env vars
    fn parse_env(key: &str) -> Option<u64> {
        std::env::var(key).ok().and_then(|v| v.parse().ok())
    }

    // Default value functions
    fn default_initial_interval_secs() -> u64 { 1 }
    fn default_max_interval_secs() -> u64 { 10 }
    fn default_timeout_secs() -> u64 { 300 }

    // Duration getters
    pub fn initial_interval(&self) -> Duration { Duration::from_secs(self.initial_interval_secs) }
    pub fn max_interval(&self) -> Duration { Duration::from_secs(self.max_interval_secs) }
    pub fn timeout(&self) -> Duration { Duration::from_secs(self.timeout_secs) }
}

impl Default for VmCheckConfig {
    fn default() -> Self {
        Self {
            initial_interval_secs: Self::default_initial_interval_secs(),
            max_interval_secs: Self::default_max_interval_secs(),
            timeout_secs: Self::default_timeout_secs(),
        }
    }
}
