//! Adapter and routing configuration

use serde::{Deserialize, Serialize};
use super::error::ConfigError;

/// Adapter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterConfig {
    /// Maximum concurrent executions
    pub max_concurrent: usize,
    /// Maximum total executions
    pub max_executions: usize,
    /// Simulated execution delay in milliseconds (for mock adapter)
    pub execution_delay_ms: u64,
    /// Maximum submission retries
    pub max_submission_retries: u32,
    /// Health check interval (seconds)
    pub health_check_interval_secs: u64,
}

impl AdapterConfig {
    /// Load adapter configuration from environment variables
    pub fn load() -> Result<Self, ConfigError> {
        Ok(Self {
            max_concurrent: Self::parse_env("ADAPTER_MAX_CONCURRENT")
                .unwrap_or_else(Self::default_max_concurrent),
            max_executions: Self::parse_env("ADAPTER_MAX_EXECUTIONS")
                .unwrap_or_else(Self::default_max_executions),
            execution_delay_ms: Self::parse_env("ADAPTER_EXECUTION_DELAY_MS")
                .unwrap_or_else(Self::default_execution_delay_ms),
            max_submission_retries: Self::parse_env("ADAPTER_MAX_SUBMISSION_RETRIES")
                .unwrap_or_else(Self::default_max_submission_retries),
            health_check_interval_secs: Self::parse_env("ADAPTER_HEALTH_CHECK_INTERVAL_SECS")
                .unwrap_or_else(Self::default_health_check_interval_secs),
        })
    }

    /// Apply environment variable overrides to existing configuration
    pub fn apply_env_overrides(&mut self) -> Result<(), ConfigError> {
        if let Some(val) = Self::parse_env("ADAPTER_MAX_CONCURRENT") {
            self.max_concurrent = val;
        }
        if let Some(val) = Self::parse_env("ADAPTER_MAX_EXECUTIONS") {
            self.max_executions = val;
        }
        if let Some(val) = Self::parse_env("ADAPTER_EXECUTION_DELAY_MS") {
            self.execution_delay_ms = val;
        }
        if let Some(val) = Self::parse_env("ADAPTER_MAX_SUBMISSION_RETRIES") {
            self.max_submission_retries = val;
        }
        if let Some(val) = Self::parse_env("ADAPTER_HEALTH_CHECK_INTERVAL_SECS") {
            self.health_check_interval_secs = val;
        }
        Ok(())
    }

    // Helper to parse env vars
    fn parse_env<T: std::str::FromStr>(key: &str) -> Option<T> {
        std::env::var(key).ok().and_then(|v| v.parse().ok())
    }

    // Default value functions
    fn default_max_concurrent() -> usize { 10 }
    fn default_max_executions() -> usize { 20 }
    fn default_execution_delay_ms() -> u64 { 100 }
    fn default_max_submission_retries() -> u32 { 3 }
    fn default_health_check_interval_secs() -> u64 { 30 }
}

impl Default for AdapterConfig {
    fn default() -> Self {
        Self {
            max_concurrent: Self::default_max_concurrent(),
            max_executions: Self::default_max_executions(),
            execution_delay_ms: Self::default_execution_delay_ms(),
            max_submission_retries: Self::default_max_submission_retries(),
            health_check_interval_secs: Self::default_health_check_interval_secs(),
        }
    }
}

/// Job router configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRouterConfig {
    /// Maximum jobs per service VM
    pub max_jobs_per_vm: u32,
    /// Enable automatic VM creation
    pub auto_create_vms: bool,
    /// Prefer service VMs over ephemeral when possible
    pub prefer_service_vms: bool,
}

impl JobRouterConfig {
    /// Load job router configuration from environment variables
    pub fn load() -> Result<Self, ConfigError> {
        Ok(Self {
            max_jobs_per_vm: Self::parse_env("JOB_ROUTER_MAX_JOBS_PER_VM")
                .unwrap_or_else(Self::default_max_jobs_per_vm),
            auto_create_vms: Self::parse_env("JOB_ROUTER_AUTO_CREATE_VMS")
                .unwrap_or_else(Self::default_auto_create_vms),
            prefer_service_vms: Self::parse_env("JOB_ROUTER_PREFER_SERVICE_VMS")
                .unwrap_or_else(Self::default_prefer_service_vms),
        })
    }

    /// Apply environment variable overrides to existing configuration
    pub fn apply_env_overrides(&mut self) -> Result<(), ConfigError> {
        if let Some(val) = Self::parse_env("JOB_ROUTER_MAX_JOBS_PER_VM") {
            self.max_jobs_per_vm = val;
        }
        if let Some(val) = Self::parse_env("JOB_ROUTER_AUTO_CREATE_VMS") {
            self.auto_create_vms = val;
        }
        if let Some(val) = Self::parse_env("JOB_ROUTER_PREFER_SERVICE_VMS") {
            self.prefer_service_vms = val;
        }
        Ok(())
    }

    // Helper to parse env vars
    fn parse_env<T: std::str::FromStr>(key: &str) -> Option<T> {
        std::env::var(key).ok().and_then(|v| v.parse().ok())
    }

    // Default value functions
    fn default_max_jobs_per_vm() -> u32 { 50 }
    fn default_auto_create_vms() -> bool { true }
    fn default_prefer_service_vms() -> bool { true }
}

impl Default for JobRouterConfig {
    fn default() -> Self {
        Self {
            max_jobs_per_vm: Self::default_max_jobs_per_vm(),
            auto_create_vms: Self::default_auto_create_vms(),
            prefer_service_vms: Self::default_prefer_service_vms(),
        }
    }
}
