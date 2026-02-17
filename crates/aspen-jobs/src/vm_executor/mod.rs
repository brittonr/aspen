//! VM-based job execution with Hyperlight micro-VMs and WASM components.
//!
//! This module provides sandboxed job execution using Hyperlight micro-VMs,
//! supporting pre-built binaries, on-demand Nix builds, and WASM Component Model
//! execution via hyperlight-wasm.

#[cfg(feature = "vm-executor")]
mod hyperlight;
#[cfg(feature = "nanvix-executor")]
mod nanvix;
#[cfg(any(feature = "vm-executor", feature = "wasm-component", feature = "nanvix-executor"))]
mod types;
#[cfg(feature = "wasm-component")]
mod wasm_component;
#[cfg(feature = "wasm-component")]
mod wasm_host;

#[cfg(feature = "vm-executor")]
pub use hyperlight::HyperlightWorker;
#[cfg(feature = "nanvix-executor")]
pub use nanvix::NanvixWorker;
#[cfg(any(feature = "vm-executor", feature = "wasm-component", feature = "nanvix-executor"))]
pub use types::JobPayload;
#[cfg(any(feature = "vm-executor", feature = "wasm-component", feature = "nanvix-executor"))]
pub use types::NixBuildOutput;
#[cfg(feature = "wasm-component")]
pub use wasm_component::WasmComponentWorker;
