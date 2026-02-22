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
    /// Capability permissions controlling which host APIs the plugin may use.
    ///
    /// Default: all denied. The manifest must explicitly grant each capability.
    /// Checked at runtime before each host function call.
    #[serde(default)]
    pub permissions: PluginPermissions,
    /// Optional Ed25519 signature of the plugin WASM binary.
    ///
    /// Populated by the signing tool (`cargo aspen-plugin sign`) and verified
    /// on install. When present, the CLI checks the signature against trusted
    /// author keys before loading the plugin.
    #[serde(default)]
    pub signature: Option<PluginSignatureInfo>,
}

/// Lightweight signature metadata embedded in plugin manifests.
///
/// This mirrors [`aspen_plugin_signing::PluginSignature`] but lives in
/// `aspen-plugin-api` to avoid pulling in crypto dependencies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSignatureInfo {
    /// Ed25519 public key of the signer (64 hex chars).
    pub author_pubkey: String,
    /// Ed25519 signature over BLAKE3(wasm_binary) (128 hex chars).
    pub signature: String,
    /// BLAKE3 hash of the signed WASM binary (64 hex chars).
    pub wasm_hash: String,
    /// Timestamp of signing (Unix milliseconds).
    pub signed_at_ms: u64,
}

/// Capability permissions for a WASM plugin.
///
/// Controls which host functions the plugin is allowed to call.
/// Checked at runtime before each host function invocation.
///
/// Default: all permissions denied (least privilege).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginPermissions {
    /// Allow reading from the KV store (kv_get, kv_scan).
    #[serde(default)]
    pub kv_read: bool,
    /// Allow writing to the KV store (kv_put, kv_delete, kv_cas, kv_batch).
    #[serde(default)]
    pub kv_write: bool,
    /// Allow reading from the blob store (blob_has, blob_get).
    #[serde(default)]
    pub blob_read: bool,
    /// Allow writing to the blob store (blob_put).
    #[serde(default)]
    pub blob_write: bool,
    /// Allow querying cluster state (is_leader, leader_id).
    #[serde(default)]
    pub cluster_info: bool,
    /// Allow generating random bytes.
    #[serde(default)]
    pub randomness: bool,
    /// Allow cryptographic signing with the node's key.
    #[serde(default)]
    pub signing: bool,
    /// Allow scheduling timers.
    #[serde(default)]
    pub timers: bool,
    /// Allow subscribing to hook events.
    #[serde(default)]
    pub hooks: bool,
    /// Allow executing SQL queries via the sql_query host function.
    #[serde(default)]
    pub sql_query: bool,
}

impl Default for PluginPermissions {
    /// Default: all permissions denied (explicit opt-in required).
    fn default() -> Self {
        Self {
            kv_read: false,
            kv_write: false,
            blob_read: false,
            blob_write: false,
            cluster_info: false,
            randomness: false,
            signing: false,
            timers: false,
            hooks: false,
            sql_query: false,
        }
    }
}

impl PluginPermissions {
    /// Create permissions with all capabilities granted.
    ///
    /// Useful for trusted system plugins.
    pub fn all() -> Self {
        Self {
            kv_read: true,
            kv_write: true,
            blob_read: true,
            blob_write: true,
            cluster_info: true,
            randomness: true,
            signing: true,
            timers: true,
            hooks: true,
            sql_query: true,
        }
    }
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
    /// Capability permissions the plugin requires.
    ///
    /// Declared in `plugin.json` and carried through to [`PluginManifest`]
    /// at install time. Default: all denied.
    #[serde(default)]
    pub permissions: PluginPermissions,
}
