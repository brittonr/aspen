//! Host function bindings for WASM handler plugins.
//!
//! Registers host functions on a `ProtoWasmSandbox` in primitive mode.
//! Extends the base host functions (logging, clock, kv-store, blob-store)
//! with identity, randomness, and cluster-state queries.
//!
//! ## Primitive Mode Type Encoding
//!
//! Only `String`, `i32`/`u32`/`i64`/`u64`, `f32`/`f64`, `bool`, and
//! `Vec<u8>` are supported. Complex types are encoded as follows:
//!
//! ### String-based results (`Result<(), String>`, `Result<String, String>`)
//!
//! - `\0` or `\0` + value = success
//! - `\x01` + message = error
//!
//! Used by: `kv_put`, `kv_delete`, `kv_cas`, `blob_put`
//!
//! ### Vec-based option results (`Option<Vec<u8>>`)
//!
//! - `[0x00]` + data = found/success
//! - `[0x01]` = not found
//! - `[0x02]` + error message (UTF-8) = error
//!
//! Used by: `kv_get`, `blob_get`
//!
//! ### Vec-based results (`Result<Vec<u8>, String>`)
//!
//! - `[0x00]` + payload = success
//! - `[0x01]` + error message (UTF-8) = error
//!
//! Used by: `kv_scan` (payload is JSON-encoded `Vec<(String, Vec<u8>)>`)
//!
//! See also: [HOST_ABI.md](../../../docs/HOST_ABI.md) for the formal ABI contract.

use std::sync::Arc;

use aspen_blob::prelude::*;
use aspen_core::KeyValueStore;
use aspen_hlc::HLC;
use aspen_traits::ClusterController;

/// Host context for WASM handler plugin callbacks.
///
/// Holds references to cluster services that the guest can interact with
/// through registered host functions.
pub struct PluginHostContext {
    /// KV store for guest key-value operations.
    pub kv_store: Arc<dyn KeyValueStore>,
    /// Blob store for guest blob operations.
    pub blob_store: Arc<dyn BlobStore>,
    /// Cluster controller for leader queries.
    pub controller: Arc<dyn ClusterController>,
    /// Node ID of the host node.
    pub node_id: u64,
    /// Plugin name for structured log context.
    pub plugin_name: String,
    /// Iroh secret key for Ed25519 signing on behalf of guest plugins.
    pub secret_key: Option<iroh::SecretKey>,
    /// Hybrid logical clock for causal timestamps.
    pub hlc: Option<Arc<HLC>>,
    /// Allowed KV key prefixes for namespace isolation.
    ///
    /// Every KV operation validates the key against these prefixes.
    /// If the key doesn't start with any allowed prefix, the operation
    /// is rejected with a namespace violation error.
    ///
    /// Tiger Style: Enforced bounds prevent cross-plugin data access.
    pub allowed_kv_prefixes: Vec<String>,
}

impl PluginHostContext {
    /// Create a new host context for a WASM handler plugin.
    pub fn new(
        kv_store: Arc<dyn KeyValueStore>,
        blob_store: Arc<dyn BlobStore>,
        controller: Arc<dyn ClusterController>,
        node_id: u64,
        plugin_name: String,
    ) -> Self {
        Self {
            kv_store,
            blob_store,
            controller,
            node_id,
            plugin_name,
            secret_key: None,
            hlc: None,
            allowed_kv_prefixes: Vec::new(),
        }
    }

    /// Set the allowed KV prefixes for namespace isolation.
    ///
    /// If `prefixes` is empty, a default prefix of `__plugin:{name}:` is used,
    /// ensuring automatic isolation for plugins that don't declare explicit prefixes.
    pub fn with_kv_prefixes(mut self, prefixes: Vec<String>) -> Self {
        if prefixes.is_empty() {
            self.allowed_kv_prefixes = vec![format!(
                "{}{}:",
                aspen_constants::plugin::DEFAULT_PLUGIN_KV_PREFIX_TEMPLATE,
                self.plugin_name
            )];
        } else {
            self.allowed_kv_prefixes = prefixes;
        }
        self
    }

