//! VM Filesystem Management
//!
//! Handles directory creation and cleanup for VMs

use anyhow::Result;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Manages filesystem operations for VMs
#[derive(Clone)]
pub struct VmFilesystem {
    state_dir: PathBuf,
}

impl VmFilesystem {
    /// Create a new filesystem manager
    pub fn new(state_dir: PathBuf) -> Self {
        Self { state_dir }
    }

    /// Create directories for a VM
    ///
    /// Returns (vm_dir, job_dir)
    pub async fn create_vm_directories(&self, vm_id: Uuid) -> Result<(PathBuf, PathBuf)> {
        let vm_dir = self.state_dir.join(format!("vms/{}", vm_id));
        let job_dir = self.state_dir.join(format!("jobs/{}", vm_id));

        tokio::fs::create_dir_all(&vm_dir).await?;
        tokio::fs::create_dir_all(&job_dir).await?;

        Ok((vm_dir, job_dir))
    }

    /// Get VM directory path
    pub fn get_vm_dir(&self, vm_id: Uuid) -> PathBuf {
        self.state_dir.join(format!("vms/{}", vm_id))
    }

    /// Get job directory path
    pub fn get_job_dir(&self, vm_id: Uuid) -> PathBuf {
        self.state_dir.join(format!("jobs/{}", vm_id))
    }

    /// Clean up VM directories
    pub async fn cleanup_vm_directories(&self, vm_id: Uuid, job_dir: Option<&Path>) {
        // Clean up job directory if provided
        if let Some(job_dir) = job_dir {
            match tokio::fs::remove_dir_all(job_dir).await {
                Ok(_) => tracing::debug!(
                    vm_id = %vm_id,
                    path = %job_dir.display(),
                    "Removed VM job directory"
                ),
                Err(e) => {
                    // NotFound is expected if directory was already cleaned up
                    if e.kind() != std::io::ErrorKind::NotFound {
                        tracing::error!(
                            vm_id = %vm_id,
                            path = %job_dir.display(),
                            error = %e,
                            "Failed to remove VM job directory - resource leak detected"
                        );
                    }
                }
            }
        }

        // Clean up VM state directory
        let vm_dir = self.get_vm_dir(vm_id);
        match tokio::fs::remove_dir_all(&vm_dir).await {
            Ok(_) => tracing::debug!(
                vm_id = %vm_id,
                path = %vm_dir.display(),
                "Removed VM state directory"
            ),
            Err(e) => {
                // NotFound is expected if directory was already cleaned up
                if e.kind() != std::io::ErrorKind::NotFound {
                    tracing::error!(
                        vm_id = %vm_id,
                        path = %vm_dir.display(),
                        error = %e,
                        "Failed to remove VM state directory - resource leak detected"
                    );
                }
            }
        }
    }

    /// Write VM configuration file
    pub async fn write_vm_config(&self, vm_id: Uuid, config: &serde_json::Value) -> Result<()> {
        let vm_dir = self.get_vm_dir(vm_id);
        let config_file = vm_dir.join("config.json");
        tokio::fs::write(&config_file, config.to_string()).await?;
        Ok(())
    }
}
