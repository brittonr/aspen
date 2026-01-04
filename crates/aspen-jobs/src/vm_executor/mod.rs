//! VM-based job execution with Hyperlight micro-VMs.
//!
//! This module provides sandboxed job execution using Hyperlight micro-VMs,
//! supporting both pre-built binaries and on-demand Nix builds.

#[cfg(feature = "vm-executor")]
mod hyperlight;
#[cfg(feature = "vm-executor")]
mod types;

#[cfg(feature = "vm-executor")]
pub use hyperlight::HyperlightWorker;
#[cfg(feature = "vm-executor")]
pub use types::JobPayload;
#[cfg(feature = "vm-executor")]
pub use types::NixBuildOutput;
