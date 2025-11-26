//! Storage-related configuration

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use super::error::ConfigError;

/// Storage-related configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Path to iroh blob storage directory
    pub iroh_blobs_path: PathBuf,
    /// Path to hiqlite data directory
    pub hiqlite_data_dir: PathBuf,
    /// Path to VM state directory (for VM lifecycle management)
    pub vm_state_dir: PathBuf,
    /// Working directory for temporary files
    pub work_dir: PathBuf,
}

impl StorageConfig {
    /// Load storage configuration from environment variables
    pub fn load() -> Result<Self, ConfigError> {
        Ok(Self {
            iroh_blobs_path: std::env::var("IROH_BLOBS_PATH")
                .unwrap_or_else(|_| Self::default_iroh_blobs_path().to_string_lossy().to_string())
                .into(),
            hiqlite_data_dir: std::env::var("HQL_DATA_DIR")
                .unwrap_or_else(|_| Self::default_hiqlite_data_dir().to_string_lossy().to_string())
                .into(),
            vm_state_dir: std::env::var("VM_STATE_DIR")
                .unwrap_or_else(|_| Self::default_vm_state_dir().to_string_lossy().to_string())
                .into(),
            work_dir: std::env::var("WORK_DIR")
                .unwrap_or_else(|_| Self::default_work_dir().to_string_lossy().to_string())
                .into(),
        })
    }

    /// Apply environment variable overrides to existing configuration
    pub fn apply_env_overrides(&mut self) -> Result<(), ConfigError> {
        if let Ok(val) = std::env::var("IROH_BLOBS_PATH") {
            self.iroh_blobs_path = val.into();
        }
        if let Ok(val) = std::env::var("HQL_DATA_DIR") {
            self.hiqlite_data_dir = val.into();
        }
        if let Ok(val) = std::env::var("VM_STATE_DIR") {
            self.vm_state_dir = val.into();
        }
        if let Ok(val) = std::env::var("WORK_DIR") {
            self.work_dir = val.into();
        }
        Ok(())
    }

    // Default value functions
    fn default_iroh_blobs_path() -> PathBuf { "./data/iroh-blobs".into() }
    fn default_hiqlite_data_dir() -> PathBuf { "./data/hiqlite".into() }
    fn default_vm_state_dir() -> PathBuf { "./data/vm-state".into() }
    fn default_work_dir() -> PathBuf { "/tmp/tofu-work".into() }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            iroh_blobs_path: Self::default_iroh_blobs_path(),
            hiqlite_data_dir: Self::default_hiqlite_data_dir(),
            vm_state_dir: Self::default_vm_state_dir(),
            work_dir: Self::default_work_dir(),
        }
    }
}