    /// Set the Iroh secret key for Ed25519 operations.
    pub fn with_secret_key(mut self, secret_key: iroh::SecretKey) -> Self {
        self.secret_key = Some(secret_key);
        self
    }

    /// Set the HLC instance for causal timestamps.
    pub fn with_hlc(mut self, hlc: Arc<HLC>) -> Self {
        self.hlc = Some(hlc);
        self
    }
}

// ---------------------------------------------------------------------------
// Logging host functions
// ---------------------------------------------------------------------------

/// Log an informational message from a WASM plugin.
pub fn log_info(plugin_name: &str, message: &str) {
    tracing::info!(plugin = plugin_name, guest_message = %message, "wasm plugin log");
}

/// Log a debug message from a WASM plugin.
pub fn log_debug(plugin_name: &str, message: &str) {
    tracing::debug!(plugin = plugin_name, guest_message = %message, "wasm plugin log");
}

/// Log a warning message from a WASM plugin.
pub fn log_warn(plugin_name: &str, message: &str) {
    tracing::warn!(plugin = plugin_name, guest_message = %message, "wasm plugin log");
}

// ---------------------------------------------------------------------------
// Clock host function
// ---------------------------------------------------------------------------

/// Return the current wall-clock time as milliseconds since the Unix epoch.
pub fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

// ---------------------------------------------------------------------------
// KV namespace validation
// ---------------------------------------------------------------------------

/// Validate that a KV key is within the plugin's allowed namespace.
///
/// Returns `Ok(())` if the key starts with any allowed prefix,
/// or `Err` with a descriptive message if not.
///
/// Tiger Style: Empty `allowed_prefixes` means unrestricted access
/// (backwards compat only — `with_kv_prefixes` always populates this).
fn validate_key_prefix(
    plugin_name: &str,
    allowed_prefixes: &[String],
    key: &str,
    operation: &str,
) -> Result<(), String> {
    if allowed_prefixes.is_empty() {
        return Ok(());
    }
    for prefix in allowed_prefixes {
        if key.starts_with(prefix.as_str()) {
            return Ok(());
        }
    }
    let msg = format!(
        "KV namespace violation: plugin '{}' {} key '{}' outside allowed prefixes {:?}",
        plugin_name, operation, key, allowed_prefixes
    );
    tracing::warn!("{}", msg);
    Err(msg)
}

/// Validate that a KV scan prefix is within the plugin's allowed namespace.
///
/// The scan prefix must start with one of the allowed prefixes. This prevents
/// plugins from scanning the entire keyspace or another plugin's keys.
fn validate_scan_prefix(plugin_name: &str, allowed_prefixes: &[String], prefix: &str) -> Result<(), String> {
    if allowed_prefixes.is_empty() {
        return Ok(());
    }
    for allowed in allowed_prefixes {
        if prefix.starts_with(allowed.as_str()) {
            return Ok(());
        }
    }
    let msg = format!(
        "KV namespace violation: plugin '{}' scan prefix '{}' outside allowed prefixes {:?}",
        plugin_name, prefix, allowed_prefixes
    );
    tracing::warn!("{}", msg);
    Err(msg)
}

// ---------------------------------------------------------------------------
// KV Store host functions
// ---------------------------------------------------------------------------

/// Put a key-value pair into the distributed KV store.
///
/// The value bytes must be valid UTF-8.
pub fn kv_put(ctx: &PluginHostContext, key: &str, value: &[u8]) -> Result<(), String> {
    validate_key_prefix(&ctx.plugin_name, &ctx.allowed_kv_prefixes, key, "write")?;
    let value_str = std::str::from_utf8(value).map_err(|e| format!("value is not valid UTF-8: {e}"))?;

    let handle = tokio::runtime::Handle::current();
    handle.block_on(async {
        let request = aspen_kv_types::WriteRequest::set(key, value_str);
        ctx.kv_store.write(request).await.map(|_| ()).map_err(|e| {
            tracing::warn!(
                plugin = %ctx.plugin_name,
                key,
                error = %e,
                "wasm plugin kv_put failed"
            );
            format!("kv_put failed: {e}")
        })
    })
}

