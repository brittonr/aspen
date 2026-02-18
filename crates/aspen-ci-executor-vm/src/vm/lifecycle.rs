//! VM lifecycle management: start, assign, release, pause, resume, shutdown.

use std::path::PathBuf;
use std::time::Duration;

use aspen_core::CI_VM_BOOT_TIMEOUT_MS;
use aspen_core::CI_VM_NIX_STORE_TAG;
use aspen_core::CI_VM_WORKSPACE_TAG;
use snafu::ResultExt;
use tracing::debug;
use tracing::info;
use tracing::warn;

use super::ManagedCiVm;
use super::types::VmState;
use crate::error::CloudHypervisorError;
use crate::error::Result;
use crate::error::{self};

impl ManagedCiVm {
    /// Get the current VM state.
    pub async fn state(&self) -> VmState {
        *self.state.read().await
    }

    /// Get the currently assigned job ID.
    pub async fn current_job(&self) -> Option<String> {
        self.current_job.read().await.clone()
    }

    /// Start the VM and wait for it to be ready.
    pub async fn start(&self) -> Result<()> {
        let current = self.state().await;
        if current != VmState::Stopped {
            return Err(CloudHypervisorError::InvalidState {
                vm_id: self.id.clone(),
                state: current.to_string(),
                operation: "start".to_string(),
            });
        }

        *self.state.write().await = VmState::Creating;

        // Ensure state directory exists
        tokio::fs::create_dir_all(&self.config.state_dir).await.context(error::WorkspaceSetupSnafu)?;

        // Start virtiofsd for Nix store (read-only, static content - use caching)
        info!(vm_id = %self.id, "starting virtiofsd for Nix store");
        let nix_store_virtiofsd = self.start_virtiofsd("/nix/store", CI_VM_NIX_STORE_TAG, "auto").await?;
        *self.virtiofsd_nix_store.write().await = Some(nix_store_virtiofsd);

        // Start workspace filesystem backend: in-process VirtioFS daemon backed by AspenFs (KV + blobs).
        {
            use std::sync::Arc;

            // Get cluster ticket for AspenFs client connection
            let ticket_str =
                self.config.get_cluster_ticket().ok_or_else(|| CloudHypervisorError::StartVirtioFsDaemon {
                    reason: "no cluster ticket configured - cannot create AspenFs client".to_string(),
                })?;

            // If bridge socket address is configured, inject it into the ticket
            let final_ticket = if let Some(bridge_addr) = self.config.bridge_socket_addr() {
                match aspen_ticket::AspenClusterTicket::deserialize(&ticket_str) {
                    Ok(mut ticket) => {
                        info!(
                            vm_id = %self.id,
                            bridge_addr = %bridge_addr,
                            "injecting bridge address into workspace ticket"
                        );
                        ticket.inject_direct_addr(bridge_addr);
                        ticket.serialize()
                    }
                    Err(e) => {
                        warn!(
                            vm_id = %self.id,
                            error = %e,
                            "failed to parse ticket for bridge injection, using original"
                        );
                        ticket_str.clone()
                    }
                }
            } else {
                ticket_str.clone()
            };

            // Create FuseSyncClient from ticket (blocking - runs inside spawn_blocking)
            let client: aspen_fuse::SharedClient = Arc::new(
                tokio::task::spawn_blocking(move || aspen_fuse::FuseSyncClient::from_ticket(&final_ticket))
                    .await
                    .map_err(|e| CloudHypervisorError::StartVirtioFsDaemon {
                        reason: format!("spawn_blocking join error: {e}"),
                    })?
                    .map_err(|e| CloudHypervisorError::StartVirtioFsDaemon {
                        reason: format!("failed to create AspenFs client: {e}"),
                    })?,
            );

            // Create AspenFs with per-VM key prefix for namespace isolation
            let prefix = format!("ci/workspaces/{}/", self.id);
            let fs = aspen_fuse::AspenFs::with_prefix(0, 0, client.clone(), prefix);

            // Remove stale socket before spawning daemon
            let socket_path = self.config.virtiofs_socket_path(&self.id, CI_VM_WORKSPACE_TAG);
            let _ = tokio::fs::remove_file(&socket_path).await;

            info!(vm_id = %self.id, socket = ?socket_path, "spawning AspenFs VirtioFS daemon for workspace");
            let handle = aspen_fuse::spawn_virtiofs_daemon(&socket_path, fs)
                .map_err(|e| CloudHypervisorError::StartVirtioFsDaemon { reason: format!("{e}") })?;

            *self.virtiofs_workspace_handle.write().await = Some(handle);
            *self.workspace_client.write().await = Some(client);
        }

        // Write cluster ticket to workspace for VM's aspen-node to read.
        // The VM runs in worker-only mode and needs the ticket to join the cluster.
        // The ticket is read from config or from a file (since the file may be written
        // after CloudHypervisorWorker is created but before VMs start).
        if let Some(ticket_str) = self.config.get_cluster_ticket() {
            let ticket_path = self.config.cluster_ticket_path(&self.id);

            // If bridge socket address is configured, inject it into the ticket
            // so VMs can reach the host's Iroh endpoint via the bridge IP.
            let final_ticket = if let Some(bridge_addr) = self.config.bridge_socket_addr() {
                // Parse ticket, inject bridge address, re-serialize
                match aspen_ticket::AspenClusterTicket::deserialize(&ticket_str) {
                    Ok(mut ticket) => {
                        info!(
                            vm_id = %self.id,
                            bridge_addr = %bridge_addr,
                            "injecting bridge address into VM ticket"
                        );
                        ticket.inject_direct_addr(bridge_addr);
                        ticket.serialize()
                    }
                    Err(e) => {
                        // Fall back to original ticket if parsing fails
                        warn!(
                            vm_id = %self.id,
                            error = %e,
                            "failed to parse ticket for bridge injection, using original"
                        );
                        ticket_str
                    }
                }
            } else {
                ticket_str
            };

            // Write ticket to workspace via the FuseSyncClient to the distributed KV store.
            info!(vm_id = %self.id, ticket_path = %ticket_path.display(), "writing cluster ticket to workspace");
            {
                let ticket_key = format!("ci/workspaces/{}/.aspen-cluster-ticket", self.id);
                let ticket_bytes = final_ticket.as_bytes().to_vec();
                let client = self.workspace_client.read().await;
                if let Some(ref c) = *client {
                    tokio::task::spawn_blocking({
                        let c = c.clone();
                        move || c.write_key(&ticket_key, &ticket_bytes)
                    })
                    .await
                    .map_err(|e| CloudHypervisorError::WorkspaceProvision {
                        reason: format!("spawn_blocking join error: {e}"),
                    })?
                    .map_err(|e| CloudHypervisorError::WorkspaceProvision {
                        reason: format!("failed to write cluster ticket via AspenFs: {e}"),
                    })?;
                }
            }
        } else {
            warn!(
                vm_id = %self.id,
                ticket_file = ?self.config.cluster_ticket_file,
                "no cluster ticket configured - VM will not be able to join cluster"
            );
        }

        // Note: We use tmpfs for /nix/.rw-store inside the VM instead of virtiofs.
        // virtiofs lacks the filesystem features required by overlayfs for its upper layer
        // (see microvm.nix issue #43). The VM config mounts tmpfs at /nix/.rw-store.

        // Wait for all virtiofsd sockets to be ready before starting Cloud Hypervisor.
        // This is critical for nested virtualization scenarios where socket creation
        // may take longer than the default timeout.
        self.wait_for_virtiofsd_sockets().await?;

        // Start Cloud Hypervisor
        info!(vm_id = %self.id, "starting cloud-hypervisor");
        let mut ch_process = self.start_cloud_hypervisor().await?;

        // Extract stderr handle for debugging (before moving process to RwLock)
        let stderr_handle = ch_process.stderr.take();
        *self.process_stderr.write().await = stderr_handle;
        *self.process.write().await = Some(ch_process);

        *self.state.write().await = VmState::Booting;

        // Wait for API socket (with process health monitoring)
        let boot_timeout = Duration::from_millis(CI_VM_BOOT_TIMEOUT_MS);
        self.wait_for_socket_with_health_check(boot_timeout).await?;

        // Boot the VM via API (if not already running)
        // Cloud Hypervisor behavior varies by version:
        // - Some versions create VM in "Created" state, requiring explicit boot
        // - Some versions auto-boot with --kernel CLI args
        // Check state first to handle both cases
        let vm_info = self.api.vm_info().await?;
        if vm_info.state == "Running" {
            info!(vm_id = %self.id, "VM already running (auto-booted)");
        } else {
            info!(vm_id = %self.id, state = %vm_info.state, "sending boot command via API");
            self.api.boot().await?;
        }

        // Wait for VM to be running
        self.wait_for_vm_running(boot_timeout).await?;

        // VM is now running aspen-node --worker-only which will:
        // 1. Read cluster ticket from /workspace/.aspen-cluster-ticket
        // 2. Join the cluster via Iroh
        // 3. Register as a worker for ci_vm jobs
        // No guest agent verification needed - the VM is an autonomous cluster participant.

        *self.state.write().await = VmState::Idle;
        info!(vm_id = %self.id, "VM is running (aspen-node will join cluster autonomously)");

        Ok(())
    }

