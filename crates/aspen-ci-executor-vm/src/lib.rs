//! Cloud Hypervisor VM executor for Aspen CI jobs.
//!
//! This crate provides the `CloudHypervisorWorker` and supporting types for executing
//! CI jobs in isolated Cloud Hypervisor microVMs.
//!
//! # Architecture
//!
//! ```text
//!                     Job Queue (Raft KV)
//!                            |
//!                            v
//! +------------------------------------------------------------------+
//! |                  CloudHypervisorWorker                           |
//! |  (implements Worker trait, manages VM lifecycle via REST API)    |
//! +------------------------------+-----------------------------------+
//!                                |
//!                +---------------+---------------+
//!                v               v               v
//!           +--------+     +--------+      +--------+
//!           |VmPool  |---->|VmPool  |----> |VmPool  |  (warm pool)
//!           | idle   |     |assigned|      |running |
//!           +--------+     +--------+      +--------+
//!                |               |               |
//!                v               v               v
//!      +-------------------------------------------------------------+
//!      |              Cloud Hypervisor REST API                       |
//!      |  (Unix socket: /tmp/aspen-ci-vm-{id}-api.sock)              |
//!      +------------------------------+------------------------------+
//!                                     |
//!      +------------------------------+------------------------------+
//!      |                    MicroVM Guest                             |
//!      |  +-------------------------------------------------------+  |
//!      |  |              Guest Agent (aspen-ci-agent)              |  |
//!      |  |  - Listens on vsock port 5000                          |  |
//!      |  |  - Receives ExecutionRequest                           |  |
//!      |  |  - Executes command in /workspace (virtiofs)           |  |
//!      |  |  - Streams logs back via vsock                         |  |
//!      |  |  - Returns ExecutionResult                             |  |
//!      |  +-------------------------------------------------------+  |
//!      |                                                              |
//!      |  VirtioFS mounts:                                           |
//!      |    /nix/.ro-store -> host /nix/store (read-only)            |
//!      |    /workspace     -> job working directory (read-write)     |
//!      +--------------------------------------------------------------+
//! ```
//!
//! # Main Components
//!
//! - [`CloudHypervisorWorker`]: Worker implementation that manages VM pool lifecycle
//! - [`CloudHypervisorWorkerConfig`]: Configuration for VM resources, paths, and networking
//! - [`VmPool`]: Warm pool of pre-booted VMs for fast job startup
//! - [`ManagedCiVm`]: State machine for individual VM lifecycle
//! - [`VmApiClient`]: REST API client for Cloud Hypervisor (Unix socket)
//! - [`CloudHypervisorPayload`]: Job payload for VM execution
//! - [`NetworkMode`]: Network configuration options (Tap, TapWithHelper, None)
//!
//! # Features
//!
//! This crate requires the HTTP dependencies (hyper, http-body-util) for the Cloud
//! Hypervisor REST API communication over Unix sockets.

mod api_client;
mod config;
mod error;
mod payload;
mod pool;
mod vm;
mod worker;

// Re-export main types at crate root
pub use api_client::VmApiClient;
// Re-export common utilities from aspen-ci-executor-shell for convenience
pub use aspen_ci_executor_shell::ArtifactCollectionResult;
pub use aspen_ci_executor_shell::ArtifactUploadResult;
pub use aspen_ci_executor_shell::CollectedArtifact;
pub use aspen_ci_executor_shell::UploadedArtifact;
pub use aspen_ci_executor_shell::collect_artifacts;
pub use aspen_ci_executor_shell::create_source_archive;
pub use aspen_ci_executor_shell::seed_workspace_from_blob;
pub use aspen_ci_executor_shell::upload_artifacts_to_blob_store;
pub use config::CloudHypervisorWorkerConfig;
pub use error::CloudHypervisorError;
pub use payload::CloudHypervisorPayload;
pub use payload::NetworkMode;
pub use pool::PoolStatus;
pub use pool::VmPool;
pub use vm::ManagedCiVm;
pub use vm::SharedVm;
pub use vm::VmState;
pub use worker::CloudHypervisorWorker;
