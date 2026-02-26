//! WASM handler plugin resource bounds.
//!
//! Tiger Style: Fixed limits for WASM handler plugins to prevent
//! resource exhaustion and ensure deterministic plugin behavior.

/// Maximum number of loaded WASM plugins per node.
pub const MAX_PLUGINS: u32 = 64;

/// KV key prefix for plugin manifests.
pub const PLUGIN_KV_PREFIX: &str = "plugins/handlers/";

/// Maximum plugin name length in bytes.
pub const MAX_PLUGIN_NAME_SIZE: u32 = 128;

/// Maximum number of request types a single plugin can handle.
pub const MAX_PLUGIN_HANDLES: u32 = 64;

/// Timeout for loading a single WASM plugin in milliseconds.
pub const PLUGIN_LOAD_TIMEOUT_MS: u64 = 30_000;

/// Minimum priority for WASM plugins (ensures they run after native handlers).
pub const MIN_PLUGIN_PRIORITY: u32 = 900;

/// Maximum priority for WASM plugins.
pub const MAX_PLUGIN_PRIORITY: u32 = 999;

/// Maximum number of KV prefixes a plugin can declare.
pub const MAX_PLUGIN_KV_PREFIXES: u32 = 16;

/// Default KV prefix template for plugins that don't declare explicit prefixes.
///
/// Plugins without `kv_prefixes` get auto-scoped to `__plugin:{name}:` to prevent
/// unrestricted KV access. Format: this template + plugin name + `:`.
pub const DEFAULT_PLUGIN_KV_PREFIX_TEMPLATE: &str = "__plugin:";
