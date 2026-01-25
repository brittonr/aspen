//! CI build workers.
//!
//! This module provides specialized workers for CI/CD job execution:
//!
//! - `NixBuildWorker`: Builds Nix flake outputs
//! - `ResourceLimiter`: cgroup-based resource isolation for CI jobs
//! - Uses existing `ShellCommandWorker` for shell jobs
//! - Uses existing `HyperlightWorker` for VM jobs

mod nix_build;
mod resource_limiter;

pub use nix_build::NixBuildPayload;
pub use nix_build::NixBuildWorker;
pub use nix_build::NixBuildWorkerConfig;
pub use resource_limiter::ResourceLimiter;
pub use resource_limiter::ResourceLimiterError;
pub use resource_limiter::ResourceLimits;
pub use resource_limiter::create_limiter;
