//! Cloud Hypervisor-based CI worker.
//!
//! This module provides a Worker implementation that manages Cloud Hypervisor
//! microVMs for isolated CI job execution. It builds on the existing dogfood
//! infrastructure (`scripts/dogfood-vm.sh`, `nix/vms/dogfood-node.nix`).
//!
//! ## Architecture
//!
//! ```text
//!                     Job Queue (Raft KV)
//!                            │
//!                            ▼
//! ┌──────────────────────────────────────────────────────────────────┐
//! │                  CloudHypervisorWorker                            │
//! │  (implements Worker trait, manages VM lifecycle via REST API)     │
//! └─────────────────────────────┬────────────────────────────────────┘
//!                               │
//!               ┌───────────────┼───────────────┐
//!               ▼               ▼               ▼
//!          ┌────────┐     ┌────────┐      ┌────────┐
//!          │VmPool  │────▶│VmPool  │────▶ │VmPool  │  (warm pool)
//!          │ idle   │     │assigned│      │running │
//!          └────────┘     └────────┘      └────────┘
//!               │               │               │
//!               ▼               ▼               ▼
//!     ┌─────────────────────────────────────────────────────────────┐
//!     │              Cloud Hypervisor REST API                       │
//!     │  (Unix socket: /tmp/aspen-ci-vm-{id}-api.sock)              │
//!     └─────────────────────────────┬───────────────────────────────┘
//!                                   │
//!     ┌─────────────────────────────┼───────────────────────────────┐
//!     │                    MicroVM Guest                             │
//!     │  ┌─────────────────────────────────────────────────────┐    │
//!     │  │              Guest Agent (aspen-ci-agent)            │    │
//!     │  │  - Listens on vsock port 5000                        │    │
//!     │  │  - Receives ExecutionRequest                         │    │
//!     │  │  - Executes command in /workspace (virtiofs)         │    │
//!     │  │  - Streams logs back via vsock                       │    │
//!     │  │  - Returns ExecutionResult                           │    │
//!     │  └─────────────────────────────────────────────────────┘    │
//!     │                                                              │
//!     │  VirtioFS mounts:                                           │
//!     │    /nix/.ro-store -> host /nix/store (read-only)            │
//!     │    /workspace     -> job working directory (read-write)     │
//!     └──────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Components
//!
//! - `api_client`: REST API client for Cloud Hypervisor (Unix socket)
//! - `config`: Configuration with Tiger Style bounds
//! - `error`: snafu error types
//! - `vm`: ManagedCiVm state machine
//! - `pool`: Warm VM pool management
//! - `executor`: Job execution via vsock to guest agent
//! - `worker`: Worker trait implementation

mod api_client;
mod config;
mod error;
mod pool;
mod vm;
mod worker;

pub use api_client::VmApiClient;
pub use config::CloudHypervisorWorkerConfig;
pub use error::CloudHypervisorError;
pub use pool::PoolStatus;
pub use pool::VmPool;
pub use vm::ManagedCiVm;
pub use vm::SharedVm;
pub use vm::VmState;
pub use worker::CloudHypervisorWorker;

// Re-export common utilities for backwards compatibility
pub use super::common::ArtifactCollectionResult;
pub use super::common::ArtifactUploadResult;
pub use super::common::CollectedArtifact;
pub use super::common::UploadedArtifact;
pub use super::common::collect_artifacts;
pub use super::common::create_source_archive;
pub use super::common::seed_workspace_from_blob;
pub use super::common::upload_artifacts_to_blob_store;
// CloudHypervisorPayload is re-exported from the common payload module
pub use super::payload::CloudHypervisorPayload;
// NetworkMode is re-exported from the common payload module
pub use super::payload::NetworkMode;
