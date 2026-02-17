//! VM-based job execution with Hyperlight micro-VMs and WASM components.
//!
//! This module provides sandboxed job execution using Hyperlight micro-VMs,
//! supporting pre-built binaries, on-demand Nix builds, and WASM Component Model
//! execution via hyperlight-wasm.

#[cfg(feature = "plugins-vm")]
mod hyperlight;
#[cfg(feature = "plugins-nanvix")]
mod nanvix;
#[cfg(any(feature = "plugins-vm", feature = "plugins-wasm", feature = "plugins-nanvix"))]
mod types;
#[cfg(feature = "plugins-wasm")]
mod wasm_component;
#[cfg(feature = "plugins-wasm")]
mod wasm_host;

#[cfg(feature = "plugins-vm")]
pub use hyperlight::HyperlightWorker;
#[cfg(feature = "plugins-nanvix")]
pub use nanvix::NanvixWorker;
#[cfg(any(feature = "plugins-vm", feature = "plugins-wasm", feature = "plugins-nanvix"))]
pub use types::JobPayload;
#[cfg(any(feature = "plugins-vm", feature = "plugins-wasm", feature = "plugins-nanvix"))]
pub use types::NixBuildOutput;
#[cfg(feature = "plugins-wasm")]
pub use wasm_component::WasmComponentWorker;
