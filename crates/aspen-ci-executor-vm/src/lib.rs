//! Cloud Hypervisor VM executor for Aspen CI jobs.
//!
//! This crate provides the `CloudHypervisorWorker` and supporting types for
//! managing a pool of Cloud Hypervisor microVMs used as ephemeral CI workers.
//!
//! # Architecture
//!
//! VMs run `aspen-node --worker-only` and autonomously join the Aspen cluster
//! via Iroh. Jobs are routed through the normal Raft-backed job queue — the
//! `CloudHypervisorWorker` only manages VM lifecycle, not job execution.
//!
//! ```text
//! +-------------------------------------------------------------+
//! |                    Host Node                                |
//! |  +-------------------------------------------------------+  |
//! |  |            CloudHypervisorWorker                      |  |
//! |  |  (VM Pool Manager - maintains warm VMs)               |  |
//! |  +-------------------------------------------------------+  |
//! |           |                    |                    |       |
//! |     +-----+-----+        +-----+-----+       +-----+-----+  |
//! |     |   VM 0    |        |   VM 1    |       |   VM N    |  |
//! |     | aspen-node|        | aspen-node|       | aspen-node|  |
//! |     | (worker)  |        | (worker)  |       | (worker)  |  |
//! |     +-----------+        +-----------+       +-----------+  |
//! |           |                    |                    |       |
//! |           +--------------------+--------------------+       |
//! |                                |                            |
//! |                    Iroh Cluster Connection                  |
//! +-------------------------------|-----------------------------+
//! |                               |                             |
//! |                  +------------+-----------+                 |
//! |                  |     Aspen Cluster      |                 |
//! |                  |  (Job Queue + SNIX)    |                 |
//! |                  +------------------------+                 |
//! |                                                             |
//! |  VirtioFS mounts per VM:                                    |
//! |    /nix/.ro-store -> host /nix/store (read-only, virtiofsd) |
//! |    /workspace     -> AspenFs KV (in-process VirtioFS)       |
//! +-------------------------------------------------------------+
//! ```
//!
//! # Main Components
//!
//! - [`CloudHypervisorWorker`]: VM pool lifecycle manager (does NOT execute jobs)
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
