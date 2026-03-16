//! VM cleanup: process termination, socket file removal, and fork directory cleanup.

use std::sync::Arc;

use aspen_core::CI_VM_NIX_STORE_TAG;
use aspen_core::CI_VM_WORKSPACE_TAG;
use tracing::debug;
use tracing::warn;

use super::ManagedCiVm;

impl ManagedCiVm {
    /// Kill all VM-related processes.
    pub(super) async fn kill_processes(&self) {
        // Kill cloud-hypervisor
        if let Some(mut process) = self.process.write().await.take()
            && let Err(e) = process.kill().await
        {
            warn!("failed to kill cloud-hypervisor process: {e}");
        }

        // Kill virtiofsd for nix store
        if let Some(mut process) = self.virtiofsd_nix_store.write().await.take()
            && let Err(e) = process.kill().await
        {
            warn!("failed to kill virtiofsd nix-store process: {e}");
        }

        // Shutdown workspace virtiofs backend
        if let Some(handle) = self.virtiofs_workspace_handle.write().await.take()
            && let Err(e) = handle.shutdown()
        {
            warn!("failed to shutdown AspenFs workspace daemon: {e}");
        }
        // Drop the workspace client reference. When using a shared client from the pool,
        // this just decrements the Arc refcount. Only the last reference triggers
        // the actual Runtime teardown, which must happen off the async thread.
        if let Some(client) = self.workspace_client.write().await.take()
            && Arc::strong_count(&client) == 1
        {
            tokio::task::spawn_blocking(move || drop(client));
        }
    }

    /// Clean up socket files.
    pub(super) async fn cleanup_sockets(&self) {
        let sockets = [
            self.config.api_socket_path(&self.id),
            self.config.vsock_socket_path(&self.id),
            self.config.virtiofs_socket_path(&self.id, CI_VM_NIX_STORE_TAG),
            self.config.virtiofs_socket_path(&self.id, CI_VM_WORKSPACE_TAG),
            self.config.console_socket_path(&self.id),
        ];

        for socket in &sockets {
            if let Err(e) = tokio::fs::remove_file(socket).await {
                // ENOENT is fine — socket may not have been created
                if e.kind() != std::io::ErrorKind::NotFound {
                    warn!(path = %socket.display(), "failed to remove socket file: {e}");
                }
            }
        }
    }

    /// Full cleanup for a restored fork: processes, sockets, and fork directory.
    ///
    /// This should be called instead of (or in addition to) the standard cleanup
    /// when destroying a snapshot-restored VM. It ensures no leaked socket files,
    /// processes, or COW overlay state remains.
    pub(crate) async fn full_fork_cleanup(&self) {
        debug!(vm_id = %self.id, "running full fork cleanup");

        self.kill_processes().await;
        self.cleanup_sockets().await;
        self.cleanup_fork_dir().await;
    }
}
