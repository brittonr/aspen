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
}
