//! CI build workers.
//!
//! This module provides specialized workers for CI/CD job execution:
//!
//! - `NixBuildWorker`: Builds Nix flake outputs
//! - Uses existing `ShellCommandWorker` for shell jobs
//! - Uses existing `HyperlightWorker` for VM jobs

mod nix_build;

pub use nix_build::NixBuildPayload;
pub use nix_build::NixBuildWorker;
pub use nix_build::NixBuildWorkerConfig;