    /// Assign a job to this VM.
    pub async fn assign(&self, job_id: String) -> Result<()> {
        let current = self.state().await;
        if current != VmState::Idle {
            return Err(CloudHypervisorError::InvalidState {
                vm_id: self.id.clone(),
                state: current.to_string(),
                operation: "assign".to_string(),
            });
        }

        *self.state.write().await = VmState::Assigned;
        *self.current_job.write().await = Some(job_id.clone());

        debug!(vm_id = %self.id, job_id = %job_id, "job assigned");
        Ok(())
    }

    /// Mark the VM as running a job.
    pub async fn mark_running(&self) -> Result<()> {
        let current = self.state().await;
        if current != VmState::Assigned {
            return Err(CloudHypervisorError::InvalidState {
                vm_id: self.id.clone(),
                state: current.to_string(),
                operation: "mark_running".to_string(),
            });
        }

        *self.state.write().await = VmState::Running;
        Ok(())
    }

    /// Release the VM back to idle state after job completion.
    pub async fn release(&self) -> Result<()> {
        let current = self.state().await;
        if current != VmState::Running && current != VmState::Assigned {
            return Err(CloudHypervisorError::InvalidState {
                vm_id: self.id.clone(),
                state: current.to_string(),
                operation: "release".to_string(),
            });
        }

        *self.state.write().await = VmState::Cleanup;

        // Clean workspace: scan and delete all keys under the VM's KV prefix.
        {
            let prefix = format!("ci/workspaces/{}/", self.id);
            let client = self.workspace_client.read().await;
            if let Some(ref c) = *client {
                debug!(vm_id = %self.id, prefix = %prefix, "cleaning workspace KV keys");
                let c = c.clone();
                let prefix_clone = prefix.clone();
                let keys = tokio::task::spawn_blocking(move || c.scan_keys(&prefix_clone, 10_000))
                    .await
                    .map_err(|e| CloudHypervisorError::WorkspaceProvision {
                        reason: format!("spawn_blocking join error: {e}"),
                    })?
                    .map_err(|e| CloudHypervisorError::WorkspaceProvision {
                        reason: format!("failed to scan workspace keys: {e}"),
                    })?;

                for (key, _) in keys {
                    let c = self.workspace_client.read().await.clone();
                    if let Some(c) = c {
                        let key_clone = key.clone();
                        let _ = tokio::task::spawn_blocking(move || c.delete_key(&key_clone)).await;
                    }
                }
                debug!(vm_id = %self.id, "workspace KV keys cleaned");
            }
        }

        let job_id = self.current_job.write().await.take();
        *self.state.write().await = VmState::Idle;

        debug!(vm_id = %self.id, job_id = ?job_id, "VM released to pool");
        Ok(())
    }

