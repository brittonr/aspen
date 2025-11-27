//! Timing and timeout configuration

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Timing configuration for periodic operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[deprecated(since = "0.2.0", note = "Use operational module instead")]
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

impl Default for TimingConfig {
    fn default() -> Self {
        Self {
            worker_no_work_sleep_secs: 2,
            worker_error_sleep_secs: 5,
            worker_heartbeat_interval_secs: 30,
            hiqlite_startup_delay_secs: 3,
            hiqlite_check_timeout_secs: 60,
            hiqlite_log_throttle_secs: 5,
            hiqlite_retry_delay_secs: 1,
        }
    }
}

impl TimingConfig {
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

crate::impl_config_loader! {
    TimingConfig {
        worker_no_work_sleep_secs: "WORKER_NO_WORK_SLEEP_SECS",
        worker_error_sleep_secs: "WORKER_ERROR_SLEEP_SECS",
        worker_heartbeat_interval_secs: "WORKER_HEARTBEAT_INTERVAL_SECS",
        hiqlite_startup_delay_secs: "HIQLITE_STARTUP_DELAY_SECS",
        hiqlite_check_timeout_secs: "HIQLITE_CHECK_TIMEOUT_SECS",
        hiqlite_log_throttle_secs: "HIQLITE_LOG_THROTTLE_SECS",
        hiqlite_retry_delay_secs: "HIQLITE_RETRY_DELAY_SECS",
    }
}

/// Timeout configuration for various operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[deprecated(since = "0.2.0", note = "Use operational module instead")]
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

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            vm_startup_timeout_secs: 30,
            vm_shutdown_timeout_secs: 30,
            vm_shutdown_retry_delay_millis: 500,
            vm_health_check_timeout_secs: 5,
            control_protocol_read_timeout_secs: 5,
            control_protocol_shutdown_timeout_secs: 10,
            control_protocol_connect_timeout_secs: 5,
            iroh_online_timeout_secs: 10,
            adapter_default_timeout_secs: 300,
            adapter_wait_timeout_secs: 30,
            adapter_poll_interval_millis: 500,
            tofu_plan_timeout_secs: 600,
        }
    }
}

impl TimeoutConfig {
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

crate::impl_config_loader! {
    TimeoutConfig {
        vm_startup_timeout_secs: "VM_STARTUP_TIMEOUT_SECS",
        vm_shutdown_timeout_secs: "VM_SHUTDOWN_TIMEOUT_SECS",
        vm_shutdown_retry_delay_millis: "VM_SHUTDOWN_RETRY_DELAY_MILLIS",
        vm_health_check_timeout_secs: "VM_HEALTH_CHECK_TIMEOUT_SECS",
        control_protocol_read_timeout_secs: "CONTROL_PROTOCOL_READ_TIMEOUT_SECS",
        control_protocol_shutdown_timeout_secs: "CONTROL_PROTOCOL_SHUTDOWN_TIMEOUT_SECS",
        control_protocol_connect_timeout_secs: "CONTROL_PROTOCOL_CONNECT_TIMEOUT_SECS",
        iroh_online_timeout_secs: "IROH_ONLINE_TIMEOUT_SECS",
        adapter_default_timeout_secs: "ADAPTER_DEFAULT_TIMEOUT_SECS",
        adapter_wait_timeout_secs: "ADAPTER_WAIT_TIMEOUT_SECS",
        adapter_poll_interval_millis: "ADAPTER_POLL_INTERVAL_MILLIS",
        tofu_plan_timeout_secs: "TOFU_PLAN_TIMEOUT_SECS",
    }
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[deprecated(since = "0.2.0", note = "Use operational module instead")]
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

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: 30,
            check_timeout_secs: 5,
            failure_threshold: 3,
            recovery_threshold: 2,
            enable_circuit_breaker: true,
            circuit_break_duration_secs: 60,
        }
    }
}

crate::impl_config_loader! {
    HealthCheckConfig {
        check_interval_secs: "HEALTH_CHECK_INTERVAL_SECS",
        check_timeout_secs: "HEALTH_CHECK_TIMEOUT_SECS",
        failure_threshold: "HEALTH_FAILURE_THRESHOLD",
        recovery_threshold: "HEALTH_RECOVERY_THRESHOLD",
        enable_circuit_breaker: "HEALTH_CIRCUIT_BREAKER_ENABLED",
        circuit_break_duration_secs: "HEALTH_CIRCUIT_BREAK_DURATION_SECS",
    }
}

/// Resource monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[deprecated(since = "0.2.0", note = "Use operational module instead")]
pub struct ResourceMonitorConfig {
    /// Interval between resource monitoring checks (seconds)
    pub monitor_interval_secs: u64,
}

impl Default for ResourceMonitorConfig {
    fn default() -> Self {
        Self {
            monitor_interval_secs: 10,
        }
    }
}

impl ResourceMonitorConfig {
    /// Get monitor interval duration
    pub fn monitor_interval(&self) -> Duration {
        Duration::from_secs(self.monitor_interval_secs)
    }
}

crate::impl_config_loader! {
    ResourceMonitorConfig {
        monitor_interval_secs: "RESOURCE_MONITOR_INTERVAL_SECS",
    }
}

/// VM check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[deprecated(since = "0.2.0", note = "Use operational module instead")]
pub struct VmCheckConfig {
    /// Initial interval for VM checks (seconds)
    pub initial_interval_secs: u64,
    /// Maximum interval for VM checks (seconds)
    pub max_interval_secs: u64,
    /// Timeout for VM checks (seconds)
    pub timeout_secs: u64,
}

impl Default for VmCheckConfig {
    fn default() -> Self {
        Self {
            initial_interval_secs: 1,
            max_interval_secs: 10,
            timeout_secs: 300,
        }
    }
}

impl VmCheckConfig {
    // Duration getters
    pub fn initial_interval(&self) -> Duration { Duration::from_secs(self.initial_interval_secs) }
    pub fn max_interval(&self) -> Duration { Duration::from_secs(self.max_interval_secs) }
    pub fn timeout(&self) -> Duration { Duration::from_secs(self.timeout_secs) }
}

crate::impl_config_loader! {
    VmCheckConfig {
        initial_interval_secs: "VM_CHECK_INITIAL_INTERVAL_SECS",
        max_interval_secs: "VM_CHECK_MAX_INTERVAL_SECS",
        timeout_secs: "VM_CHECK_TIMEOUT_SECS",
    }
}
