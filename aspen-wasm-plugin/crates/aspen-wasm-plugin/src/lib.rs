//! WASM plugin helpers shipped with Aspen.
//!
//! The crate provides small, deterministic plugin-side cores that can be
//! compiled into WASM guests or exercised directly by host/runtime tests.

use aspen_forge_protocol::JJ_NATIVE_FORGE_ALPN_STR;
use aspen_forge_protocol::JJ_TRANSPORT_VERSION_CURRENT;
use aspen_forge_protocol::JjNativeOperation;
use aspen_forge_protocol::JjNativeRequest;
use aspen_forge_protocol::JjNativeResponse;
use aspen_forge_protocol::JjNativeStatus;
use aspen_forge_protocol::jj_native_response;
use aspen_plugin_api::PluginManifest;
use aspen_plugin_api::PluginPermissions;
use aspen_plugin_api::PluginProtocol;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

pub const JJ_PLUGIN_NAME: &str = "forge-jj-native";
pub const JJ_PLUGIN_VERSION: &str = "0.1.0";
pub const JJ_PLUGIN_PRIORITY: u32 = 550;
pub const JJ_PLUGIN_DEFAULT_WASM_HASH: &str = "builtin-jj-native-forge-plugin";
pub const JJ_PLUGIN_MAX_CONCURRENT_SESSIONS: u32 = 16;
pub const JJ_PLUGIN_MAX_CHUNK_SIZE_BYTES: u64 = 4 * 1024 * 1024;
pub const JJ_PLUGIN_MAX_IN_FLIGHT_BYTES: u64 = 64 * 1024 * 1024;
pub const JJ_PLUGIN_SESSION_TIMEOUT_MS: u64 = 30_000;
pub const JJ_BOOKMARK_PREFIX: &str = "forge:jj:bookmark:";
pub const JJ_CHANGE_PREFIX: &str = "forge:jj:change:";
pub const JJ_REACH_PREFIX: &str = "forge:jj:reach:";
pub const JJ_STAGE_PREFIX: &str = "forge:jj:stage:";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JjPluginSessionLimits {
    pub active_sessions: u32,
    pub chunk_size_bytes: u64,
    pub in_flight_bytes: u64,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum JjPluginSessionError {
    #[error("protocol identifier is not declared by the JJ plugin")]
    UndeclaredProtocol,
    #[error("transport version is incompatible with the JJ plugin")]
    IncompatibleTransportVersion,
    #[error("too many concurrent JJ sessions")]
    TooManySessions,
    #[error("JJ session chunk exceeds configured limit")]
    ChunkTooLarge,
    #[error("JJ session in-flight bytes exceed configured limit")]
    TooMuchInFlight,
    #[error("JJ session timed out")]
    SessionTimedOut,
}

#[must_use]
pub fn jj_plugin_permissions() -> PluginPermissions {
    PluginPermissions {
        kv_read: true,
        kv_write: true,
        blob_read: true,
        blob_write: true,
        cluster_info: false,
        randomness: false,
        signing: false,
        timers: true,
        hooks: false,
        sql_query: false,
        nostr_publish: false,
    }
}

#[must_use]
pub fn jj_plugin_kv_prefixes() -> Vec<String> {
    vec![
        JJ_BOOKMARK_PREFIX.to_string(),
        JJ_CHANGE_PREFIX.to_string(),
        JJ_REACH_PREFIX.to_string(),
        JJ_STAGE_PREFIX.to_string(),
    ]
}

#[must_use]
pub fn jj_plugin_protocol() -> PluginProtocol {
    PluginProtocol {
        identifier: JJ_NATIVE_FORGE_ALPN_STR.to_string(),
        version: JJ_TRANSPORT_VERSION_CURRENT,
        max_concurrent_sessions: JJ_PLUGIN_MAX_CONCURRENT_SESSIONS,
        max_chunk_size_bytes: JJ_PLUGIN_MAX_CHUNK_SIZE_BYTES,
        max_in_flight_bytes: JJ_PLUGIN_MAX_IN_FLIGHT_BYTES,
        session_timeout_ms: JJ_PLUGIN_SESSION_TIMEOUT_MS,
    }
}

#[must_use]
pub fn jj_plugin_manifest(wasm_hash: impl Into<String>) -> PluginManifest {
    PluginManifest {
        name: JJ_PLUGIN_NAME.to_string(),
        version: JJ_PLUGIN_VERSION.to_string(),
        wasm_hash: wasm_hash.into(),
        handles: vec!["ForgeJjNative".to_string()],
        protocols: vec![jj_plugin_protocol()],
        priority: JJ_PLUGIN_PRIORITY,
        fuel_limit: None,
        memory_limit: None,
        enabled: true,
        app_id: Some("forge".to_string()),
        execution_timeout_secs: None,
        kv_prefixes: jj_plugin_kv_prefixes(),
        permissions: jj_plugin_permissions(),
        signature: None,
        description: Some("JJ-native Forge protocol plugin".to_string()),
        author: Some("Aspen".to_string()),
        tags: vec!["forge".to_string(), "jj".to_string()],
        min_api_version: None,
        dependencies: Vec::new(),
    }
}

#[must_use]
pub fn builtin_jj_plugin_manifest() -> PluginManifest {
    jj_plugin_manifest(JJ_PLUGIN_DEFAULT_WASM_HASH)
}

pub fn admit_jj_plugin_session(
    protocol_identifier: &str,
    transport_version: u16,
    limits: JjPluginSessionLimits,
) -> Result<(), JjPluginSessionError> {
    if protocol_identifier != JJ_NATIVE_FORGE_ALPN_STR {
        return Err(JjPluginSessionError::UndeclaredProtocol);
    }
    if transport_version != JJ_TRANSPORT_VERSION_CURRENT {
        return Err(JjPluginSessionError::IncompatibleTransportVersion);
    }
    if limits.active_sessions >= JJ_PLUGIN_MAX_CONCURRENT_SESSIONS {
        return Err(JjPluginSessionError::TooManySessions);
    }
    if limits.chunk_size_bytes > JJ_PLUGIN_MAX_CHUNK_SIZE_BYTES {
        return Err(JjPluginSessionError::ChunkTooLarge);
    }
    if limits.in_flight_bytes > JJ_PLUGIN_MAX_IN_FLIGHT_BYTES {
        return Err(JjPluginSessionError::TooMuchInFlight);
    }
    if limits.elapsed_ms > JJ_PLUGIN_SESSION_TIMEOUT_MS {
        return Err(JjPluginSessionError::SessionTimedOut);
    }
    Ok(())
}

#[must_use]
pub fn handle_jj_native_request(request: &JjNativeRequest) -> JjNativeResponse {
    if request.transport_version != JJ_TRANSPORT_VERSION_CURRENT {
        return jj_native_response(
            JjNativeStatus::IncompatibleTransportVersion,
            Some("JJ plugin transport version is incompatible".to_string()),
        );
    }

    match request.operation {
        JjNativeOperation::Clone
        | JjNativeOperation::Fetch
        | JjNativeOperation::Push
        | JjNativeOperation::BookmarkSync
        | JjNativeOperation::ResolveChangeId => jj_native_response(
            JjNativeStatus::CapabilityUnavailable,
            Some("JJ-native plugin request handlers require host storage session wiring".to_string()),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> JjPluginSessionLimits {
        JjPluginSessionLimits {
            active_sessions: 0,
            chunk_size_bytes: JJ_PLUGIN_MAX_CHUNK_SIZE_BYTES,
            in_flight_bytes: JJ_PLUGIN_MAX_IN_FLIGHT_BYTES,
            elapsed_ms: JJ_PLUGIN_SESSION_TIMEOUT_MS,
        }
    }

    fn request(operation: JjNativeOperation, transport_version: u16) -> JjNativeRequest {
        JjNativeRequest {
            repo_id: "repo".to_string(),
            operation,
            transport_version,
            want_objects: Vec::new(),
            have_objects: Vec::new(),
            change_ids: Vec::new(),
            bookmark_mutations: Vec::new(),
        }
    }

    #[test]
    fn manifest_declares_jj_permissions_protocol_and_prefixes() {
        let manifest = builtin_jj_plugin_manifest();
        assert_eq!(manifest.name, JJ_PLUGIN_NAME);
        assert_eq!(manifest.handles, vec!["ForgeJjNative".to_string()]);
        assert_eq!(manifest.protocols, vec![jj_plugin_protocol()]);
        assert_eq!(manifest.kv_prefixes, jj_plugin_kv_prefixes());
        assert!(manifest.permissions.kv_read);
        assert!(manifest.permissions.kv_write);
        assert!(manifest.permissions.blob_read);
        assert!(manifest.permissions.blob_write);
        assert!(manifest.permissions.timers);
        assert!(!manifest.permissions.sql_query);
    }

    #[test]
    fn session_admission_accepts_declared_protocol_within_limits() {
        let result = admit_jj_plugin_session(JJ_NATIVE_FORGE_ALPN_STR, JJ_TRANSPORT_VERSION_CURRENT, limits());
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn session_admission_rejects_undeclared_protocol() {
        let result = admit_jj_plugin_session("/aspen/forge/other/1", JJ_TRANSPORT_VERSION_CURRENT, limits());
        assert_eq!(result, Err(JjPluginSessionError::UndeclaredProtocol));
    }

    #[test]
    fn session_admission_rejects_resource_limit_violations() {
        let mut too_many = limits();
        too_many.active_sessions = JJ_PLUGIN_MAX_CONCURRENT_SESSIONS;
        assert_eq!(
            admit_jj_plugin_session(JJ_NATIVE_FORGE_ALPN_STR, JJ_TRANSPORT_VERSION_CURRENT, too_many),
            Err(JjPluginSessionError::TooManySessions)
        );

        let mut too_large = limits();
        too_large.chunk_size_bytes = JJ_PLUGIN_MAX_CHUNK_SIZE_BYTES.saturating_add(1);
        assert_eq!(
            admit_jj_plugin_session(JJ_NATIVE_FORGE_ALPN_STR, JJ_TRANSPORT_VERSION_CURRENT, too_large),
            Err(JjPluginSessionError::ChunkTooLarge)
        );

        let mut too_slow = limits();
        too_slow.elapsed_ms = JJ_PLUGIN_SESSION_TIMEOUT_MS.saturating_add(1);
        assert_eq!(
            admit_jj_plugin_session(JJ_NATIVE_FORGE_ALPN_STR, JJ_TRANSPORT_VERSION_CURRENT, too_slow),
            Err(JjPluginSessionError::SessionTimedOut)
        );
    }

    #[test]
    fn request_handler_rejects_incompatible_transport_version() {
        let response = handle_jj_native_request(&request(
            JjNativeOperation::Fetch,
            JJ_TRANSPORT_VERSION_CURRENT.saturating_add(1),
        ));
        assert_eq!(response.status, JjNativeStatus::IncompatibleTransportVersion);
    }
}
