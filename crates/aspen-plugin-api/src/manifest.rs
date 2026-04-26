//! Plugin manifest and info types for WASM handler plugins.

use serde::Deserialize;
use serde::Serialize;

fn default_optional_string() -> Option<String> {
    None
}

fn default_optional_u64() -> Option<u64> {
    None
}

fn default_string_list() -> Vec<String> {
    Vec::new()
}

fn default_permissions() -> PluginPermissions {
    PluginPermissions::default()
}

fn default_signature_info() -> Option<PluginSignatureInfo> {
    None
}

fn default_dependencies() -> Vec<PluginDependency> {
    Vec::new()
}

fn default_protocols() -> Vec<PluginProtocol> {
    Vec::new()
}

/// Protocol session advertised by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginProtocol {
    /// Stable transport identifier advertised to clients.
    pub identifier: String,
    /// Transport protocol version.
    pub version: u16,
    /// Maximum concurrent sessions this plugin accepts.
    pub max_concurrent_sessions: u32,
    /// Maximum chunk size accepted by one protocol session.
    pub max_chunk_size_bytes: u64,
    /// Maximum in-flight bytes across sessions.
    pub max_in_flight_bytes: u64,
    /// Session timeout in milliseconds.
    pub session_timeout_ms: u64,
}

/// Protocol identifier collision detected across manifests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginProtocolCollision {
    /// Colliding transport identifier.
    pub identifier: String,
    /// Plugin names that declared the same identifier.
    pub plugins: Vec<String>,
}

/// A dependency on another plugin.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginDependency {
    /// Name of the required plugin.
    pub name: String,
    /// Minimum version (semver). If None, any version satisfies.
    #[serde(default = "default_optional_string")]
    pub min_version: Option<String>,
    /// If true, missing dependency produces a warning, not an error.
    #[serde(default)]
    pub optional: bool,
}

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
    /// Protocol sessions this plugin can serve.
    #[serde(default = "default_protocols")]
    pub protocols: Vec<PluginProtocol>,
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
    #[serde(default = "default_optional_string")]
    pub app_id: Option<String>,
    /// Wall-clock execution timeout in seconds for a single guest call.
    ///
    /// Uses `DEFAULT_WASM_EXECUTION_TIMEOUT_SECS` (30s) if `None`.
    /// Clamped to `MAX_WASM_EXECUTION_TIMEOUT_SECS` (300s).
    ///
    /// hyperlight-wasm 0.12 does not support fuel metering, so this
    /// wall-clock timeout is the only execution-time safeguard.
    #[serde(default = "default_optional_u64")]
    pub execution_timeout_secs: Option<u64>,
    /// KV key prefixes this plugin is allowed to access.
    ///
    /// Every KV operation (get, put, delete, scan, cas) is validated to ensure
    /// the key starts with one of these prefixes. When empty, a default prefix
    /// of `__plugin:{name}:` is enforced, automatically isolating the plugin's
    /// keyspace.
    ///
    /// Tiger Style: Explicit bounds prevent cross-plugin data access.
    #[serde(default = "default_string_list")]
    pub kv_prefixes: Vec<String>,
    /// Capability permissions controlling which host APIs the plugin may use.
    ///
    /// Default: all denied. The manifest must explicitly grant each capability.
    /// Checked at runtime before each host function call.
    #[serde(default = "default_permissions")]
    pub permissions: PluginPermissions,
    /// Optional Ed25519 signature of the plugin WASM binary.
    ///
    /// Populated by the signing tool (`cargo aspen-plugin sign`) and verified
    /// on install. When present, the CLI checks the signature against trusted
    /// author keys before loading the plugin.
    #[serde(default = "default_signature_info")]
    pub signature: Option<PluginSignatureInfo>,
    /// Human-readable description.
    #[serde(default = "default_optional_string")]
    pub description: Option<String>,
    /// Author or organization.
    #[serde(default = "default_optional_string")]
    pub author: Option<String>,
    /// Searchable tags (e.g., ["storage", "kv", "core"]).
    #[serde(default = "default_string_list")]
    pub tags: Vec<String>,
    /// Minimum plugin API version required.
    #[serde(default = "default_optional_string")]
    pub min_api_version: Option<String>,
    /// Dependencies on other plugins.
    #[serde(default = "default_dependencies")]
    pub dependencies: Vec<PluginDependency>,
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
    /// Allow publishing events to the Nostr relay via nostr_publish_event.
    #[serde(default)]
    pub nostr_publish: bool,
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
            nostr_publish: false,
        }
    }
}