    /// Pause the VM (for snapshot).
    pub async fn pause(&self) -> Result<()> {
        let current = self.state().await;
        if current != VmState::Idle {
            return Err(CloudHypervisorError::InvalidState {
                vm_id: self.id.clone(),
                state: current.to_string(),
                operation: "pause".to_string(),
            });
        }

        self.api.pause().await?;
        *self.state.write().await = VmState::Paused;

        debug!(vm_id = %self.id, "VM paused");
        Ok(())
    }

    /// Resume the VM from paused state.
    pub async fn resume(&self) -> Result<()> {
        let current = self.state().await;
        if current != VmState::Paused {
            return Err(CloudHypervisorError::InvalidState {
                vm_id: self.id.clone(),
                state: current.to_string(),
                operation: "resume".to_string(),
            });
        }

        self.api.resume().await?;
        *self.state.write().await = VmState::Idle;

        debug!(vm_id = %self.id, "VM resumed");
        Ok(())
    }

    /// Create a snapshot of the VM.
    pub async fn snapshot(&self, dest: &PathBuf) -> Result<()> {
        let current = self.state().await;
        if current != VmState::Paused {
            return Err(CloudHypervisorError::InvalidState {
                vm_id: self.id.clone(),
                state: current.to_string(),
                operation: "snapshot".to_string(),
            });
        }

        let dest_url = format!("file://{}", dest.display());
        self.api.snapshot(&dest_url).await?;

        info!(vm_id = %self.id, dest = ?dest, "snapshot created");
        Ok(())
    }

    /// Shutdown the VM gracefully.
    pub async fn shutdown(&self) -> Result<()> {
        let current = self.state().await;
        if current == VmState::Stopped {
            return Ok(());
        }

        info!(vm_id = %self.id, "shutting down VM");

        // Try graceful shutdown via API
        if let Err(e) = self.api.shutdown().await {
            warn!(vm_id = %self.id, error = ?e, "graceful shutdown failed, force killing");
        }

        // Wait a bit for graceful shutdown
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Kill processes if still running
        self.kill_processes().await;

        // Clean up socket files
        self.cleanup_sockets().await;

        *self.state.write().await = VmState::Stopped;
        *self.current_job.write().await = None;

        info!(vm_id = %self.id, "VM shutdown complete");
        Ok(())
    }

    /// Get the workspace directory path.
    pub fn workspace_dir(&self) -> PathBuf {
        self.config.workspace_dir(&self.id)
    }

    /// Get the vsock socket path for guest agent communication.
    pub fn vsock_socket_path(&self) -> PathBuf {
        self.config.vsock_socket_path(&self.id)
    }

    /// Get the API client for direct VM control.
    pub fn api(&self) -> &crate::api_client::VmApiClient {
        &self.api
    }
}
