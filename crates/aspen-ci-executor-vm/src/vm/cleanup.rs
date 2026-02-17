//! VM cleanup: process termination and socket file removal.

use aspen_core::CI_VM_NIX_STORE_TAG;
use aspen_core::CI_VM_WORKSPACE_TAG;
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
        #[cfg(not(feature = "aspen-workspace-fs"))]
        if let Some(mut process) = self.virtiofsd_workspace.write().await.take()
            && let Err(e) = process.kill().await
        {
            warn!("failed to kill virtiofsd workspace process: {e}");
        }

        #[cfg(feature = "aspen-workspace-fs")]
        {
            if let Some(handle) = self.virtiofs_workspace_handle.write().await.take()
                && let Err(e) = handle.shutdown()
            {
                warn!("failed to shutdown AspenFs workspace daemon: {e}");
            }
            // Drop the shared client
            let _ = self.workspace_client.write().await.take();
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
                warn!(path = %socket.display(), "failed to remove socket file: {e}");
            }
        }
    }
}