/// Return protocol identifier collisions across enabled plugin manifests.
#[must_use]
pub fn protocol_identifier_collisions(manifests: &[PluginManifest]) -> Vec<PluginProtocolCollision> {
    use std::collections::BTreeMap;

    let mut owners: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for manifest in manifests.iter().filter(|manifest| manifest.enabled) {
        debug_assert!(!manifest.name.is_empty(), "enabled manifest must have a name");
        for protocol in &manifest.protocols {
            debug_assert!(!protocol.identifier.is_empty(), "protocol must have an identifier");
            owners.entry(protocol.identifier.as_str()).or_default().push(manifest.name.clone());
        }
    }

    owners
        .into_iter()
        .filter_map(|(identifier, plugins)| {
            if plugins.len() > 1 {
                Some(PluginProtocolCollision {
                    identifier: identifier.to_string(),
                    plugins,
                })
            } else {
                None
            }
        })
        .collect()
}

/// Host operation attempted by a plugin protocol session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginHostAccess<'a> {
    /// KV read under a concrete key.
    KvRead { key: &'a str },
    /// KV write under a concrete key.
    KvWrite { key: &'a str },
    /// Blob read capability.
    BlobRead,
    /// Blob write capability.
    BlobWrite,
    /// Protocol-session capability.
    Protocol { identifier: &'a str },
}

/// Reason a host access request was denied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginHostAccessDenyReason {
    /// Required capability bit was not declared.
    PermissionMissing,
    /// KV key was outside declared prefixes.
    KvPrefixDenied,
    /// Protocol identifier was not declared.
    ProtocolNotDeclared,
}

/// Host access denial.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginHostAccessDenied {
    /// Denial reason.
    pub reason: PluginHostAccessDenyReason,
}

#[must_use]
pub fn plugin_kv_key_allowed(manifest: &PluginManifest, key: &str) -> bool {
    let default_prefix = format!("__plugin:{}:", manifest.name);
    let prefixes = if manifest.kv_prefixes.is_empty() {
        core::slice::from_ref(&default_prefix)
    } else {
        manifest.kv_prefixes.as_slice()
    };
    prefixes.iter().any(|prefix| key.starts_with(prefix))
}

