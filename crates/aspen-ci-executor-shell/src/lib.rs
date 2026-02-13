//! Shell/local executor for Aspen CI jobs.
//!
//! This crate provides the `LocalExecutorWorker` and supporting types for executing
//! CI jobs directly on the host without VM isolation. It is suitable for environments
//! that are already isolated (e.g., running inside a dedicated CI VM).
//!
//! # Features
//!
//! - `snix`: Enable SNIX integration for Nix binary cache storage
//! - `nix-cache-proxy`: Enable HTTP/3 cache proxy for Nix substituter support
//!
//! # Main Components
//!
//! - [`LocalExecutorWorker`]: Worker implementation for direct command execution
//! - [`LocalExecutorWorkerConfig`]: Configuration for the worker
//! - [`LocalExecutorPayload`]: Job payload for local executor jobs
//! - [`ResourceLimiter`]: cgroup-based resource isolation for CI jobs
//! - [`Executor`]: Command execution engine with streaming output
//!
//! # Example
//!
//! ```ignore
//! use aspen_ci_executor_shell::{LocalExecutorWorker, LocalExecutorWorkerConfig};
//! use std::path::PathBuf;
//!
//! let config = LocalExecutorWorkerConfig {
//!     workspace_dir: PathBuf::from("/tmp/ci-workspace"),
//!     cleanup_workspaces: true,
//!     ..Default::default()
//! };
//!
//! let worker = LocalExecutorWorker::new(config);
//! ```

pub mod agent;
#[cfg(feature = "nix-cache-proxy")]
pub mod cache_proxy;
pub mod common;
pub mod local_executor;
pub mod resource_limiter;

// Re-export main types at crate root
pub use agent::ExecutionRequest;
pub use agent::ExecutionResult;
pub use agent::Executor;
pub use agent::LogMessage;
// Cache proxy re-exports (feature-gated)
#[cfg(feature = "nix-cache-proxy")]
pub use cache_proxy::CacheProxy;
#[cfg(feature = "nix-cache-proxy")]
pub use cache_proxy::CacheProxyError;
#[cfg(feature = "nix-cache-proxy")]
pub use cache_proxy::NIX_CACHE_ALPN;
pub use common::ArtifactCollectionResult;
pub use common::ArtifactUploadResult;
pub use common::CollectedArtifact;
pub use common::UploadedArtifact;
pub use common::WorkerUtilError;
pub use common::collect_artifacts;
pub use common::create_source_archive;
pub use common::seed_workspace_from_blob;
pub use common::upload_artifacts_to_blob_store;
pub use local_executor::LocalExecutorPayload;
pub use local_executor::LocalExecutorWorker;
pub use local_executor::LocalExecutorWorkerConfig;
pub use local_executor::OutputRef;
pub use resource_limiter::NetworkIsolation;
pub use resource_limiter::ResourceLimiter;
pub use resource_limiter::ResourceLimiterError;
pub use resource_limiter::ResourceLimits;
pub use resource_limiter::create_limiter;
