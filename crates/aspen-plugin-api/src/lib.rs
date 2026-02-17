//! Shared types and constants for the Aspen WASM plugin system.
//!
//! This crate defines the API boundary between native host code and WASM
//! guest plugins. Both sides depend on these types to ensure a stable
//! serialization contract.

pub mod manifest;

pub use manifest::PluginInfo;
pub use manifest::PluginManifest;

/// Maximum priority value for WASM plugins.
pub const MAX_PLUGIN_PRIORITY: u32 = 999;

/// Minimum priority value for WASM plugins (ensures they run after native handlers).
pub const MIN_PLUGIN_PRIORITY: u32 = 900;

/// KV key prefix for plugin manifests in the cluster store.
pub const PLUGIN_KV_PREFIX: &str = "plugins/handlers/";

/// Maximum number of loaded WASM plugins per node.
pub const MAX_PLUGINS: u32 = 64;

/// Default fuel budget for a single plugin invocation.
pub const PLUGIN_DEFAULT_FUEL: u64 = 500_000_000;

/// Default memory limit for a single plugin instance (128 MB).
pub const PLUGIN_DEFAULT_MEMORY: u64 = 128 * 1024 * 1024;
