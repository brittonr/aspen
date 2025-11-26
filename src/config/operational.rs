//! Operational configuration for timing, timeouts, and health monitoring
//!
//! Consolidates timing, timeouts, health checks, resource monitoring, and VM checks
//! into a unified operational configuration module.

use serde::{Deserialize, Serialize};
use std::time::Duration;
use super::error::ConfigError;

/// Operational configuration for system timing and monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationalConfig {
    // Worker timing
    pub worker_no_work_sleep_secs: u64,
    pub worker_error_sleep_secs: u64,
    pub worker_heartbeat_interval_secs: u64,
    pub worker_heartbeat_timeout_secs: u64,
    pub worker_cleanup_interval_secs: u64,

    // Hiqlite timing
    pub hiqlite_startup_delay_secs: u64,
    pub hiqlite_check_timeout_secs: u64,
    pub hiqlite_log_throttle_secs: u64,
    pub hiqlite_retry_delay_secs: u64,

    // VM timeouts
    pub vm_startup_timeout_secs: u64,
    pub vm_shutdown_timeout_secs: u64,
    pub vm_shutdown_retry_delay_millis: u64,
    pub vm_health_check_timeout_secs: u64,
    pub vm_health_check_interval_secs: u64,
    pub vm_failed_check_limit: u32,
    pub vm_max_idle_time_secs: u64,

    // Control protocol timeouts
    pub control_protocol_read_timeout_secs: u64,
    pub control_protocol_shutdown_timeout_secs: u64,
    pub control_protocol_connect_timeout_secs: u64,

    // Network timeouts
    pub iroh_online_timeout_secs: u64,
    pub adapter_operation_timeout_secs: u64,

    // Resource monitoring
    pub resource_monitor_interval_secs: u64,
    pub resource_monitor_cleanup_on_startup: bool,
    pub memory_usage_threshold_percent: u64,
    pub disk_usage_threshold_percent: u64,
}

impl OperationalConfig {
    pub fn load() -> Result<Self, ConfigError> {
        Ok(Self {
            // Worker timing
            worker_no_work_sleep_secs: Self::env_u64("WORKER_NO_WORK_SLEEP_SECS").unwrap_or(2),
            worker_error_sleep_secs: Self::env_u64("WORKER_ERROR_SLEEP_SECS").unwrap_or(5),
            worker_heartbeat_interval_secs: Self::env_u64("WORKER_HEARTBEAT_INTERVAL_SECS").unwrap_or(30),
            worker_heartbeat_timeout_secs: Self::env_u64("WORKER_HEARTBEAT_TIMEOUT_SECS").unwrap_or(120),
            worker_cleanup_interval_secs: Self::env_u64("WORKER_CLEANUP_INTERVAL_SECS").unwrap_or(60),

            // Hiqlite timing
            hiqlite_startup_delay_secs: Self::env_u64("HIQLITE_STARTUP_DELAY_SECS").unwrap_or(3),
            hiqlite_check_timeout_secs: Self::env_u64("HIQLITE_CHECK_TIMEOUT_SECS").unwrap_or(60),
            hiqlite_log_throttle_secs: Self::env_u64("HIQLITE_LOG_THROTTLE_SECS").unwrap_or(5),
            hiqlite_retry_delay_secs: Self::env_u64("HIQLITE_RETRY_DELAY_SECS").unwrap_or(1),

            // VM timeouts
            vm_startup_timeout_secs: Self::env_u64("VM_STARTUP_TIMEOUT_SECS").unwrap_or(60),
            vm_shutdown_timeout_secs: Self::env_u64("VM_SHUTDOWN_TIMEOUT_SECS").unwrap_or(30),
            vm_shutdown_retry_delay_millis: Self::env_u64("VM_SHUTDOWN_RETRY_DELAY_MILLIS").unwrap_or(500),
            vm_health_check_timeout_secs: Self::env_u64("VM_HEALTH_CHECK_TIMEOUT_SECS").unwrap_or(10),
            vm_health_check_interval_secs: Self::env_u64("VM_HEALTH_CHECK_INTERVAL_SECS").unwrap_or(30),
            vm_failed_check_limit: Self::env_u32("VM_FAILED_CHECK_LIMIT").unwrap_or(3),
            vm_max_idle_time_secs: Self::env_u64("VM_MAX_IDLE_TIME_SECS").unwrap_or(300),

            // Control protocol timeouts
            control_protocol_read_timeout_secs: Self::env_u64("CONTROL_PROTOCOL_READ_TIMEOUT_SECS").unwrap_or(30),
            control_protocol_shutdown_timeout_secs: Self::env_u64("CONTROL_PROTOCOL_SHUTDOWN_TIMEOUT_SECS").unwrap_or(10),
            control_protocol_connect_timeout_secs: Self::env_u64("CONTROL_PROTOCOL_CONNECT_TIMEOUT_SECS").unwrap_or(10),

            // Network timeouts
            iroh_online_timeout_secs: Self::env_u64("IROH_ONLINE_TIMEOUT_SECS").unwrap_or(30),
            adapter_operation_timeout_secs: Self::env_u64("ADAPTER_OPERATION_TIMEOUT_SECS").unwrap_or(3600),

            // Resource monitoring
            resource_monitor_interval_secs: Self::env_u64("RESOURCE_MONITOR_INTERVAL_SECS").unwrap_or(60),
            resource_monitor_cleanup_on_startup: Self::env_bool("RESOURCE_MONITOR_CLEANUP_ON_STARTUP").unwrap_or(true),
            memory_usage_threshold_percent: Self::env_u64("MEMORY_USAGE_THRESHOLD_PERCENT").unwrap_or(90),
            disk_usage_threshold_percent: Self::env_u64("DISK_USAGE_THRESHOLD_PERCENT").unwrap_or(90),
        })
    }

