//! Plugin manifest and info types for WASM handler plugins.

use serde::Deserialize;
use serde::Serialize;

/// Manifest describing a WASM handler plugin and its resource requirements.
///
/// Stored in the cluster KV store under `PLUGIN_KV_PREFIX/{name}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Unique plugin name.
    pub name: String,
    /// Semantic version string.
    pub version: String,
    /// BLAKE3 hash of the WASM binary stored in blob storage.
    pub wasm_hash: String,
    /// Request type names this plugin handles.
    pub handles: Vec<String>,
    /// Dispatch priority (higher = later). Must be in `MIN_PLUGIN_PRIORITY..=MAX_PLUGIN_PRIORITY`.
    pub priority: u32,
    /// Fuel limit override. Uses `PLUGIN_DEFAULT_FUEL` if `None`.
    pub fuel_limit: Option<u64>,
    /// Memory limit override in bytes. Uses `PLUGIN_DEFAULT_MEMORY` if `None`.
    pub memory_limit: Option<u64>,
    /// Whether the plugin is active.
    pub enabled: bool,
    /// Optional application ID for federation discovery.
    ///
    /// When set, the plugin's app capability is registered with [`AppRegistry`]
    /// during loading so that federation dispatch can route requests to clusters
    /// running this plugin.
    #[serde(default)]
    pub app_id: Option<String>,
    /// Wall-clock execution timeout in seconds for a single guest call.
    ///
    /// Uses `DEFAULT_WASM_EXECUTION_TIMEOUT_SECS` (30s) if `None`.
    /// Clamped to `MAX_WASM_EXECUTION_TIMEOUT_SECS` (300s).
    ///
    /// hyperlight-wasm 0.12 does not support fuel metering, so this
    /// wall-clock timeout is the only execution-time safeguard.
    #[serde(default)]
    pub execution_timeout_secs: Option<u64>,
    /// KV key prefixes this plugin is allowed to access.
    ///
    /// Every KV operation (get, put, delete, scan, cas) is validated to ensure
    /// the key starts with one of these prefixes. When empty, a default prefix
    /// of `__plugin:{name}:` is enforced, automatically isolating the plugin's
    /// keyspace.
    ///
    /// Tiger Style: Explicit bounds prevent cross-plugin data access.
    #[serde(default)]
    pub kv_prefixes: Vec<String>,
}

/// Lightweight plugin info returned by `plugin-info` guest export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    /// Plugin name.
    pub name: String,
    /// Semantic version string.
    pub version: String,
    /// Request type names this plugin handles.
    pub handles: Vec<String>,
    /// Dispatch priority.
    pub priority: u32,
    /// Optional application ID for federation discovery.
    #[serde(default)]
    pub app_id: Option<String>,
    /// KV key prefixes this plugin is allowed to access.
    ///
    /// See [`PluginManifest::kv_prefixes`] for details.
    #[serde(default)]
    pub kv_prefixes: Vec<String>,
}