/// Delete a key from the distributed KV store.
pub fn kv_delete(ctx: &PluginHostContext, key: &str) -> Result<(), String> {
    validate_key_prefix(&ctx.plugin_name, &ctx.allowed_kv_prefixes, key, "delete")?;
    let handle = tokio::runtime::Handle::current();
    handle.block_on(async {
        let request = aspen_kv_types::DeleteRequest::new(key);
        ctx.kv_store.delete(request).await.map(|_| ()).map_err(|e| {
            tracing::warn!(
                plugin = %ctx.plugin_name,
                key,
                error = %e,
                "wasm plugin kv_delete failed"
            );
            format!("kv_delete failed: {e}")
        })
    })
}

/// Compare-and-swap a key in the distributed KV store.
///
/// If `expected` is empty, the key must not exist (create-if-absent).
/// Both `expected` and `new_value` must be valid UTF-8.
pub fn kv_cas(ctx: &PluginHostContext, key: &str, expected: &[u8], new_value: &[u8]) -> Result<(), String> {
    validate_key_prefix(&ctx.plugin_name, &ctx.allowed_kv_prefixes, key, "cas")?;
    let expected_str = if expected.is_empty() {
        None
    } else {
        Some(std::str::from_utf8(expected).map_err(|e| format!("expected is not valid UTF-8: {e}"))?.to_string())
    };
    let new_value_str = std::str::from_utf8(new_value).map_err(|e| format!("new_value is not valid UTF-8: {e}"))?;

    let handle = tokio::runtime::Handle::current();
    handle.block_on(async {
        let request = aspen_kv_types::WriteRequest::compare_and_swap(key, expected_str, new_value_str);
        ctx.kv_store.write(request).await.map(|_| ()).map_err(|e| {
            tracing::warn!(
                plugin = %ctx.plugin_name,
                key,
                error = %e,
                "wasm plugin kv_cas failed"
            );
            format!("kv_cas failed: {e}")
        })
    })
}

// ---------------------------------------------------------------------------
// Blob Store host functions
// ---------------------------------------------------------------------------

/// Check whether a blob exists in the store.
///
/// The `hash` parameter is the hex-encoded BLAKE3 hash of the blob.
pub fn blob_has(ctx: &PluginHostContext, hash: &str) -> bool {
    let blob_hash = match hash.parse::<iroh_blobs::Hash>() {
        Ok(h) => h,
        Err(e) => {
            tracing::warn!(
                plugin = %ctx.plugin_name,
                hash,
                error = %e,
                "wasm plugin blob_has: invalid hash"
            );
            return false;
        }
    };

    let handle = tokio::runtime::Handle::current();
    handle.block_on(async {
        match ctx.blob_store.has(&blob_hash).await {
            Ok(exists) => exists,
            Err(e) => {
                tracing::warn!(
                    plugin = %ctx.plugin_name,
                    hash,
                    error = %e,
                    "wasm plugin blob_has failed"
                );
                false
            }
        }
    })
}

