//! Virtiofs Daemon Management
//!
//! Handles lifecycle of virtiofsd processes for service VMs

use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tokio::time::Duration;
use uuid::Uuid;

/// Manages virtiofsd daemons for VMs
pub struct VirtiofsDaemon {
    flake_dir: PathBuf,
}

impl VirtiofsDaemon {
    /// Create a new virtiofs daemon manager
    pub fn new(flake_dir: PathBuf) -> Self {
        Self { flake_dir }
    }

    /// Start virtiofsd daemon for a service VM
    ///
    /// Returns the PID of the virtiofsd process
    pub async fn start_virtiofsd(&self, vm_id: Uuid, log_dir: &Path) -> Result<u32> {
        tracing::info!(vm_id = %vm_id, "Starting virtiofsd for VM");

        let virtiofsd_path = self.flake_dir.join("result-service/bin/virtiofsd-run");

        // Clean up any existing sockets (they'll be created in the flake directory)
        let store_sock = self.flake_dir.join("service-vm-virtiofs-store.sock");
        let jobs_sock = self.flake_dir.join("service-vm-virtiofs-jobs.sock");

        self.cleanup_socket(&store_sock, vm_id).await;
        self.cleanup_socket(&jobs_sock, vm_id).await;

        // Start virtiofsd-run in the project/flake directory
        let mut virtiofsd_cmd = Command::new(&virtiofsd_path);
        virtiofsd_cmd.current_dir(&self.flake_dir);

        // Redirect virtiofsd output to log files
        let virtiofsd_stdout = tokio::fs::File::create(log_dir.join("virtiofsd.stdout.log")).await?;
        let virtiofsd_stderr = tokio::fs::File::create(log_dir.join("virtiofsd.stderr.log")).await?;
        virtiofsd_cmd.stdout(virtiofsd_stdout.into_std().await);
        virtiofsd_cmd.stderr(virtiofsd_stderr.into_std().await);

        let virtiofsd_child = virtiofsd_cmd.spawn()?;
        let virtiofsd_pid = virtiofsd_child.id()
            .ok_or_else(|| anyhow!("Failed to get virtiofsd PID"))?;

        tracing::info!(vm_id = %vm_id, pid = virtiofsd_pid, "Started virtiofsd daemon");

        // Wait for virtiofs sockets to be created
        self.wait_for_sockets(&store_sock, &jobs_sock, vm_id).await?;

        Ok(virtiofsd_pid)
    }

    /// Clean up a socket file
    async fn cleanup_socket(&self, socket_path: &Path, vm_id: Uuid) {
        if let Err(e) = tokio::fs::remove_file(socket_path).await {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(
                    vm_id = %vm_id,
                    path = %socket_path.display(),
                    error = %e,
                    "Failed to remove old virtiofs socket"
                );
            }
        }
    }

    /// Wait for virtiofs sockets to be created
    async fn wait_for_sockets(
        &self,
        store_sock: &Path,
        jobs_sock: &Path,
        vm_id: Uuid,
    ) -> Result<()> {
        let mut retries = 0;
        loop {
            if store_sock.exists() && jobs_sock.exists() {
                tracing::info!(vm_id = %vm_id, "virtiofs sockets ready");
                return Ok(());
            }
            if retries >= 30 {
                return Err(anyhow!("virtiofs sockets not created after 15 seconds"));
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
            retries += 1;
        }
    }
}
