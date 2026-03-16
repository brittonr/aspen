//! VM restore from golden snapshot.
//!
//! Restores a VM from a golden snapshot instead of cold-booting.
//! Each restored VM ("fork") gets fresh host-side VirtioFS daemons
//! at socket paths matching the golden VM's expected layout.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use aspen_core::CI_VM_NIX_STORE_TAG;
use aspen_core::CI_VM_WORKSPACE_TAG;
use tracing::debug;
use tracing::info;
use tracing::warn;

use super::ManagedCiVm;
use super::types::VmState;
use crate::error::CloudHypervisorError;
use crate::error::Result;
use crate::snapshot::GoldenSnapshot;

/// Inject a bridge socket address into a cluster ticket string.
fn inject_bridge_addr_for_fork(ticket_str: &str, bridge_addr: SocketAddr, fork_id: &str) -> String {
    match aspen_ticket::AspenClusterTicket::deserialize(ticket_str) {
        Ok(mut ticket) => {
            ticket.inject_direct_addr(bridge_addr);
            ticket.serialize()
        }
        Err(e) => {
            warn!(
                fork_id = %fork_id,
                error = %e,
                "failed to parse ticket for bridge injection, using original"
            );
            ticket_str.to_string()
        }
    }
}

impl ManagedCiVm {
    /// Restore this VM from a golden snapshot.
    ///
    /// Instead of cold-booting, this method:
    /// 1. Creates a fork-specific socket directory
    /// 2. Starts fresh host-side virtiofsd (nix store) at fork-specific paths
    /// 3. Starts fresh host-side AspenFs daemon (workspace) with fork-specific KV prefix
    /// 4. Calls Cloud Hypervisor `vm.restore` with `prefault=false`
    /// 5. Runs a VirtioFS health probe to verify the data path
    /// 6. Transitions to Idle
    pub async fn restore_from_snapshot(&self, snapshot: &GoldenSnapshot) -> Result<()> {
        let current = self.state().await;
        if current != VmState::Stopped {
            return Err(CloudHypervisorError::InvalidState {
                vm_id: self.id.clone(),
                state: current.to_string(),
                operation: "restore_from_snapshot".to_string(),
            });
        }

        *self.state.write().await = VmState::Creating;

        if let Err(e) = self.restore_inner(snapshot).await {
            *self.state.write().await = VmState::Error;
            self.kill_processes().await;
            self.cleanup_sockets().await;
            self.cleanup_fork_dir().await;
            return Err(e);
        }

        Ok(())
    }

