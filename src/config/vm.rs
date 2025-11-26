//! Virtual Machine configuration

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use super::error::ConfigError;

/// Virtual Machine configuration (Cloud Hypervisor)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    /// Path to microvm flake directory
    pub flake_dir: PathBuf,
    /// Path to store VM state and logs
    pub state_dir: PathBuf,
    /// Default memory allocation for VMs (MB)
    pub default_memory_mb: u32,
    /// Default vCPU count for VMs
    pub default_vcpus: u32,
    /// Maximum number of concurrent VMs
    pub max_concurrent_vms: usize,
    /// Enable auto-scaling
    pub auto_scaling: bool,
    /// Pre-warm this many idle VMs
    pub pre_warm_count: usize,
}

impl VmConfig {
    /// Load VM configuration from environment variables
    pub fn load() -> Result<Self, ConfigError> {
        Ok(Self {
            flake_dir: std::env::var("FIRECRACKER_FLAKE_DIR")
                .unwrap_or_else(|_| Self::default_flake_dir().to_string_lossy().to_string())
                .into(),
            state_dir: std::env::var("FIRECRACKER_STATE_DIR")
                .unwrap_or_else(|_| Self::default_state_dir().to_string_lossy().to_string())
                .into(),
            default_memory_mb: std::env::var("FIRECRACKER_DEFAULT_MEMORY_MB")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(Self::default_memory_mb),
            default_vcpus: std::env::var("FIRECRACKER_DEFAULT_VCPUS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(Self::default_vcpus),
            max_concurrent_vms: std::env::var("FIRECRACKER_MAX_CONCURRENT_VMS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(Self::default_max_concurrent),
            auto_scaling: std::env::var("VM_AUTO_SCALING")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(Self::default_auto_scaling),
            pre_warm_count: std::env::var("VM_PRE_WARM_COUNT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(Self::default_pre_warm_count),
        })
    }

    /// Apply environment variable overrides to existing configuration
    pub fn apply_env_overrides(&mut self) -> Result<(), ConfigError> {
        if let Ok(val) = std::env::var("FIRECRACKER_FLAKE_DIR") {
            self.flake_dir = val.into();
        }
        if let Ok(val) = std::env::var("FIRECRACKER_STATE_DIR") {
            self.state_dir = val.into();
        }
        if let Ok(val) = std::env::var("FIRECRACKER_DEFAULT_MEMORY_MB") {
            self.default_memory_mb = val.parse().ok().unwrap_or(self.default_memory_mb);
        }
        if let Ok(val) = std::env::var("FIRECRACKER_DEFAULT_VCPUS") {
            self.default_vcpus = val.parse().ok().unwrap_or(self.default_vcpus);
        }
        if let Ok(val) = std::env::var("FIRECRACKER_MAX_CONCURRENT_VMS") {
            self.max_concurrent_vms = val.parse().ok().unwrap_or(self.max_concurrent_vms);
        }
        if let Ok(val) = std::env::var("VM_AUTO_SCALING") {
            self.auto_scaling = val.parse().ok().unwrap_or(self.auto_scaling);
        }
        if let Ok(val) = std::env::var("VM_PRE_WARM_COUNT") {
            self.pre_warm_count = val.parse().ok().unwrap_or(self.pre_warm_count);
        }
        Ok(())
    }

    // Default value functions
    fn default_flake_dir() -> PathBuf { "./microvms".into() }
    fn default_state_dir() -> PathBuf { "./data/firecracker-vms".into() }
    fn default_memory_mb() -> u32 { 512 }
    fn default_vcpus() -> u32 { 1 }
    fn default_max_concurrent() -> usize { 10 }
    fn default_auto_scaling() -> bool { true }
    fn default_pre_warm_count() -> usize { 2 }
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            flake_dir: Self::default_flake_dir(),
            state_dir: Self::default_state_dir(),
            default_memory_mb: Self::default_memory_mb(),
            default_vcpus: Self::default_vcpus(),
            max_concurrent_vms: Self::default_max_concurrent(),
            auto_scaling: Self::default_auto_scaling(),
            pre_warm_count: Self::default_pre_warm_count(),
        }
    }
}