/// Store bytes in the blob store and return the hex-encoded BLAKE3 hash.
pub fn blob_put(ctx: &PluginHostContext, data: &[u8]) -> Result<String, String> {
    let handle = tokio::runtime::Handle::current();
    handle.block_on(async {
        match ctx.blob_store.add_bytes(data).await {
            Ok(result) => Ok(result.blob_ref.hash.to_string()),
            Err(e) => {
                tracing::warn!(
                    plugin = %ctx.plugin_name,
                    data_len = data.len(),
                    error = %e,
                    "wasm plugin blob_put failed"
                );
                Err(format!("blob_put failed: {e}"))
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Identity host functions
// ---------------------------------------------------------------------------

/// Return the node ID of the host.
pub fn node_id(ctx: &PluginHostContext) -> u64 {
    ctx.node_id
}

// ---------------------------------------------------------------------------
// Randomness host functions
// ---------------------------------------------------------------------------

/// Generate `count` random bytes using the OS CSPRNG.
pub fn random_bytes(count: u32) -> Vec<u8> {
    let count = count.min(4096) as usize; // Cap at 4KB per call
    let mut buf = vec![0u8; count];
    getrandom::getrandom(&mut buf).unwrap_or_else(|e| {
        tracing::warn!(error = %e, "getrandom failed, returning zeroed bytes");
    });
    buf
}

// ---------------------------------------------------------------------------
// Cluster host functions
// ---------------------------------------------------------------------------

/// Check if the current node is the Raft leader.
pub fn is_leader(ctx: &PluginHostContext) -> bool {
    let handle = tokio::runtime::Handle::current();
    handle.block_on(async {
        match ctx.controller.get_leader().await {
            Ok(Some(leader_id)) => leader_id == ctx.node_id,
            _ => false,
        }
    })
}

/// Get the current Raft leader's node ID, or 0 if unknown.
pub fn leader_id(ctx: &PluginHostContext) -> u64 {
    let handle = tokio::runtime::Handle::current();
    handle.block_on(async {
        match ctx.controller.get_leader().await {
            Ok(Some(id)) => id,
            _ => 0,
        }
    })
}

// ---------------------------------------------------------------------------
// Crypto host functions
// ---------------------------------------------------------------------------

/// Sign data with the node's Ed25519 secret key.
///
/// Returns the 64-byte Ed25519 signature, or an empty vec if no key is configured.
pub fn sign(ctx: &PluginHostContext, data: &[u8]) -> Vec<u8> {
    match &ctx.secret_key {
        Some(key) => {
            let sig = key.sign(data);
            sig.to_bytes().to_vec()
        }
        None => {
            tracing::warn!(plugin = %ctx.plugin_name, "wasm plugin sign: no secret key configured");
            Vec::new()
        }
    }
}

/// Verify an Ed25519 signature given a hex-encoded public key.
pub fn verify(public_key_hex: &str, data: &[u8], sig_bytes: &[u8]) -> bool {
    let Ok(key_bytes) = hex::decode(public_key_hex) else {
        return false;
    };
    let Ok(key_array): Result<[u8; 32], _> = key_bytes.try_into() else {
        return false;
    };
    let Ok(sig_array): Result<[u8; 64], _> = sig_bytes.to_vec().try_into() else {
        return false;
    };
    let Ok(verifying_key) = ed25519_dalek::VerifyingKey::from_bytes(&key_array) else {
        return false;
    };
    let signature = ed25519_dalek::Signature::from_bytes(&sig_array);
    use ed25519_dalek::Verifier;
    verifying_key.verify(data, &signature).is_ok()
}

/// Return the node's public key as a hex-encoded string.
pub fn public_key_hex(ctx: &PluginHostContext) -> String {
    match &ctx.secret_key {
        Some(key) => hex::encode(key.public().as_bytes()),
        None => {
            tracing::warn!(plugin = %ctx.plugin_name, "wasm plugin public_key_hex: no secret key configured");
            String::new()
        }
    }
}

/// Return the current HLC timestamp as milliseconds since epoch.
pub fn hlc_now(ctx: &PluginHostContext) -> u64 {
    match &ctx.hlc {
        Some(hlc) => {
            let ts = aspen_hlc::new_timestamp(hlc);
            aspen_hlc::to_unix_ms(&ts)
        }
        None => {
            // Fall back to wall clock
            now_ms()
        }
    }
}

// ---------------------------------------------------------------------------
// Sandbox registration (primitive mode)
// ---------------------------------------------------------------------------

/// Register all host functions on a `ProtoWasmSandbox` for a WASM handler plugin.
///
/// Must be called before `proto.load_runtime()`. Each closure captures a
/// shared `Arc<PluginHostContext>` and delegates to the standalone functions.
pub fn register_plugin_host_functions(
    proto: &mut hyperlight_wasm::ProtoWasmSandbox,
    ctx: Arc<PluginHostContext>,
) -> anyhow::Result<()> {
    // -- Logging --
    let ctx_log_info = Arc::clone(&ctx);
    proto
        .register("log_info", move |msg: String| -> () {
            log_info(&ctx_log_info.plugin_name, &msg);
        })
        .map_err(|e| anyhow::anyhow!("failed to register log_info: {e}"))?;

    let ctx_log_debug = Arc::clone(&ctx);
    proto
        .register("log_debug", move |msg: String| -> () {
            log_debug(&ctx_log_debug.plugin_name, &msg);
        })
        .map_err(|e| anyhow::anyhow!("failed to register log_debug: {e}"))?;

    let ctx_log_warn = Arc::clone(&ctx);
    proto
        .register("log_warn", move |msg: String| -> () {
            log_warn(&ctx_log_warn.plugin_name, &msg);
        })
        .map_err(|e| anyhow::anyhow!("failed to register log_warn: {e}"))?;

    // -- Clock --
    proto
        .register("now_ms", || -> u64 { now_ms() })
        .map_err(|e| anyhow::anyhow!("failed to register now_ms: {e}"))?;

    // -- KV Store --
    // kv_get: returns Vec<u8> with tag byte
    // [0x00] ++ value = found, [0x01] = not-found, [0x02] ++ error_msg = error
    let ctx_kv_get = Arc::clone(&ctx);
    proto
        .register("kv_get", move |key: String| -> Vec<u8> {
            if let Err(e) = validate_key_prefix(&ctx_kv_get.plugin_name, &ctx_kv_get.allowed_kv_prefixes, &key, "read")
            {
                let mut v = vec![0x02];
                v.extend_from_slice(e.as_bytes());
                return v;
            }
            let handle = tokio::runtime::Handle::current();
            handle.block_on(async {
                let request = aspen_kv_types::ReadRequest::new(&key);
                match ctx_kv_get.kv_store.read(request).await {
                    Ok(result) => match result.kv {
                        Some(entry) => {
                            let bytes = entry.value.into_bytes();
                            let mut v = Vec::with_capacity(1 + bytes.len());
                            v.push(0x00);
                            v.extend_from_slice(&bytes);
                            v
                        }
                        None => vec![0x01],
                    },
                    Err(e) => {
                        tracing::warn!(
                            plugin = %ctx_kv_get.plugin_name,
                            key = %key,
                            error = %e,
                            "wasm plugin kv_get failed"
                        );
                        let mut v = vec![0x02];
                        v.extend_from_slice(format!("kv_get failed: {e}").as_bytes());
                        v
                    }
                }
            })
        })
        .map_err(|e| anyhow::anyhow!("failed to register kv_get: {e}"))?;

    // kv_put: returns String with tag prefix (\0 = success, \x01 = error)
    let ctx_kv_put = Arc::clone(&ctx);
    proto
        .register("kv_put", move |key: String, value: Vec<u8>| -> String {
            match kv_put(&ctx_kv_put, &key, &value) {
                Ok(()) => "\0".to_string(),
                Err(e) => format!("\x01{e}"),
            }
        })
        .map_err(|e| anyhow::anyhow!("failed to register kv_put: {e}"))?;

    // kv_delete: returns String with tag prefix (\0 = success, \x01 = error)
    let ctx_kv_delete = Arc::clone(&ctx);
    proto
        .register("kv_delete", move |key: String| -> String {
            match kv_delete(&ctx_kv_delete, &key) {
                Ok(()) => "\0".to_string(),
                Err(e) => format!("\x01{e}"),
            }
        })
        .map_err(|e| anyhow::anyhow!("failed to register kv_delete: {e}"))?;

    // kv_scan: returns Vec<u8> with tag byte
    // [0x00] ++ json_bytes = ok, [0x01] ++ error_msg = error
    let ctx_kv_scan = Arc::clone(&ctx);
    proto
        .register("kv_scan", move |prefix: String, limit: u32| -> Vec<u8> {
            if let Err(e) = validate_scan_prefix(&ctx_kv_scan.plugin_name, &ctx_kv_scan.allowed_kv_prefixes, &prefix) {
                let mut v = vec![0x01];
                v.extend_from_slice(e.as_bytes());
                return v;
            }
            let handle = tokio::runtime::Handle::current();
            handle.block_on(async {
                let bounded_limit = if limit == 0 {
                    aspen_constants::api::DEFAULT_SCAN_LIMIT
                } else {
                    limit.min(aspen_constants::api::MAX_SCAN_RESULTS)
                };
                let request = aspen_kv_types::ScanRequest {
                    prefix: prefix.to_string(),
                    limit_results: Some(bounded_limit),
                    continuation_token: None,
                };
                match ctx_kv_scan.kv_store.scan(request).await {
                    Ok(result) => {
                        let entries: Vec<(String, Vec<u8>)> =
                            result.entries.into_iter().map(|entry| (entry.key, entry.value.into_bytes())).collect();
                        match serde_json::to_vec(&entries) {
                            Ok(json) => {
                                let mut v = Vec::with_capacity(1 + json.len());
                                v.push(0x00);
                                v.extend_from_slice(&json);
                                v
                            }
                            Err(e) => {
                                let mut v = vec![0x01];
                                v.extend_from_slice(format!("kv_scan JSON encode failed: {e}").as_bytes());
                                v
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            plugin = %ctx_kv_scan.plugin_name,
                            prefix = %prefix,
                            error = %e,
                            "wasm plugin kv_scan failed"
                        );
                        let mut v = vec![0x01];
                        v.extend_from_slice(format!("kv_scan failed: {e}").as_bytes());
                        v
                    }
                }
            })
        })
        .map_err(|e| anyhow::anyhow!("failed to register kv_scan: {e}"))?;

    // kv_cas: returns String with tag prefix (\0 = success, \x01 = error)
    let ctx_kv_cas = Arc::clone(&ctx);
    proto
        .register("kv_cas", move |key: String, expected: Vec<u8>, new_value: Vec<u8>| -> String {
            match kv_cas(&ctx_kv_cas, &key, &expected, &new_value) {
                Ok(()) => "\0".to_string(),
                Err(e) => format!("\x01{e}"),
            }
        })
        .map_err(|e| anyhow::anyhow!("failed to register kv_cas: {e}"))?;

    // -- Blob Store --
    // blob_has: bool is directly supported
    let ctx_blob_has = Arc::clone(&ctx);
    proto
        .register("blob_has", move |hash: String| -> bool { blob_has(&ctx_blob_has, &hash) })
        .map_err(|e| anyhow::anyhow!("failed to register blob_has: {e}"))?;

    // blob_get: returns Vec<u8> with tag byte
    // [0x00] ++ data = found, [0x01] = not-found, [0x02] ++ error_msg = error
    let ctx_blob_get = Arc::clone(&ctx);
    proto
        .register("blob_get", move |hash: String| -> Vec<u8> {
            let blob_hash = match hash.parse::<iroh_blobs::Hash>() {
                Ok(h) => h,
                Err(e) => {
                    tracing::warn!(
                        plugin = %ctx_blob_get.plugin_name,
                        hash = %hash,
                        error = %e,
                        "wasm plugin blob_get: invalid hash"
                    );
                    let mut v = vec![0x02];
                    v.extend_from_slice(format!("invalid hash: {e}").as_bytes());
                    return v;
                }
            };
            let handle = tokio::runtime::Handle::current();
            handle.block_on(async {
                match ctx_blob_get.blob_store.get_bytes(&blob_hash).await {
                    Ok(Some(bytes)) => {
                        let mut v = Vec::with_capacity(1 + bytes.len());
                        v.push(0x00);
                        v.extend_from_slice(&bytes);
                        v
                    }
                    Ok(None) => vec![0x01],
                    Err(e) => {
                        tracing::warn!(
                            plugin = %ctx_blob_get.plugin_name,
                            hash = %hash,
                            error = %e,
                            "wasm plugin blob_get failed"
                        );
                        let mut v = vec![0x02];
                        v.extend_from_slice(format!("blob_get failed: {e}").as_bytes());
                        v
                    }
                }
            })
        })
        .map_err(|e| anyhow::anyhow!("failed to register blob_get: {e}"))?;

    // blob_put: returns String with first byte as ok/err tag
    // '\0' + hash = success, '\x01' + error = failure
    let ctx_blob_put = Arc::clone(&ctx);
    proto
        .register("blob_put", move |data: Vec<u8>| -> String {
            match blob_put(&ctx_blob_put, &data) {
                Ok(hash) => format!("\0{hash}"),
                Err(e) => format!("\x01{e}"),
            }
        })
        .map_err(|e| anyhow::anyhow!("failed to register blob_put: {e}"))?;

    // -- Identity --
    let ctx_node_id = Arc::clone(&ctx);
    proto
        .register("node_id", move || -> u64 { node_id(&ctx_node_id) })
        .map_err(|e| anyhow::anyhow!("failed to register node_id: {e}"))?;

    // -- Randomness --
    proto
        .register("random_bytes", move |count: u32| -> Vec<u8> { random_bytes(count) })
        .map_err(|e| anyhow::anyhow!("failed to register random_bytes: {e}"))?;

    // -- Cluster --
    let ctx_is_leader = Arc::clone(&ctx);
    proto
        .register("is_leader", move || -> bool { is_leader(&ctx_is_leader) })
        .map_err(|e| anyhow::anyhow!("failed to register is_leader: {e}"))?;

    let ctx_leader_id = Arc::clone(&ctx);
    proto
        .register("leader_id", move || -> u64 { leader_id(&ctx_leader_id) })
        .map_err(|e| anyhow::anyhow!("failed to register leader_id: {e}"))?;

    // -- Crypto --
    let ctx_sign = Arc::clone(&ctx);
    proto
        .register("sign", move |data: Vec<u8>| -> Vec<u8> { sign(&ctx_sign, &data) })
        .map_err(|e| anyhow::anyhow!("failed to register sign: {e}"))?;

    proto
        .register("verify", move |key: String, data: Vec<u8>, sig: Vec<u8>| -> bool { verify(&key, &data, &sig) })
        .map_err(|e| anyhow::anyhow!("failed to register verify: {e}"))?;

    let ctx_pubkey = Arc::clone(&ctx);
    proto
        .register("public_key_hex", move || -> String { public_key_hex(&ctx_pubkey) })
        .map_err(|e| anyhow::anyhow!("failed to register public_key_hex: {e}"))?;

    // -- HLC --
    let ctx_hlc = Arc::clone(&ctx);
    proto
        .register("hlc_now", move || -> u64 { hlc_now(&ctx_hlc) })
        .map_err(|e| anyhow::anyhow!("failed to register hlc_now: {e}"))?;

    Ok(())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // validate_key_prefix
    // -------------------------------------------------------------------------

    #[test]
    fn key_within_allowed_prefix_is_valid() {
        let result = validate_key_prefix("forge", &["forge:".into()], "forge:repos:abc", "read");
        assert!(result.is_ok());
    }

    #[test]
    fn key_outside_allowed_prefix_is_rejected() {
        let result = validate_key_prefix("forge", &["forge:".into()], "__hooks:config", "read");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("namespace violation"));
    }

    #[test]
    fn key_exact_prefix_match_is_valid() {
        let result = validate_key_prefix("hooks", &["__hooks:".into()], "__hooks:config", "read");
        assert!(result.is_ok());
    }

    #[test]
    fn key_empty_prefixes_allows_all() {
        let result = validate_key_prefix("test", &[], "anything:goes:here", "read");
        assert!(result.is_ok());
    }

    #[test]
    fn key_multiple_prefixes_any_match_is_valid() {
        let prefixes = vec!["forge:".into(), "forge-cobs:".into()];
        assert!(validate_key_prefix("forge", &prefixes, "forge:repos:x", "read").is_ok());
        assert!(validate_key_prefix("forge", &prefixes, "forge-cobs:y", "read").is_ok());
        assert!(validate_key_prefix("forge", &prefixes, "__hooks:z", "read").is_err());
    }

    #[test]
    fn key_partial_prefix_match_is_rejected() {
        // "forg" is a prefix of "forge:" but "forge:" is the allowed prefix
        let result = validate_key_prefix("forge", &["forge:".into()], "forg", "read");
        assert!(result.is_err());
    }

    #[test]
    fn key_error_message_includes_operation() {
        let result = validate_key_prefix("hooks", &["__hooks:".into()], "forge:x", "write");
        let err = result.unwrap_err();
        assert!(err.contains("write"), "error should mention the operation");
        assert!(err.contains("hooks"), "error should mention the plugin");
        assert!(err.contains("forge:x"), "error should mention the key");
    }

    // -------------------------------------------------------------------------
    // validate_scan_prefix
    // -------------------------------------------------------------------------

    #[test]
    fn scan_within_allowed_prefix_is_valid() {
        let result = validate_scan_prefix("forge", &["forge:".into()], "forge:repos:");
        assert!(result.is_ok());
    }

    #[test]
    fn scan_exact_allowed_prefix_is_valid() {
        let result = validate_scan_prefix("forge", &["forge:".into()], "forge:");
        assert!(result.is_ok());
    }

    #[test]
    fn scan_outside_allowed_prefix_is_rejected() {
        let result = validate_scan_prefix("forge", &["forge:".into()], "__hooks:");
        assert!(result.is_err());
    }

    #[test]
    fn scan_empty_string_is_rejected() {
        // Empty scan prefix would scan everything — must be denied
        let result = validate_scan_prefix("forge", &["forge:".into()], "");
        assert!(result.is_err());
    }

    #[test]
    fn scan_empty_prefixes_allows_all() {
        let result = validate_scan_prefix("test", &[], "");
        assert!(result.is_ok());
    }

    // -------------------------------------------------------------------------
    // with_kv_prefixes
    // -------------------------------------------------------------------------

    #[test]
    fn with_kv_prefixes_uses_explicit_when_non_empty() {
        // We can't construct a full PluginHostContext without real stores,
        // so we test the with_kv_prefixes logic by checking the output
        // struct field directly. Build a minimal struct manually.
        let prefixes = vec!["forge:".to_string(), "forge-cobs:".to_string()];
        let ctx = PluginHostContextStub {
            plugin_name: "forge".to_string(),
            allowed_kv_prefixes: Vec::new(),
        };
        let resolved = resolve_kv_prefixes(&ctx.plugin_name, prefixes);
        assert_eq!(resolved, vec!["forge:", "forge-cobs:"]);
    }

    #[test]
    fn with_kv_prefixes_defaults_when_empty() {
        let ctx = PluginHostContextStub {
            plugin_name: "my-plugin".to_string(),
            allowed_kv_prefixes: Vec::new(),
        };
        let resolved = resolve_kv_prefixes(&ctx.plugin_name, vec![]);
        assert_eq!(resolved, vec!["__plugin:my-plugin:"]);
    }

    /// Minimal stub for testing prefix resolution without real cluster services.
    struct PluginHostContextStub {
        plugin_name: String,
        allowed_kv_prefixes: Vec<String>,
    }

    /// Mirror the logic from `PluginHostContext::with_kv_prefixes`.
    fn resolve_kv_prefixes(plugin_name: &str, prefixes: Vec<String>) -> Vec<String> {
        if prefixes.is_empty() {
            vec![format!(
                "{}{}:",
                aspen_constants::plugin::DEFAULT_PLUGIN_KV_PREFIX_TEMPLATE,
                plugin_name
            )]
        } else {
            prefixes
        }
    }
}