    /// Inner restore logic, separated so the caller can handle error transitions.
    async fn restore_inner(&self, snapshot: &GoldenSnapshot) -> Result<()> {
        // Ensure state directory exists
        tokio::fs::create_dir_all(&self.config.state_dir)
            .await
            .map_err(|e| CloudHypervisorError::RestoreFailed {
                path: self.config.state_dir.clone(),
                reason: format!("failed to create state directory: {e}"),
            })?;

        // Create fork-specific directory for sockets
        let fork_dir = self.config.fork_dir(&self.id);
        tokio::fs::create_dir_all(&fork_dir).await.map_err(|e| CloudHypervisorError::RestoreFailed {
            path: fork_dir.clone(),
            reason: format!("failed to create fork directory: {e}"),
        })?;

        info!(
            vm_id = %self.id,
            fork_dir = %fork_dir.display(),
            snapshot = %snapshot.dir.display(),
            "restoring VM from golden snapshot"
        );

        // Start host-side virtiofsd for nix store (fresh process, not in snapshot)
        info!(vm_id = %self.id, "starting virtiofsd for nix store (fork)");
        let nix_store_virtiofsd = self.start_virtiofsd("/nix/store", CI_VM_NIX_STORE_TAG, "auto").await?;
        *self.virtiofsd_nix_store.write().await = Some(nix_store_virtiofsd);

        // Start host-side AspenFs workspace daemon with fork-specific KV prefix.
        // Uses the shared workspace client if injected by the pool, avoiding
        // ~25s of Iroh endpoint creation per VM.
        {
            let existing_client = self.workspace_client.read().await.clone();
            let client: aspen_fuse::SharedClient = if let Some(c) = existing_client {
                debug!(vm_id = %self.id, "using shared workspace client from pool for restore");
                c
            } else {
                let ticket_str =
                    self.config.get_cluster_ticket().ok_or_else(|| CloudHypervisorError::RestoreFailed {
                        path: snapshot.dir.clone(),
                        reason: "no cluster ticket configured".to_string(),
                    })?;

                let final_ticket = if let Some(bridge_addr) = self.config.bridge_socket_addr() {
                    inject_bridge_addr_for_fork(&ticket_str, bridge_addr, &self.id)
                } else {
                    ticket_str.clone()
                };

                info!(vm_id = %self.id, "creating new workspace client for restore (no shared client)");
                let new_client: aspen_fuse::SharedClient = Arc::new(
                    tokio::task::spawn_blocking(move || aspen_fuse::FuseSyncClient::from_ticket(&final_ticket))
                        .await
                        .map_err(|e| CloudHypervisorError::RestoreFailed {
                            path: snapshot.dir.clone(),
                            reason: format!("spawn_blocking join error: {e}"),
                        })?
                        .map_err(|e| CloudHypervisorError::RestoreFailed {
                            path: snapshot.dir.clone(),
                            reason: format!("failed to create AspenFs client for fork: {e}"),
                        })?,
                );
                *self.workspace_client.write().await = Some(new_client.clone());
                new_client
            };

            // Fork-specific KV prefix for workspace isolation
            let prefix = format!("ci/workspaces/{}/", self.id);
            let fs = aspen_fuse::AspenFs::with_prefix(0, 0, client.clone(), prefix);

            let socket_path = self.config.virtiofs_socket_path(&self.id, CI_VM_WORKSPACE_TAG);
            let _ = tokio::fs::remove_file(&socket_path).await;

            info!(vm_id = %self.id, socket = ?socket_path, "spawning AspenFs VirtioFS daemon for fork workspace");
            let handle = aspen_fuse::spawn_virtiofs_daemon(&socket_path, fs).map_err(|e| {
                CloudHypervisorError::RestoreFailed {
                    path: snapshot.dir.clone(),
                    reason: format!("failed to start workspace VirtioFS daemon: {e}"),
                }
            })?;

            *self.virtiofs_workspace_handle.write().await = Some(handle);
        }

        // Wait for virtiofsd sockets to be ready before restore
        self.wait_for_virtiofsd_sockets().await?;

        // Remove stale sockets before restore creates them.
        // Cloud Hypervisor binds vsock and API sockets during restore;
        // stale files from previous runs cause "Address already in use".
        for socket in [
            self.config.api_socket_path(&self.id),
            self.config.vsock_socket_path(&self.id),
            self.config.console_socket_path(&self.id),
        ] {
            let _ = tokio::fs::remove_file(&socket).await;
        }

        // Prepare a fork-specific snapshot with updated virtiofs socket paths.
        // Each fork gets its own config.json pointing to its own virtiofsd sockets,
        // while sharing the memory and state files via symlinks.
        let fork_snap_dir = self.prepare_fork_snapshot(snapshot).await?;
        let fork_source_url = format!("file://{}", fork_snap_dir.display());

        // Restore VM from snapshot via Cloud Hypervisor API.
        info!(vm_id = %self.id, source = %fork_source_url, "calling vm.restore");
        self.start_cloud_hypervisor_for_restore().await?;

        // Wait for API socket
        let restore_timeout = Duration::from_millis(self.config.boot_timeout_ms);
        self.wait_for_socket_with_health_check(restore_timeout).await?;

        // Call vm.restore via API
        self.api.restore(&fork_source_url).await.map_err(|e| CloudHypervisorError::RestoreFailed {
            path: snapshot.dir.clone(),
            reason: format!("vm.restore API call failed: {e}"),
        })?;

        info!(vm_id = %self.id, "vm.restore succeeded, running VirtioFS health probe");

        // Post-restore VirtioFS health probe
        self.run_virtiofs_health_probe().await?;

        *self.state.write().await = VmState::Idle;
        info!(vm_id = %self.id, "VM restored from snapshot and ready");

        Ok(())
    }

    /// Start Cloud Hypervisor in restore mode (API socket only, no VM config).
    ///
    /// Cloud Hypervisor needs to be started with just `--api-socket` so we can
    /// call `vm.restore` via the API. The VM config comes from the snapshot.
    async fn start_cloud_hypervisor_for_restore(&self) -> Result<()> {
        use std::process::Stdio;

        use snafu::ResultExt;
        use tokio::process::Command;

        let api_socket = self.config.api_socket_path(&self.id);
        let ch_path = self
            .config
            .cloud_hypervisor_path
            .as_deref()
            .unwrap_or_else(|| std::path::Path::new("cloud-hypervisor"));

        info!(
            vm_id = %self.id,
            api_socket = ?api_socket,
            "starting cloud-hypervisor for restore"
        );

        let mut child = Command::new(ch_path)
            .arg("--api-socket")
            .arg(format!("path={}", api_socket.display()))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .context(crate::error::StartCloudHypervisorSnafu)?;

        let stderr_handle = child.stderr.take();
        *self.process_stderr.write().await = stderr_handle;
        *self.process.write().await = Some(child);

        Ok(())
    }