    pub fn apply_env_overrides(&mut self) -> Result<(), ConfigError> {
        macro_rules! apply_override {
            ($field:ident, $env:literal, $parser:ident) => {
                if let Some(val) = Self::$parser($env) {
                    self.$field = val;
                }
            };
        }

        // Worker timing
        apply_override!(worker_no_work_sleep_secs, "WORKER_NO_WORK_SLEEP_SECS", env_u64);
        apply_override!(worker_error_sleep_secs, "WORKER_ERROR_SLEEP_SECS", env_u64);
        apply_override!(worker_heartbeat_interval_secs, "WORKER_HEARTBEAT_INTERVAL_SECS", env_u64);
        apply_override!(worker_heartbeat_timeout_secs, "WORKER_HEARTBEAT_TIMEOUT_SECS", env_u64);
        apply_override!(worker_cleanup_interval_secs, "WORKER_CLEANUP_INTERVAL_SECS", env_u64);

        // Hiqlite timing
        apply_override!(hiqlite_startup_delay_secs, "HIQLITE_STARTUP_DELAY_SECS", env_u64);
        apply_override!(hiqlite_check_timeout_secs, "HIQLITE_CHECK_TIMEOUT_SECS", env_u64);
        apply_override!(hiqlite_log_throttle_secs, "HIQLITE_LOG_THROTTLE_SECS", env_u64);
        apply_override!(hiqlite_retry_delay_secs, "HIQLITE_RETRY_DELAY_SECS", env_u64);

        // VM timeouts
        apply_override!(vm_startup_timeout_secs, "VM_STARTUP_TIMEOUT_SECS", env_u64);
        apply_override!(vm_shutdown_timeout_secs, "VM_SHUTDOWN_TIMEOUT_SECS", env_u64);
        apply_override!(vm_shutdown_retry_delay_millis, "VM_SHUTDOWN_RETRY_DELAY_MILLIS", env_u64);
        apply_override!(vm_health_check_timeout_secs, "VM_HEALTH_CHECK_TIMEOUT_SECS", env_u64);
        apply_override!(vm_health_check_interval_secs, "VM_HEALTH_CHECK_INTERVAL_SECS", env_u64);
        apply_override!(vm_failed_check_limit, "VM_FAILED_CHECK_LIMIT", env_u32);
        apply_override!(vm_max_idle_time_secs, "VM_MAX_IDLE_TIME_SECS", env_u64);

        // Control protocol
        apply_override!(control_protocol_read_timeout_secs, "CONTROL_PROTOCOL_READ_TIMEOUT_SECS", env_u64);
        apply_override!(control_protocol_shutdown_timeout_secs, "CONTROL_PROTOCOL_SHUTDOWN_TIMEOUT_SECS", env_u64);
        apply_override!(control_protocol_connect_timeout_secs, "CONTROL_PROTOCOL_CONNECT_TIMEOUT_SECS", env_u64);

        // Network
        apply_override!(iroh_online_timeout_secs, "IROH_ONLINE_TIMEOUT_SECS", env_u64);
        apply_override!(adapter_operation_timeout_secs, "ADAPTER_OPERATION_TIMEOUT_SECS", env_u64);

        // Resource monitoring
        apply_override!(resource_monitor_interval_secs, "RESOURCE_MONITOR_INTERVAL_SECS", env_u64);
        apply_override!(resource_monitor_cleanup_on_startup, "RESOURCE_MONITOR_CLEANUP_ON_STARTUP", env_bool);
        apply_override!(memory_usage_threshold_percent, "MEMORY_USAGE_THRESHOLD_PERCENT", env_u64);
        apply_override!(disk_usage_threshold_percent, "DISK_USAGE_THRESHOLD_PERCENT", env_u64);

        Ok(())
    }

