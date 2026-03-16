//! ManagedCiVm - State machine for Cloud Hypervisor CI VMs.
//!
//! This module manages the lifecycle of a single Cloud Hypervisor microVM
//! used as an ephemeral CI worker. It handles:
//!
//! - VM creation and boot
//! - Cluster ticket provisioning for worker mode
//! - State transitions (Idle -> Assigned -> Running -> Cleanup -> Idle)
//! - Graceful shutdown and cleanup
//!
//! # Architecture
//!
//! VMs run `aspen-node --worker-only` which:
//! 1. Reads cluster ticket from `/workspace/.aspen-cluster-ticket`
//! 2. Joins the Aspen cluster via Iroh
//! 3. Registers as a worker for `ci_vm` jobs
//! 4. Executes jobs and uploads artifacts to SNIX
//!
//! The VM is an ephemeral cluster participant - it has full access to SNIX
//! for artifact upload but does not participate in Raft consensus.

mod cleanup;
mod lifecycle;
pub mod memory;
mod monitoring;
mod provisioning;
mod restore;
mod types;

use std::sync::Arc;

use tokio::process::Child;
use tokio::process::ChildStderr;
use tokio::sync::OwnedSemaphorePermit;
use tokio::sync::RwLock;
pub use types::SharedVm;
pub use types::VmState;

use crate::api_client::VmApiClient;
use crate::config::CloudHypervisorWorkerConfig;

/// A managed Cloud Hypervisor CI VM.
pub struct ManagedCiVm {
    /// Unique VM identifier.
    pub id: String,

    /// VM configuration.
    pub(super) config: CloudHypervisorWorkerConfig,

    /// Current VM state.
    pub(super) state: RwLock<VmState>,

    /// API client for Cloud Hypervisor.
    pub(super) api: VmApiClient,

    /// Cloud Hypervisor process handle.
    pub(super) process: RwLock<Option<Child>>,

    /// Cloud Hypervisor stderr handle (for debugging startup failures).
    pub(super) process_stderr: RwLock<Option<ChildStderr>>,

    /// Virtiofsd process for Nix store.
    pub(super) virtiofsd_nix_store: RwLock<Option<Child>>,

    /// In-process VirtioFS daemon handle for workspace (AspenFs-backed).
    pub(super) virtiofs_workspace_handle: RwLock<Option<aspen_fuse::VirtioFsDaemonHandle>>,

    /// Shared client for workspace KV operations (used during release to clean up keys).
    pub(super) workspace_client: RwLock<Option<aspen_fuse::SharedClient>>,

    /// Currently assigned job ID.
    pub(super) current_job: RwLock<Option<String>>,

    /// VM index (for networking).
    pub(super) vm_index: u32,

    /// Pool semaphore permit — held while this VM exists.
    /// Automatically released on drop, preventing semaphore leaks even
    /// if the VM is dropped without going through `destroy_vm()`.
    pub(crate) pool_permit: RwLock<Option<OwnedSemaphorePermit>>,
}

impl ManagedCiVm {
    /// Create a new managed VM (not yet started).
    pub fn new(config: CloudHypervisorWorkerConfig, vm_index: u32) -> Self {
        let id = config.generate_vm_id(vm_index);
        let api_socket = config.api_socket_path(&id);

        Self {
            id: id.clone(),
            config,
            state: RwLock::new(VmState::Stopped),
            api: VmApiClient::new(api_socket),
            process: RwLock::new(None),
            process_stderr: RwLock::new(None),
            virtiofsd_nix_store: RwLock::new(None),
            virtiofs_workspace_handle: RwLock::new(None),
            workspace_client: RwLock::new(None),
            current_job: RwLock::new(None),
            vm_index,
            pool_permit: RwLock::new(None),
        }
    }

    /// Set the workspace client from a pool-level shared client.
    ///
    /// When provided, the VM reuses the shared Iroh endpoint instead of
    /// creating its own, avoiding ~25s of relay discovery overhead.
    pub async fn set_shared_workspace_client(&self, client: aspen_fuse::SharedClient) {
        *self.workspace_client.write().await = Some(client);
    }
}

impl Drop for ManagedCiVm {
    fn drop(&mut self) {
        // Processes are killed on drop due to kill_on_drop(true)
        // Socket cleanup happens in shutdown()

        // FuseSyncClient holds an internal tokio Runtime that cannot be dropped
        // inside an async context. Only the *last* Arc reference triggers the
        // actual Runtime drop, so check strong_count before spawning a thread.
        if let Some(client) = self.workspace_client.get_mut().take()
            && Arc::strong_count(&client) == 1
        {
            // Last reference — dropping will destroy the tokio Runtime.
            // Use std::thread::spawn rather than tokio::task::spawn_blocking
            // because Drop can run outside any tokio context.
            std::thread::spawn(move || drop(client));
        }
    }
}