pub fn validate_plugin_host_access(
    manifest: &PluginManifest,
    access: PluginHostAccess<'_>,
) -> Result<(), PluginHostAccessDenied> {
    match access {
        PluginHostAccess::KvRead { key } => {
            if !manifest.permissions.kv_read {
                return Err(PluginHostAccessDenied {
                    reason: PluginHostAccessDenyReason::PermissionMissing,
                });
            }
            if !plugin_kv_key_allowed(manifest, key) {
                return Err(PluginHostAccessDenied {
                    reason: PluginHostAccessDenyReason::KvPrefixDenied,
                });
            }
            Ok(())
        }
        PluginHostAccess::KvWrite { key } => {
            if !manifest.permissions.kv_write {
                return Err(PluginHostAccessDenied {
                    reason: PluginHostAccessDenyReason::PermissionMissing,
                });
            }
            if !plugin_kv_key_allowed(manifest, key) {
                return Err(PluginHostAccessDenied {
                    reason: PluginHostAccessDenyReason::KvPrefixDenied,
                });
            }
            Ok(())
        }
        PluginHostAccess::BlobRead => {
            if manifest.permissions.blob_read {
                Ok(())
            } else {
                Err(PluginHostAccessDenied {
                    reason: PluginHostAccessDenyReason::PermissionMissing,
                })
            }
        }
        PluginHostAccess::BlobWrite => {
            if manifest.permissions.blob_write {
                Ok(())
            } else {
                Err(PluginHostAccessDenied {
                    reason: PluginHostAccessDenyReason::PermissionMissing,
                })
            }
        }
        PluginHostAccess::Protocol { identifier } => {
            if manifest.protocols.iter().any(|protocol| protocol.identifier == identifier) {
                Ok(())
            } else {
                Err(PluginHostAccessDenied {
                    reason: PluginHostAccessDenyReason::ProtocolNotDeclared,
                })
            }
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
            nostr_publish: true,
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
    /// Protocol sessions this plugin can serve.
    #[serde(default = "default_protocols")]
    pub protocols: Vec<PluginProtocol>,
    /// Dispatch priority.
    pub priority: u32,
    /// Optional application ID for federation discovery.
    #[serde(default = "default_optional_string")]
    pub app_id: Option<String>,
    /// KV key prefixes this plugin is allowed to access.
    ///
    /// See [`PluginManifest::kv_prefixes`] for details.
    #[serde(default = "default_string_list")]
    pub kv_prefixes: Vec<String>,
    /// Capability permissions the plugin requires.
    ///
    /// Declared in `plugin.json` and carried through to [`PluginManifest`]
    /// at install time. Default: all denied.
    #[serde(default = "default_permissions")]
    pub permissions: PluginPermissions,
    /// Human-readable description.
    #[serde(default = "default_optional_string")]
    pub description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PROTOCOL_ID: &str = "/aspen/test/1";
    const TEST_VERSION: u16 = 1;
    const TEST_MAX_SESSIONS: u32 = 2;
    const TEST_MAX_CHUNK_BYTES: u64 = 64 * 1024;
    const TEST_MAX_IN_FLIGHT_BYTES: u64 = 256 * 1024;
    const TEST_TIMEOUT_MS: u64 = 30_000;

    fn protocol(identifier: &str) -> PluginProtocol {
        PluginProtocol {
            identifier: identifier.to_string(),
            version: TEST_VERSION,
            max_concurrent_sessions: TEST_MAX_SESSIONS,
            max_chunk_size_bytes: TEST_MAX_CHUNK_BYTES,
            max_in_flight_bytes: TEST_MAX_IN_FLIGHT_BYTES,
            session_timeout_ms: TEST_TIMEOUT_MS,
        }
    }

    fn manifest(name: &str, enabled: bool, identifier: &str) -> PluginManifest {
        PluginManifest {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            wasm_hash: "hash".to_string(),
            handles: vec![],
            protocols: vec![protocol(identifier)],
            priority: 900,
            fuel_limit: None,
            memory_limit: None,
            enabled,
            app_id: None,
            execution_timeout_secs: None,
            kv_prefixes: vec![],
            permissions: PluginPermissions::default(),
            signature: None,
            description: None,
            author: None,
            tags: vec![],
            min_api_version: None,
            dependencies: vec![],
        }
    }

    #[test]
    fn protocol_manifest_roundtrips() {
        let protocol = protocol(TEST_PROTOCOL_ID);
        let json = serde_json::to_string(&protocol).expect("serialize protocol");
        let decoded: PluginProtocol = serde_json::from_str(&json).expect("deserialize protocol");

        assert_eq!(decoded, protocol);
    }

    #[test]
    fn detects_enabled_protocol_collision() {
        let manifests = vec![
            manifest("left", true, TEST_PROTOCOL_ID),
            manifest("right", true, TEST_PROTOCOL_ID),
        ];

        let collisions = protocol_identifier_collisions(&manifests);

        assert_eq!(collisions.len(), 1);
        assert_eq!(collisions[0].identifier, TEST_PROTOCOL_ID);
        assert_eq!(collisions[0].plugins.len(), 2);
    }

    #[test]
    fn ignores_disabled_protocol_collision() {
        let manifests = vec![
            manifest("left", true, TEST_PROTOCOL_ID),
            manifest("right", false, TEST_PROTOCOL_ID),
        ];

        let collisions = protocol_identifier_collisions(&manifests);

        assert!(collisions.is_empty());
    }

    #[test]
    fn validates_declared_protocol_and_permissions() {
        let mut manifest = manifest("jj-native-forge", true, TEST_PROTOCOL_ID);
        manifest.kv_prefixes = vec!["forge:jj:".to_string()];
        manifest.permissions.kv_read = true;
        manifest.permissions.kv_write = true;
        manifest.permissions.blob_read = true;
        manifest.permissions.blob_write = true;

        assert!(
            validate_plugin_host_access(&manifest, PluginHostAccess::Protocol {
                identifier: TEST_PROTOCOL_ID
            })
            .is_ok()
        );
        assert!(validate_plugin_host_access(&manifest, PluginHostAccess::KvRead { key: "forge:jj:repo" }).is_ok());
        assert!(validate_plugin_host_access(&manifest, PluginHostAccess::BlobRead).is_ok());
        assert!(validate_plugin_host_access(&manifest, PluginHostAccess::BlobWrite).is_ok());
    }

    #[test]
    fn rejects_undeclared_host_access() {
        let manifest = manifest("jj-native-forge", true, TEST_PROTOCOL_ID);

        let protocol_err = validate_plugin_host_access(&manifest, PluginHostAccess::Protocol {
            identifier: "/aspen/other/1",
        })
        .expect_err("undeclared protocol rejected");
        let kv_err = validate_plugin_host_access(&manifest, PluginHostAccess::KvWrite { key: "forge:jj:repo" })
            .expect_err("missing kv permission rejected");
        let blob_err = validate_plugin_host_access(&manifest, PluginHostAccess::BlobRead)
            .expect_err("missing blob permission rejected");

        assert_eq!(protocol_err.reason, PluginHostAccessDenyReason::ProtocolNotDeclared);
        assert_eq!(kv_err.reason, PluginHostAccessDenyReason::PermissionMissing);
        assert_eq!(blob_err.reason, PluginHostAccessDenyReason::PermissionMissing);
    }

    #[test]
    fn rejects_kv_key_outside_declared_prefix() {
        let mut manifest = manifest("jj-native-forge", true, TEST_PROTOCOL_ID);
        manifest.kv_prefixes = vec!["forge:jj:".to_string()];
        manifest.permissions.kv_write = true;

        let err = validate_plugin_host_access(&manifest, PluginHostAccess::KvWrite { key: "forge:git:repo" })
            .expect_err("wrong kv prefix rejected");

        assert_eq!(err.reason, PluginHostAccessDenyReason::KvPrefixDenied);
    }
}
