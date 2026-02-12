//! CI build workers.
//!
//! This module provides specialized workers for CI/CD job execution:
//!
//! - `NixBuildWorker`: Builds Nix flake outputs
//! - `ResourceLimiter`: cgroup-based resource isolation for CI jobs
//! - `CacheProxy`: HTTP-to-Iroh proxy for Nix binary cache substitution (requires `nix-cache-proxy`
//!   feature)
//! - `CloudHypervisorWorker`: Full VM isolation via Cloud Hypervisor microVMs (requires
//!   `cloud-hypervisor` feature)
//! - `LocalExecutorWorker`: Direct process execution (no VM overhead)
//! - Uses existing `ShellCommandWorker` for shell jobs
//! - Uses existing `HyperlightWorker` for VM jobs

#[cfg(feature = "nix-cache-proxy")]
mod cache_proxy;
#[cfg(feature = "cloud-hypervisor")]
pub mod cloud_hypervisor;
pub mod common;
mod local_executor;
#[cfg(feature = "snix")]
mod nix_build;
mod payload;
mod resource_limiter;

#[cfg(feature = "nix-cache-proxy")]
pub use cache_proxy::CacheProxy;
#[cfg(feature = "nix-cache-proxy")]
pub use cache_proxy::CacheProxyError;
#[cfg(feature = "nix-cache-proxy")]
pub use cache_proxy::NIX_CACHE_ALPN;
#[cfg(feature = "cloud-hypervisor")]
pub use cloud_hypervisor::CloudHypervisorWorker;
#[cfg(feature = "cloud-hypervisor")]
pub use cloud_hypervisor::CloudHypervisorWorkerConfig;
// Common utilities (always available, no HTTP deps)
pub use common::ArtifactCollectionResult;
pub use common::ArtifactUploadResult;
pub use common::CollectedArtifact;
pub use common::UploadedArtifact;
pub use common::collect_artifacts;
pub use common::create_source_archive;
pub use common::seed_workspace_from_blob;
pub use common::upload_artifacts_to_blob_store;
pub use local_executor::LocalExecutorPayload;
pub use local_executor::LocalExecutorWorker;
pub use local_executor::LocalExecutorWorkerConfig;
#[cfg(feature = "snix")]
pub use nix_build::NixBuildPayload;
#[cfg(feature = "snix")]
pub use nix_build::NixBuildWorker;
#[cfg(feature = "snix")]
pub use nix_build::NixBuildWorkerConfig;
// CloudHypervisorPayload and NetworkMode are always available (no HTTP deps)
// CloudHypervisorWorker and CloudHypervisorWorkerConfig require cloud-hypervisor feature
pub use payload::CloudHypervisorPayload;
pub use payload::NetworkMode;
pub use resource_limiter::ResourceLimiter;
pub use resource_limiter::ResourceLimiterError;
pub use resource_limiter::ResourceLimits;
pub use resource_limiter::create_limiter;
