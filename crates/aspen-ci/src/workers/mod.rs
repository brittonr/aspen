//! CI build workers.
//!
//! This module provides specialized workers for CI/CD job execution:
//!
//! - `NixBuildWorker`: Builds Nix flake outputs
//! - `ResourceLimiter`: cgroup-based resource isolation for CI jobs
//! - `CacheProxy`: HTTP-to-Iroh proxy for Nix binary cache substitution
//! - `CloudHypervisorWorker`: Full VM isolation via Cloud Hypervisor microVMs
//! - `LocalExecutorWorker`: Direct process execution (no VM overhead)
//! - Uses existing `ShellCommandWorker` for shell jobs
//! - Uses existing `HyperlightWorker` for VM jobs

mod cache_proxy;
pub mod cloud_hypervisor;
mod local_executor;
#[cfg(feature = "snix")]
mod nix_build;
mod resource_limiter;

pub use cache_proxy::CacheProxy;
pub use cache_proxy::CacheProxyError;
pub use cache_proxy::NIX_CACHE_ALPN;
pub use cloud_hypervisor::CloudHypervisorPayload;
pub use cloud_hypervisor::CloudHypervisorWorker;
pub use cloud_hypervisor::CloudHypervisorWorkerConfig;
pub use cloud_hypervisor::NetworkMode;
pub use local_executor::LocalExecutorPayload;
pub use local_executor::LocalExecutorWorker;
pub use local_executor::LocalExecutorWorkerConfig;
#[cfg(feature = "snix")]
pub use nix_build::NixBuildPayload;
#[cfg(feature = "snix")]
pub use nix_build::NixBuildWorker;
#[cfg(feature = "snix")]
pub use nix_build::NixBuildWorkerConfig;
pub use resource_limiter::ResourceLimiter;
pub use resource_limiter::ResourceLimiterError;
pub use resource_limiter::ResourceLimits;
pub use resource_limiter::create_limiter;
