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
mod monitoring;
mod provisioning;
mod types;

use tokio::process::Child;
use tokio::process::ChildStderr;
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

    /// Virtiofsd process for workspace.
    pub(super) virtiofsd_workspace: RwLock<Option<Child>>,

    /// Currently assigned job ID.
    pub(super) current_job: RwLock<Option<String>>,

    /// VM index (for networking).
    pub(super) vm_index: u32,
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
            virtiofsd_workspace: RwLock::new(None),
            current_job: RwLock::new(None),
            vm_index,
        }
    }
}

impl Drop for ManagedCiVm {
    fn drop(&mut self) {
        // Processes are killed on drop due to kill_on_drop(true)
        // Socket cleanup happens in shutdown()
    }
}