    /// Run the post-restore VirtioFS health probe.
    ///
    /// Verifies the end-to-end data path by issuing a KV scan through
    /// the fork's workspace client: host daemon → vhost-user → guest driver → KV.
    async fn run_virtiofs_health_probe(&self) -> Result<()> {
        let prefix = format!("ci/workspaces/{}/", self.id);

        let client = self.workspace_client.read().await;
        let client = client.as_ref().ok_or_else(|| CloudHypervisorError::RestoreFailed {
            path: PathBuf::from(&self.id),
            reason: "no workspace client available for health probe".to_string(),
        })?;

        let c = client.clone();
        let vm_id = self.id.clone();
        let probe_result = tokio::task::spawn_blocking(move || {
            debug!(vm_id = %vm_id, prefix = %prefix, "running VirtioFS health probe");
            c.scan_keys(&prefix, 1)
        })
        .await
        .map_err(|e| CloudHypervisorError::RestoreFailed {
            path: PathBuf::from(&self.id),
            reason: format!("health probe spawn_blocking join error: {e}"),
        })?;

        match probe_result {
            Ok(_keys) => {
                info!(vm_id = %self.id, "VirtioFS health probe passed");
                Ok(())
            }
            Err(e) => {
                warn!(vm_id = %self.id, error = %e, "VirtioFS health probe failed");
                Err(CloudHypervisorError::RestoreFailed {
                    path: PathBuf::from(&self.id),
                    reason: format!("VirtioFS health probe failed: {e}"),
                })
            }
        }
    }

    /// Prepare a fork-specific snapshot config with updated virtiofs socket paths.
    ///
    /// Cloud Hypervisor's `vm.restore` reconnects to the virtiofs socket paths
    /// stored in the snapshot's `config.json`. The original VM used paths like
    /// `{state_dir}/ci-n99-vm0-virtiofs-nix-store.sock`, but this fork's
    /// virtiofsd processes listen on `{state_dir}/ci-n99-vm1-virtiofs-nix-store.sock`.
    ///
    /// We create a fork-specific copy of the snapshot with an updated `config.json`
    /// so each restore uses the correct socket paths without race conditions.
    async fn prepare_fork_snapshot(&self, snapshot: &GoldenSnapshot) -> Result<PathBuf> {
        let fork_snap_dir = self.config.fork_dir(&self.id).join("snapshot");
        tokio::fs::create_dir_all(&fork_snap_dir).await.map_err(|e| CloudHypervisorError::RestoreFailed {
            path: fork_snap_dir.clone(),
            reason: format!("failed to create fork snapshot dir: {e}"),
        })?;

        // Read original config
        let config_path = snapshot.dir.join("config.json");
        let mut config_str =
            tokio::fs::read_to_string(&config_path).await.map_err(|e| CloudHypervisorError::RestoreFailed {
                path: config_path.clone(),
                reason: format!("failed to read snapshot config: {e}"),
            })?;

        // Replace all socket paths that reference the original VM ID with fork-specific paths.
        // This covers virtiofs sockets, vsock sockets, and any other per-VM socket paths.
        // The snapshot config embeds the original VM's socket paths; we need to rewrite them
        // to this fork's paths so Cloud Hypervisor connects to the correct sockets.
        let original_vm0_id = self.config.generate_vm_id(0);
        if original_vm0_id != self.id {
            info!(
                vm_id = %self.id,
                original_vm_id = %original_vm0_id,
                "rewriting all socket paths from original VM to fork"
            );
            config_str = config_str.replace(&original_vm0_id, &self.id);
        }

        // Write the modified config to the fork snapshot dir
        let fork_config = fork_snap_dir.join("config.json");
        tokio::fs::write(&fork_config, &config_str).await.map_err(|e| CloudHypervisorError::RestoreFailed {
            path: fork_config.clone(),
            reason: format!("failed to write fork config: {e}"),
        })?;

        // Symlink the memory and state files (they're read-only, safe to share)
        for filename in &["memory-ranges", "state.json"] {
            let src = snapshot.dir.join(filename);
            let dst = fork_snap_dir.join(filename);
            if src.exists() {
                let _ = tokio::fs::remove_file(&dst).await;
                tokio::fs::symlink(&src, &dst).await.map_err(|e| CloudHypervisorError::RestoreFailed {
                    path: dst.clone(),
                    reason: format!("failed to symlink {filename}: {e}"),
                })?;
            }
        }

        Ok(fork_snap_dir)
    }

    /// Clean up fork-specific directory and its contents.
    pub(super) async fn cleanup_fork_dir(&self) {
        let fork_dir = self.config.fork_dir(&self.id);
        if fork_dir.exists()
            && let Err(e) = tokio::fs::remove_dir_all(&fork_dir).await
        {
            warn!(vm_id = %self.id, dir = %fork_dir.display(), error = %e, "failed to clean up fork directory");
        }
    }
}
