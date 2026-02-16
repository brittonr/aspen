//! VM-based job execution with Hyperlight micro-VMs and WASM components.
//!
//! This module provides sandboxed job execution using Hyperlight micro-VMs,
//! supporting pre-built binaries, on-demand Nix builds, and WASM Component Model
//! execution via hyperlight-wasm.

#[cfg(feature = "vm-executor")]
mod hyperlight;
#[cfg(any(feature = "vm-executor", feature = "wasm-component"))]
mod types;
#[cfg(feature = "wasm-component")]
mod wasm_component;
// Host bindings are defined but not yet called; suppress dead_code until
// hyperlight-wasm sandbox integration is wired up.
#[cfg(feature = "wasm-component")]
#[allow(dead_code, unused_imports)]
mod wasm_host;

#[cfg(feature = "vm-executor")]
pub use hyperlight::HyperlightWorker;
#[cfg(any(feature = "vm-executor", feature = "wasm-component"))]
pub use types::JobPayload;
#[cfg(any(feature = "vm-executor", feature = "wasm-component"))]
pub use types::NixBuildOutput;
#[cfg(feature = "wasm-component")]
pub use wasm_component::WasmComponentWorker;