    fn env_u64(key: &str) -> Option<u64> {
        std::env::var(key).ok().and_then(|v| v.parse().ok())
    }

    fn env_u32(key: &str) -> Option<u32> {
        std::env::var(key).ok().and_then(|v| v.parse().ok())
    }

    fn env_bool(key: &str) -> Option<bool> {
        std::env::var(key).ok().and_then(|v| v.parse().ok())
    }

    // Duration getters for convenient access
    pub fn worker_no_work_sleep(&self) -> Duration {
        Duration::from_secs(self.worker_no_work_sleep_secs)
    }

    pub fn worker_error_sleep(&self) -> Duration {
        Duration::from_secs(self.worker_error_sleep_secs)
    }

    pub fn worker_heartbeat_interval(&self) -> Duration {
        Duration::from_secs(self.worker_heartbeat_interval_secs)
    }

    pub fn worker_heartbeat_timeout(&self) -> Duration {
        Duration::from_secs(self.worker_heartbeat_timeout_secs)
    }

    pub fn worker_cleanup_interval(&self) -> Duration {
        Duration::from_secs(self.worker_cleanup_interval_secs)
    }

    pub fn vm_startup_timeout(&self) -> Duration {
        Duration::from_secs(self.vm_startup_timeout_secs)
    }

    pub fn vm_shutdown_timeout(&self) -> Duration {
        Duration::from_secs(self.vm_shutdown_timeout_secs)
    }

    pub fn vm_health_check_interval(&self) -> Duration {
        Duration::from_secs(self.vm_health_check_interval_secs)
    }

    pub fn vm_max_idle_time(&self) -> Duration {
        Duration::from_secs(self.vm_max_idle_time_secs)
    }

    pub fn resource_monitor_interval(&self) -> Duration {
        Duration::from_secs(self.resource_monitor_interval_secs)
    }
}

impl Default for OperationalConfig {
    fn default() -> Self {
        Self {
            worker_no_work_sleep_secs: 2,
            worker_error_sleep_secs: 5,
            worker_heartbeat_interval_secs: 30,
            worker_heartbeat_timeout_secs: 120,
            worker_cleanup_interval_secs: 60,
            hiqlite_startup_delay_secs: 3,
            hiqlite_check_timeout_secs: 60,
            hiqlite_log_throttle_secs: 5,
            hiqlite_retry_delay_secs: 1,
            vm_startup_timeout_secs: 60,
            vm_shutdown_timeout_secs: 30,
            vm_shutdown_retry_delay_millis: 500,
            vm_health_check_timeout_secs: 10,
            vm_health_check_interval_secs: 30,
            vm_failed_check_limit: 3,
            vm_max_idle_time_secs: 300,
            control_protocol_read_timeout_secs: 30,
            control_protocol_shutdown_timeout_secs: 10,
            control_protocol_connect_timeout_secs: 10,
            iroh_online_timeout_secs: 30,
            adapter_operation_timeout_secs: 3600,
            resource_monitor_interval_secs: 60,
            resource_monitor_cleanup_on_startup: true,
            memory_usage_threshold_percent: 90,
            disk_usage_threshold_percent: 90,
        }
    }
}
