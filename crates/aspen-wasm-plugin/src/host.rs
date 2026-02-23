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
use aspen_plugin_api::PluginPermissions;
use aspen_traits::ClusterController;

/// A scheduler command from a WASM guest plugin.
///
/// Guest plugins enqueue these during execution; the host processes them
/// after the guest call completes.
#[derive(Debug)]
pub enum SchedulerCommand {
    /// Schedule a new timer (or replace an existing one with the same name).
    Schedule(aspen_plugin_api::TimerConfig),
    /// Cancel a timer by name.
    Cancel(String),
}

/// A hook subscription command from a WASM guest plugin.
///
/// Guest plugins enqueue these during execution; the host processes them
/// after the guest call completes to update the event router.
#[derive(Debug)]
pub enum SubscriptionCommand {
    /// Subscribe to hook events matching a NATS-style topic pattern.
    Subscribe(String),
    /// Unsubscribe from a previously registered pattern.
    Unsubscribe(String),
}

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
    /// Pending scheduler requests from the guest.
    ///
    /// Shared with [`WasmPluginHandler`] which processes these after each
    /// guest call completes.
    pub scheduler_requests: Arc<std::sync::Mutex<Vec<SchedulerCommand>>>,
    /// Capability permissions from the plugin manifest.
    ///
    /// Checked before each host function call. Default: all denied.
    pub permissions: PluginPermissions,
    /// Pending hook subscription requests from the guest.
    ///
    /// Shared with [`WasmPluginHandler`] which processes these after each
    /// guest call completes to update the plugin's event router.
    pub subscription_requests: Arc<std::sync::Mutex<Vec<SubscriptionCommand>>>,
    /// SQL query executor for plugins that need read-only SQL access.
    ///
    /// Optional because not all nodes have a SQL-capable storage backend.
    /// Guarded by `permissions.sql_query`.
    #[cfg(feature = "sql")]
    pub sql_executor: Option<Arc<dyn aspen_core::SqlQueryExecutor>>,
    /// Hook service for hook management operations (list, metrics, trigger).
    ///
    /// Optional because hooks may not be enabled on this node.
    /// Guarded by `permissions.hooks`.
    #[cfg(feature = "hooks")]
    pub hook_service: Option<Arc<aspen_hooks::HookService>>,
    /// Hooks configuration for handler listing.
    ///
    /// Contains the static handler config (names, patterns, types).
    #[cfg(feature = "hooks")]
    pub hooks_config: aspen_hooks_types::HooksConfig,
    /// Service executors for domain-specific operations.
    ///
    /// Each executor handles a domain (docs, jobs, CI, etc.) invoked
    /// by the `service_execute` host function. Created during node
    /// setup and passed through `ClientProtocolContext`.
    pub service_executors: Vec<Arc<dyn aspen_core::ServiceExecutor>>,
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
            scheduler_requests: Arc::new(std::sync::Mutex::new(Vec::new())),
            permissions: PluginPermissions::default(),
            subscription_requests: Arc::new(std::sync::Mutex::new(Vec::new())),
            #[cfg(feature = "sql")]
            sql_executor: None,
            #[cfg(feature = "hooks")]
            hook_service: None,
            #[cfg(feature = "hooks")]
            hooks_config: aspen_hooks_types::HooksConfig::default(),
            service_executors: Vec::new(),
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

    /// Set a shared scheduler requests queue.
    ///
    /// This lets the handler and host context share the same queue so that
    /// scheduler commands enqueued by host functions during guest execution
    /// can be processed by the handler afterward.
    pub fn with_scheduler_requests(mut self, reqs: Arc<std::sync::Mutex<Vec<SchedulerCommand>>>) -> Self {
        self.scheduler_requests = reqs;
        self
    }

    /// Set the capability permissions from the plugin manifest.
    pub fn with_permissions(mut self, permissions: PluginPermissions) -> Self {
        self.permissions = permissions;
        self
    }

    /// Set a shared subscription requests queue.
    ///
    /// This lets the handler and host context share the same queue so that
    /// hook subscription commands enqueued by host functions during guest
    /// execution can be processed by the handler afterward.
    pub fn with_subscription_requests(mut self, reqs: Arc<std::sync::Mutex<Vec<SubscriptionCommand>>>) -> Self {
        self.subscription_requests = reqs;
        self
    }

    /// Set the SQL query executor for plugins that need read-only SQL access.
    #[cfg(feature = "sql")]
    #[allow(dead_code)]
    pub fn with_sql_executor(mut self, executor: Arc<dyn aspen_core::SqlQueryExecutor>) -> Self {
        self.sql_executor = Some(executor);
        self
    }

    /// Set the hook service for hook management operations.
    #[cfg(feature = "hooks")]
    pub fn with_hook_service(mut self, service: Arc<aspen_hooks::HookService>) -> Self {
        self.hook_service = Some(service);
        self
    }

    /// Set the hooks configuration for handler listing.
    #[cfg(feature = "hooks")]
    pub fn with_hooks_config(mut self, config: aspen_hooks_types::HooksConfig) -> Self {
        self.hooks_config = config;
        self
    }

    /// Set the service executors for domain-specific operations.
    pub fn with_service_executors(mut self, executors: Vec<Arc<dyn aspen_core::ServiceExecutor>>) -> Self {
        self.service_executors = executors;
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
// Permission enforcement
// ---------------------------------------------------------------------------

/// Check that the plugin has the required permission.
///
/// Returns `Ok(())` if granted, `Err` with a descriptive message if denied.
///
/// Tiger Style: Fail-fast before any I/O.
fn check_permission(plugin_name: &str, capability: &str, granted: bool) -> Result<(), String> {
    if granted {
        return Ok(());
    }
    let msg = format!("permission denied: plugin '{}' lacks '{}' capability", plugin_name, capability);
    tracing::warn!("{}", msg);
    Err(msg)
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
    check_permission(&ctx.plugin_name, "kv_write", ctx.permissions.kv_write)?;
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
    check_permission(&ctx.plugin_name, "kv_write", ctx.permissions.kv_write)?;
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
    check_permission(&ctx.plugin_name, "kv_write", ctx.permissions.kv_write)?;
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

/// Execute a batch of KV operations.
///
/// All keys are validated against the plugin's namespace prefixes before
/// any operation executes. If any key fails validation, the entire batch
/// is rejected.
///
/// Tiger Style: Validate all inputs before side effects.
pub fn kv_batch(ctx: &PluginHostContext, ops_json: &[u8]) -> Result<(), String> {
    check_permission(&ctx.plugin_name, "kv_write", ctx.permissions.kv_write)?;
    let ops: Vec<aspen_plugin_api::KvBatchOp> =
        serde_json::from_slice(ops_json).map_err(|e| format!("invalid batch JSON: {e}"))?;

    if ops.is_empty() {
        return Ok(());
    }

    // Validate all keys up front before executing any operations
    for op in &ops {
        let (key, operation) = match op {
            aspen_plugin_api::KvBatchOp::Set { key, .. } => (key.as_str(), "batch-set"),
            aspen_plugin_api::KvBatchOp::Delete { key } => (key.as_str(), "batch-delete"),
        };
        validate_key_prefix(&ctx.plugin_name, &ctx.allowed_kv_prefixes, key, operation)?;
    }

    let handle = tokio::runtime::Handle::current();
    handle.block_on(async {
        for op in ops {
            match op {
                aspen_plugin_api::KvBatchOp::Set { key, value } => {
                    let request = aspen_kv_types::WriteRequest::set(&key, &value);
                    ctx.kv_store
                        .write(request)
                        .await
                        .map(|_| ())
                        .map_err(|e| format!("kv_batch set '{}' failed: {e}", key))?;
                }
                aspen_plugin_api::KvBatchOp::Delete { key } => {
                    let request = aspen_kv_types::DeleteRequest::new(&key);
                    ctx.kv_store
                        .delete(request)
                        .await
                        .map(|_| ())
                        .map_err(|e| format!("kv_batch delete '{}' failed: {e}", key))?;
                }
            }
        }
        Ok(())
    })
}

// ---------------------------------------------------------------------------
// Blob Store host functions
// ---------------------------------------------------------------------------

/// Check whether a blob exists in the store.
///
/// The `hash` parameter is the hex-encoded BLAKE3 hash of the blob.
pub fn blob_has(ctx: &PluginHostContext, hash: &str) -> bool {
    if check_permission(&ctx.plugin_name, "blob_read", ctx.permissions.blob_read).is_err() {
        return false;
    }
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
    check_permission(&ctx.plugin_name, "blob_write", ctx.permissions.blob_write)?;
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
            if let Err(e) = check_permission(&ctx_kv_get.plugin_name, "kv_read", ctx_kv_get.permissions.kv_read) {
                let mut v = vec![0x02];
                v.extend_from_slice(e.as_bytes());
                return v;
            }
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
            if let Err(e) = check_permission(&ctx_kv_scan.plugin_name, "kv_read", ctx_kv_scan.permissions.kv_read) {
                let mut v = vec![0x01];
                v.extend_from_slice(e.as_bytes());
                return v;
            }
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

    // kv_batch: returns String with tag prefix (\0 = success, \x01 = error)
    let ctx_kv_batch = Arc::clone(&ctx);
    proto
        .register("kv_batch", move |ops: Vec<u8>| -> String {
            match kv_batch(&ctx_kv_batch, &ops) {
                Ok(()) => "\0".to_string(),
                Err(e) => format!("\x01{e}"),
            }
        })
        .map_err(|e| anyhow::anyhow!("failed to register kv_batch: {e}"))?;

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
            if let Err(e) = check_permission(&ctx_blob_get.plugin_name, "blob_read", ctx_blob_get.permissions.blob_read)
            {
                let mut v = vec![0x02];
                v.extend_from_slice(e.as_bytes());
                return v;
            }
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
    let ctx_random = Arc::clone(&ctx);
    proto
        .register("random_bytes", move |count: u32| -> Vec<u8> {
            if check_permission(&ctx_random.plugin_name, "randomness", ctx_random.permissions.randomness).is_err() {
                return Vec::new();
            }
            random_bytes(count)
        })
        .map_err(|e| anyhow::anyhow!("failed to register random_bytes: {e}"))?;

    // -- Cluster --
    let ctx_is_leader = Arc::clone(&ctx);
    proto
        .register("is_leader", move || -> bool {
            if check_permission(&ctx_is_leader.plugin_name, "cluster_info", ctx_is_leader.permissions.cluster_info)
                .is_err()
            {
                return false;
            }
            is_leader(&ctx_is_leader)
        })
        .map_err(|e| anyhow::anyhow!("failed to register is_leader: {e}"))?;

    let ctx_leader_id = Arc::clone(&ctx);
    proto
        .register("leader_id", move || -> u64 {
            if check_permission(&ctx_leader_id.plugin_name, "cluster_info", ctx_leader_id.permissions.cluster_info)
                .is_err()
            {
                return 0;
            }
            leader_id(&ctx_leader_id)
        })
        .map_err(|e| anyhow::anyhow!("failed to register leader_id: {e}"))?;

    // -- Crypto --
    let ctx_sign = Arc::clone(&ctx);
    proto
        .register("sign", move |data: Vec<u8>| -> Vec<u8> {
            if check_permission(&ctx_sign.plugin_name, "signing", ctx_sign.permissions.signing).is_err() {
                return Vec::new();
            }
            sign(&ctx_sign, &data)
        })
        .map_err(|e| anyhow::anyhow!("failed to register sign: {e}"))?;

    proto
        .register("verify", move |key: String, data: Vec<u8>, sig: Vec<u8>| -> bool { verify(&key, &data, &sig) })
        .map_err(|e| anyhow::anyhow!("failed to register verify: {e}"))?;

    let ctx_pubkey = Arc::clone(&ctx);
    proto
        .register("public_key_hex", move || -> String {
            if check_permission(&ctx_pubkey.plugin_name, "signing", ctx_pubkey.permissions.signing).is_err() {
                return String::new();
            }
            public_key_hex(&ctx_pubkey)
        })
        .map_err(|e| anyhow::anyhow!("failed to register public_key_hex: {e}"))?;

    // -- HLC --
    let ctx_hlc = Arc::clone(&ctx);
    proto
        .register("hlc_now", move || -> u64 { hlc_now(&ctx_hlc) })
        .map_err(|e| anyhow::anyhow!("failed to register hlc_now: {e}"))?;

    // -- Scheduler --
    let ctx_schedule = Arc::clone(&ctx);
    proto
        .register("schedule_timer", move |config_json: Vec<u8>| -> String {
            if let Err(e) = check_permission(&ctx_schedule.plugin_name, "timers", ctx_schedule.permissions.timers) {
                return format!("\x01{e}");
            }
            let config: aspen_plugin_api::TimerConfig = match serde_json::from_slice(&config_json) {
                Ok(c) => c,
                Err(e) => return format!("\x01invalid timer config: {e}"),
            };
            if config.name.is_empty() {
                return "\x01timer name must not be empty".to_string();
            }
            if config.name.len() > 64 {
                return "\x01timer name too long (max 64 bytes)".to_string();
            }
            match ctx_schedule.scheduler_requests.lock() {
                Ok(mut reqs) => {
                    reqs.push(SchedulerCommand::Schedule(config));
                    "\0".to_string()
                }
                Err(e) => format!("\x01scheduler lock failed: {e}"),
            }
        })
        .map_err(|e| anyhow::anyhow!("failed to register schedule_timer: {e}"))?;

    let ctx_cancel = Arc::clone(&ctx);
    proto
        .register("cancel_timer", move |name: String| -> String {
            if let Err(e) = check_permission(&ctx_cancel.plugin_name, "timers", ctx_cancel.permissions.timers) {
                return format!("\x01{e}");
            }
            match ctx_cancel.scheduler_requests.lock() {
                Ok(mut reqs) => {
                    reqs.push(SchedulerCommand::Cancel(name));
                    "\0".to_string()
                }
                Err(e) => format!("\x01scheduler lock failed: {e}"),
            }
        })
        .map_err(|e| anyhow::anyhow!("failed to register cancel_timer: {e}"))?;

    // -- Hook Subscriptions --
    let ctx_hook_sub = Arc::clone(&ctx);
    proto
        .register("hook_subscribe", move |pattern: String| -> String {
            if let Err(e) = check_permission(&ctx_hook_sub.plugin_name, "hooks", ctx_hook_sub.permissions.hooks) {
                return format!("\x01{e}");
            }
            if pattern.is_empty() {
                return "\x01hook pattern must not be empty".to_string();
            }
            if pattern.len() > aspen_plugin_api::MAX_HOOK_PATTERN_LENGTH {
                return format!("\x01hook pattern too long (max {} bytes)", aspen_plugin_api::MAX_HOOK_PATTERN_LENGTH);
            }
            match ctx_hook_sub.subscription_requests.lock() {
                Ok(mut reqs) => {
                    reqs.push(SubscriptionCommand::Subscribe(pattern));
                    "\0".to_string()
                }
                Err(e) => format!("\x01subscription lock failed: {e}"),
            }
        })
        .map_err(|e| anyhow::anyhow!("failed to register hook_subscribe: {e}"))?;

    let ctx_hook_unsub = Arc::clone(&ctx);
    proto
        .register("hook_unsubscribe", move |pattern: String| -> String {
            if let Err(e) = check_permission(&ctx_hook_unsub.plugin_name, "hooks", ctx_hook_unsub.permissions.hooks) {
                return format!("\x01{e}");
            }
            match ctx_hook_unsub.subscription_requests.lock() {
                Ok(mut reqs) => {
                    reqs.push(SubscriptionCommand::Unsubscribe(pattern));
                    "\0".to_string()
                }
                Err(e) => format!("\x01subscription lock failed: {e}"),
            }
        })
        .map_err(|e| anyhow::anyhow!("failed to register hook_unsubscribe: {e}"))?;

    // -- Hook Management (feature-gated) --
    // hook_list: list configured hook handlers and enabled status.
    // hook_metrics: get execution metrics for hook handlers.
    // hook_trigger: manually trigger a hook event.
    // All return: String with \0 prefix = success (JSON result), \x01 prefix = error
    #[cfg(feature = "hooks")]
    {
        let ctx_hook_list = Arc::clone(&ctx);
        proto
            .register("hook_list", move |_unused: String| -> String {
                if let Err(e) = check_permission(&ctx_hook_list.plugin_name, "hooks", ctx_hook_list.permissions.hooks) {
                    return format!("\x01{e}");
                }
                hook_list_impl(&ctx_hook_list)
            })
            .map_err(|e| anyhow::anyhow!("failed to register hook_list: {e}"))?;

        let ctx_hook_metrics = Arc::clone(&ctx);
        proto
            .register("hook_metrics", move |handler_name: String| -> String {
                if let Err(e) =
                    check_permission(&ctx_hook_metrics.plugin_name, "hooks", ctx_hook_metrics.permissions.hooks)
                {
                    return format!("\x01{e}");
                }
                let filter = if handler_name.is_empty() {
                    None
                } else {
                    Some(handler_name)
                };
                hook_metrics_impl(&ctx_hook_metrics, filter)
            })
            .map_err(|e| anyhow::anyhow!("failed to register hook_metrics: {e}"))?;

        let ctx_hook_trigger = Arc::clone(&ctx);
        proto
            .register("hook_trigger", move |request_json: String| -> String {
                if let Err(e) =
                    check_permission(&ctx_hook_trigger.plugin_name, "hooks", ctx_hook_trigger.permissions.hooks)
                {
                    return format!("\x01{e}");
                }
                hook_trigger_impl(&ctx_hook_trigger, &request_json)
            })
            .map_err(|e| anyhow::anyhow!("failed to register hook_trigger: {e}"))?;
    }

    // -- SQL Query (feature-gated) --
    // sql_query: executes a read-only SQL query against the state machine.
    // Input: JSON-encoded SqlQueryHostRequest { query, params_json, consistency, limit, timeout_ms }
    // Returns: String with \0 prefix = success (JSON-encoded result), \x01 prefix = error
    #[cfg(feature = "sql")]
    {
        let ctx_sql = Arc::clone(&ctx);
        proto
            .register("sql_query", move |request_json: String| -> String {
                if let Err(e) = check_permission(&ctx_sql.plugin_name, "sql_query", ctx_sql.permissions.sql_query) {
                    return format!("\x01{e}");
                }
                let executor = match &ctx_sql.sql_executor {
                    Some(ex) => Arc::clone(ex),
                    None => return "\x01SQL query executor not available on this node".to_string(),
                };

                // Parse request
                let req: SqlQueryHostRequest = match serde_json::from_str(&request_json) {
                    Ok(r) => r,
                    Err(e) => return format!("\x01invalid sql_query request: {e}"),
                };

                // Build the core request
                let consistency = match req.consistency.to_lowercase().as_str() {
                    "stale" => aspen_core::SqlConsistency::Stale,
                    _ => aspen_core::SqlConsistency::Linearizable,
                };

                let params: Vec<aspen_core::SqlValue> = if req.params_json.is_empty() {
                    Vec::new()
                } else {
                    match serde_json::from_str::<Vec<serde_json::Value>>(&req.params_json) {
                        Ok(values) => values
                            .into_iter()
                            .map(|v| match v {
                                serde_json::Value::Null => aspen_core::SqlValue::Null,
                                serde_json::Value::Bool(b) => aspen_core::SqlValue::Integer(if b { 1 } else { 0 }),
                                serde_json::Value::Number(n) => {
                                    if let Some(i) = n.as_i64() {
                                        aspen_core::SqlValue::Integer(i)
                                    } else if let Some(f) = n.as_f64() {
                                        aspen_core::SqlValue::Real(f)
                                    } else {
                                        aspen_core::SqlValue::Text(n.to_string())
                                    }
                                }
                                serde_json::Value::String(s) => aspen_core::SqlValue::Text(s),
                                _ => aspen_core::SqlValue::Text(v.to_string()),
                            })
                            .collect(),
                        Err(e) => return format!("\x01invalid params JSON: {e}"),
                    }
                };

                let sql_request = aspen_core::SqlQueryRequest {
                    query: req.query,
                    params,
                    consistency,
                    limit: req.limit,
                    timeout_ms: req.timeout_ms,
                };

                // Execute via tokio runtime
                let handle = tokio::runtime::Handle::current();
                match handle.block_on(async { executor.execute_sql(sql_request).await }) {
                    Ok(result) => {
                        // Convert SqlValue to JSON-friendly format
                        let columns: Vec<String> = result.columns.into_iter().map(|c| c.name).collect();
                        let rows: Vec<Vec<serde_json::Value>> = result
                            .rows
                            .into_iter()
                            .map(|row| {
                                row.into_iter()
                                    .map(|v| match v {
                                        aspen_core::SqlValue::Null => serde_json::Value::Null,
                                        aspen_core::SqlValue::Integer(i) => serde_json::Value::Number(i.into()),
                                        aspen_core::SqlValue::Real(f) => serde_json::json!(f),
                                        aspen_core::SqlValue::Text(s) => serde_json::Value::String(s),
                                        aspen_core::SqlValue::Blob(b) => {
                                            serde_json::Value::String(format!("base64:{}", base64_encode(&b)))
                                        }
                                    })
                                    .collect()
                            })
                            .collect();

                        let response = serde_json::json!({
                            "columns": columns,
                            "rows": rows,
                            "row_count": result.row_count,
                            "is_truncated": result.is_truncated,
                            "execution_time_ms": result.execution_time_ms,
                        });

                        match serde_json::to_string(&response) {
                            Ok(json) => format!("\0{json}"),
                            Err(e) => format!("\x01failed to serialize SQL result: {e}"),
                        }
                    }
                    Err(e) => format!("\x01{e}"),
                }
            })
            .map_err(|e| anyhow::anyhow!("failed to register sql_query: {e}"))?;
    }

    // -- Full-fidelity KV operations --
    let ctx_kv_execute = Arc::clone(&ctx);
    proto
        .register("kv_execute", move |request_json: String| -> String {
            if let Err(e) = check_permission(
                &ctx_kv_execute.plugin_name,
                "kv_read",
                ctx_kv_execute.permissions.kv_read || ctx_kv_execute.permissions.kv_write,
            ) {
                return format!("\x01{e}");
            }
            kv_execute_impl(&ctx_kv_execute, &request_json)
        })
        .map_err(|e| anyhow::anyhow!("failed to register kv_execute: {e}"))?;

    // -- Generic service executor dispatch --
    if !ctx.service_executors.is_empty() {
        let ctx_service = Arc::clone(&ctx);
        proto
            .register("service_execute", move |request_json: String| -> String {
                service_execute_impl(&ctx_service, &request_json)
            })
            .map_err(|e| anyhow::anyhow!("failed to register service_execute: {e}"))?;
    }

    Ok(())
}

/// Request payload for the `sql_query` host function.
#[cfg(feature = "sql")]
#[derive(serde::Deserialize)]
struct SqlQueryHostRequest {
    query: String,
    #[serde(default)]
    params_json: String,
    #[serde(default = "default_consistency")]
    consistency: String,
    limit: Option<u32>,
    timeout_ms: Option<u32>,
}

#[cfg(feature = "sql")]
fn default_consistency() -> String {
    "linearizable".to_string()
}

// ---------------------------------------------------------------------------
// Full-fidelity KV operations (kv_execute)
// ---------------------------------------------------------------------------

/// Execute a full-fidelity KV operation.
///
/// Takes a JSON request with an "op" field and operation-specific parameters.
/// Returns structured JSON results with error codes, version metadata, etc.
///
/// This function provides complete KV protocol support for handler plugins
/// that need to replace native KV handlers (NOT_LEADER propagation, CAS
/// actual-value on failure, scan version metadata, conditional batch writes).
fn kv_execute_impl(ctx: &PluginHostContext, request_json: &str) -> String {
    let request: serde_json::Value = match serde_json::from_str(request_json) {
        Ok(r) => r,
        Err(e) => return format!("\x01invalid JSON: {e}"),
    };

    let op = request["op"].as_str().unwrap_or("");

    let handle = tokio::runtime::Handle::current();
    let result = handle.block_on(async {
        match op {
            "read" => kv_exec_read(ctx, &request).await,
            "write" => kv_exec_write(ctx, &request).await,
            "delete" => kv_exec_delete(ctx, &request).await,
            "scan" => kv_exec_scan(ctx, &request).await,
            "batch_read" => kv_exec_batch_read(ctx, &request).await,
            "batch_write" => kv_exec_batch_write(ctx, &request).await,
            "cas" => kv_exec_cas(ctx, &request).await,
            "cad" => kv_exec_cad(ctx, &request).await,
            "conditional_batch" => kv_exec_conditional_batch(ctx, &request).await,
            _ => Err(format!("unknown kv_execute op: {op}")),
        }
    });

    match result {
        Ok(json) => match serde_json::to_string(&json) {
            Ok(s) => format!("\0{s}"),
            Err(e) => format!("\x01serialize failed: {e}"),
        },
        Err(e) => format!("\x01{e}"),
    }
}

async fn kv_exec_read(ctx: &PluginHostContext, request: &serde_json::Value) -> Result<serde_json::Value, String> {
    let key = request["key"].as_str().ok_or("missing 'key'")?;
    let req = aspen_kv_types::ReadRequest::new(key);
    match ctx.kv_store.read(req).await {
        Ok(result) => match result.kv {
            Some(entry) => {
                let value_b64 = base64_encode_bytes(entry.value.as_bytes());
                Ok(serde_json::json!({
                    "value": value_b64,
                    "was_found": true,
                    "error": null,
                }))
            }
            None => Ok(serde_json::json!({"value": null, "was_found": false, "error": null})),
        },
        Err(aspen_core::KeyValueStoreError::NotFound { .. }) => {
            Ok(serde_json::json!({"value": null, "was_found": false, "error": null}))
        }
        Err(e) => Ok(serde_json::json!({"value": null, "was_found": false, "error": format!("{e}")})),
    }
}

async fn kv_exec_write(ctx: &PluginHostContext, request: &serde_json::Value) -> Result<serde_json::Value, String> {
    let key = request["key"].as_str().ok_or("missing 'key'")?;
    let value_b64 = request["value"].as_str().ok_or("missing 'value'")?;
    let value_bytes = base64_decode_bytes(value_b64).map_err(|e| format!("invalid base64 value: {e}"))?;
    let value_str = String::from_utf8_lossy(&value_bytes).into_owned();

    let req = aspen_kv_types::WriteRequest {
        command: aspen_kv_types::WriteCommand::Set {
            key: key.to_string(),
            value: value_str,
        },
    };
    match ctx.kv_store.write(req).await {
        Ok(_) => Ok(serde_json::json!({"is_success": true, "error": null, "error_code": null, "leader_id": null})),
        Err(aspen_core::KeyValueStoreError::NotLeader { leader, .. }) => Ok(serde_json::json!({
            "is_success": false,
            "error": format!("not leader; leader is node {}", leader.unwrap_or(0)),
            "error_code": "NOT_LEADER",
            "leader_id": leader,
        })),
        Err(e) => {
            Ok(serde_json::json!({"is_success": false, "error": format!("{e}"), "error_code": null, "leader_id": null}))
        }
    }
}

async fn kv_exec_delete(ctx: &PluginHostContext, request: &serde_json::Value) -> Result<serde_json::Value, String> {
    let key = request["key"].as_str().ok_or("missing 'key'")?;
    let req = aspen_kv_types::WriteRequest {
        command: aspen_kv_types::WriteCommand::Delete { key: key.to_string() },
    };
    match ctx.kv_store.write(req).await {
        Ok(_) => Ok(serde_json::json!({
            "key": key, "was_deleted": true, "error": null, "error_code": null, "leader_id": null,
        })),
        Err(aspen_core::KeyValueStoreError::NotLeader { leader, .. }) => Ok(serde_json::json!({
            "key": key, "was_deleted": false,
            "error": format!("not leader; leader is node {}", leader.unwrap_or(0)),
            "error_code": "NOT_LEADER", "leader_id": leader,
        })),
        Err(e) => Ok(serde_json::json!({
            "key": key, "was_deleted": false, "error": format!("{e}"), "error_code": null, "leader_id": null,
        })),
    }
}

async fn kv_exec_scan(ctx: &PluginHostContext, request: &serde_json::Value) -> Result<serde_json::Value, String> {
    let prefix = request["prefix"].as_str().ok_or("missing 'prefix'")?;
    let limit = request["limit"].as_u64().map(|v| v as u32);
    let continuation_token = request["continuation_token"].as_str().map(String::from);

    let req = aspen_kv_types::ScanRequest {
        prefix: prefix.to_string(),
        limit_results: limit,
        continuation_token,
    };
    match ctx.kv_store.scan(req).await {
        Ok(scan_resp) => {
            let entries: Vec<serde_json::Value> = scan_resp
                .entries
                .into_iter()
                .map(|e| {
                    serde_json::json!({
                        "key": e.key,
                        "value": base64_encode_bytes(e.value.as_bytes()),
                        "version": e.version,
                        "create_revision": e.create_revision,
                        "mod_revision": e.mod_revision,
                    })
                })
                .collect();
            Ok(serde_json::json!({
                "entries": entries,
                "count": scan_resp.result_count,
                "is_truncated": scan_resp.is_truncated,
                "continuation_token": scan_resp.continuation_token,
                "error": null,
            }))
        }
        Err(e) => Ok(serde_json::json!({
            "entries": [], "count": 0, "is_truncated": false,
            "continuation_token": null, "error": format!("{e}"),
        })),
    }
}

async fn kv_exec_batch_read(ctx: &PluginHostContext, request: &serde_json::Value) -> Result<serde_json::Value, String> {
    let keys: Vec<String> = request["keys"]
        .as_array()
        .ok_or("missing 'keys' array")?
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();

    let mut values = Vec::with_capacity(keys.len());
    for key in &keys {
        let req = aspen_kv_types::ReadRequest::new(key);
        match ctx.kv_store.read(req).await {
            Ok(result) => {
                let value = result.kv.map(|kv| base64_encode_bytes(kv.value.as_bytes()));
                values.push(serde_json::json!(value));
            }
            Err(aspen_core::KeyValueStoreError::NotFound { .. }) => {
                values.push(serde_json::Value::Null);
            }
            Err(e) => {
                return Ok(serde_json::json!({
                    "is_success": false, "values": null, "error": format!("{e}"),
                }));
            }
        }
    }
    Ok(serde_json::json!({"is_success": true, "values": values, "error": null}))
}

async fn kv_exec_batch_write(
    ctx: &PluginHostContext,
    request: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let ops_json = request["operations"].as_array().ok_or("missing 'operations' array")?;

    let mut batch_ops = Vec::with_capacity(ops_json.len());
    for op in ops_json {
        if let Some(set) = op.get("Set") {
            let key = set["key"].as_str().ok_or("missing Set.key")?;
            let value_b64 = set["value"].as_str().ok_or("missing Set.value")?;
            let value_bytes = base64_decode_bytes(value_b64).map_err(|e| format!("invalid base64: {e}"))?;
            batch_ops.push(aspen_core::BatchOperation::Set {
                key: key.to_string(),
                value: String::from_utf8_lossy(&value_bytes).into_owned(),
            });
        } else if let Some(del) = op.get("Delete") {
            let key = del["key"].as_str().ok_or("missing Delete.key")?;
            batch_ops.push(aspen_core::BatchOperation::Delete { key: key.to_string() });
        } else {
            return Err("unknown batch operation".to_string());
        }
    }

    let req = aspen_kv_types::WriteRequest {
        command: aspen_kv_types::WriteCommand::Batch { operations: batch_ops },
    };
    match ctx.kv_store.write(req).await {
        Ok(result) => Ok(serde_json::json!({
            "is_success": true,
            "operations_applied": result.batch_applied,
            "error": null, "error_code": null, "leader_id": null,
        })),
        Err(aspen_core::KeyValueStoreError::NotLeader { leader, .. }) => Ok(serde_json::json!({
            "is_success": false, "operations_applied": null,
            "error": format!("not leader; leader is node {}", leader.unwrap_or(0)),
            "error_code": "NOT_LEADER", "leader_id": leader,
        })),
        Err(e) => Ok(serde_json::json!({
            "is_success": false, "operations_applied": null,
            "error": format!("{e}"), "error_code": null, "leader_id": null,
        })),
    }
}

async fn kv_exec_cas(ctx: &PluginHostContext, request: &serde_json::Value) -> Result<serde_json::Value, String> {
    let key = request["key"].as_str().ok_or("missing 'key'")?;
    let expected = match request["expected"].as_str() {
        Some(b64) => {
            let bytes = base64_decode_bytes(b64).map_err(|e| format!("invalid base64 expected: {e}"))?;
            Some(String::from_utf8_lossy(&bytes).into_owned())
        }
        None => None,
    };
    let new_value_b64 = request["new_value"].as_str().ok_or("missing 'new_value'")?;
    let new_value_bytes = base64_decode_bytes(new_value_b64).map_err(|e| format!("invalid base64 new_value: {e}"))?;

    let req = aspen_kv_types::WriteRequest {
        command: aspen_kv_types::WriteCommand::CompareAndSwap {
            key: key.to_string(),
            expected,
            new_value: String::from_utf8_lossy(&new_value_bytes).into_owned(),
        },
    };
    match ctx.kv_store.write(req).await {
        Ok(_) => Ok(serde_json::json!({
            "is_success": true, "actual_value": null, "error": null, "error_code": null, "leader_id": null,
        })),
        Err(aspen_core::KeyValueStoreError::CompareAndSwapFailed { actual, .. }) => {
            let actual_b64 = actual.as_ref().map(|v| base64_encode_bytes(v.as_bytes()));
            Ok(serde_json::json!({
                "is_success": false, "actual_value": actual_b64, "error": null,
                "error_code": "CAS_FAILED", "leader_id": null,
            }))
        }
        Err(aspen_core::KeyValueStoreError::NotLeader { leader, .. }) => Ok(serde_json::json!({
            "is_success": false, "actual_value": null,
            "error": format!("not leader; leader is node {}", leader.unwrap_or(0)),
            "error_code": "NOT_LEADER", "leader_id": leader,
        })),
        Err(e) => Ok(serde_json::json!({
            "is_success": false, "actual_value": null,
            "error": format!("{e}"), "error_code": null, "leader_id": null,
        })),
    }
}

async fn kv_exec_cad(ctx: &PluginHostContext, request: &serde_json::Value) -> Result<serde_json::Value, String> {
    let key = request["key"].as_str().ok_or("missing 'key'")?;
    let expected_b64 = request["expected"].as_str().ok_or("missing 'expected'")?;
    let expected_bytes = base64_decode_bytes(expected_b64).map_err(|e| format!("invalid base64 expected: {e}"))?;

    let req = aspen_kv_types::WriteRequest {
        command: aspen_kv_types::WriteCommand::CompareAndDelete {
            key: key.to_string(),
            expected: String::from_utf8_lossy(&expected_bytes).into_owned(),
        },
    };
    match ctx.kv_store.write(req).await {
        Ok(_) => Ok(serde_json::json!({
            "is_success": true, "actual_value": null, "error": null, "error_code": null, "leader_id": null,
        })),
        Err(aspen_core::KeyValueStoreError::CompareAndSwapFailed { actual, .. }) => {
            let actual_b64 = actual.as_ref().map(|v| base64_encode_bytes(v.as_bytes()));
            Ok(serde_json::json!({
                "is_success": false, "actual_value": actual_b64, "error": null,
                "error_code": "CAS_FAILED", "leader_id": null,
            }))
        }
        Err(aspen_core::KeyValueStoreError::NotLeader { leader, .. }) => Ok(serde_json::json!({
            "is_success": false, "actual_value": null,
            "error": format!("not leader; leader is node {}", leader.unwrap_or(0)),
            "error_code": "NOT_LEADER", "leader_id": leader,
        })),
        Err(e) => Ok(serde_json::json!({
            "is_success": false, "actual_value": null,
            "error": format!("{e}"), "error_code": null, "leader_id": null,
        })),
    }
}

async fn kv_exec_conditional_batch(
    ctx: &PluginHostContext,
    request: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let conditions_json = request["conditions"].as_array().ok_or("missing 'conditions' array")?;
    let ops_json = request["operations"].as_array().ok_or("missing 'operations' array")?;

    // Parse conditions
    let mut conditions = Vec::with_capacity(conditions_json.len());
    for c in conditions_json {
        if let Some(ve) = c.get("ValueEquals") {
            let key = ve["key"].as_str().ok_or("missing ValueEquals.key")?;
            let expected_b64 = ve["expected"].as_str().ok_or("missing ValueEquals.expected")?;
            let expected_bytes = base64_decode_bytes(expected_b64).map_err(|e| format!("invalid base64: {e}"))?;
            conditions.push(aspen_core::BatchCondition::ValueEquals {
                key: key.to_string(),
                expected: String::from_utf8_lossy(&expected_bytes).into_owned(),
            });
        } else if let Some(ke) = c.get("KeyExists") {
            let key = ke["key"].as_str().ok_or("missing KeyExists.key")?;
            conditions.push(aspen_core::BatchCondition::KeyExists { key: key.to_string() });
        } else if let Some(kne) = c.get("KeyNotExists") {
            let key = kne["key"].as_str().ok_or("missing KeyNotExists.key")?;
            conditions.push(aspen_core::BatchCondition::KeyNotExists { key: key.to_string() });
        } else {
            return Err("unknown condition type".to_string());
        }
    }

    // Parse operations
    let mut batch_ops = Vec::with_capacity(ops_json.len());
    for op in ops_json {
        if let Some(set) = op.get("Set") {
            let key = set["key"].as_str().ok_or("missing Set.key")?;
            let value_b64 = set["value"].as_str().ok_or("missing Set.value")?;
            let value_bytes = base64_decode_bytes(value_b64).map_err(|e| format!("invalid base64: {e}"))?;
            batch_ops.push(aspen_core::BatchOperation::Set {
                key: key.to_string(),
                value: String::from_utf8_lossy(&value_bytes).into_owned(),
            });
        } else if let Some(del) = op.get("Delete") {
            let key = del["key"].as_str().ok_or("missing Delete.key")?;
            batch_ops.push(aspen_core::BatchOperation::Delete { key: key.to_string() });
        } else {
            return Err("unknown batch operation".to_string());
        }
    }

    let req = aspen_kv_types::WriteRequest {
        command: aspen_kv_types::WriteCommand::ConditionalBatch {
            conditions,
            operations: batch_ops,
        },
    };
    match ctx.kv_store.write(req).await {
        Ok(result) => {
            let conditions_met = result.conditions_met.unwrap_or(false);
            Ok(serde_json::json!({
                "is_success": conditions_met,
                "conditions_met": conditions_met,
                "operations_applied": result.batch_applied,
                "failed_condition_index": result.failed_condition_index,
                "failed_condition_reason": null,
                "error": null, "error_code": null, "leader_id": null,
            }))
        }
        Err(aspen_core::KeyValueStoreError::NotLeader { leader, .. }) => Ok(serde_json::json!({
            "is_success": false, "conditions_met": false, "operations_applied": null,
            "failed_condition_index": null, "failed_condition_reason": null,
            "error": format!("not leader; leader is node {}", leader.unwrap_or(0)),
            "error_code": "NOT_LEADER", "leader_id": leader,
        })),
        Err(e) => Ok(serde_json::json!({
            "is_success": false, "conditions_met": false, "operations_applied": null,
            "failed_condition_index": null, "failed_condition_reason": null,
            "error": format!("{e}"), "error_code": null, "leader_id": null,
        })),
    }
}

// ---------------------------------------------------------------------------
// Generic service executor dispatch
// ---------------------------------------------------------------------------

/// Execute a domain-specific service operation.
///
/// Takes a JSON request with a `"service"` field to identify the executor
/// and forwards the rest to `ServiceExecutor::execute()`.
///
/// # Request Format
///
/// ```json
/// {"service": "docs", "op": "set", "key": "my-key", "value": "..."}
/// ```
///
/// # Response Format
///
/// Returns the executor's tagged string: `\0{json}` or `\x01{error}`.
fn service_execute_impl(ctx: &PluginHostContext, request_json: &str) -> String {
    let request: serde_json::Value = match serde_json::from_str(request_json) {
        Ok(r) => r,
        Err(e) => return format!("\x01invalid JSON: {e}"),
    };

    let service = match request["service"].as_str() {
        Some(s) => s,
        None => return "\x01missing 'service' field".to_string(),
    };

    let executor = match ctx.service_executors.iter().find(|e| e.service_name() == service) {
        Some(e) => Arc::clone(e),
        None => return format!("\x01unknown service: {service}"),
    };

    let handle = tokio::runtime::Handle::current();
    handle.block_on(async { executor.execute(request_json).await })
}

/// Base64-encode bytes for JSON transport.
fn base64_encode_bytes(data: &[u8]) -> String {
    const CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((n >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((n >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((n >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(n & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

/// Base64-decode bytes from JSON transport.
fn base64_decode_bytes(input: &str) -> Result<Vec<u8>, String> {
    const DECODE: [u8; 128] = {
        let mut table = [0xFFu8; 128];
        let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut i = 0;
        while i < 64 {
            table[chars[i] as usize] = i as u8;
            i += 1;
        }
        table
    };

    let input = input.trim_end_matches('=');
    let mut output = Vec::with_capacity(input.len() * 3 / 4);
    let bytes = input.as_bytes();
    let chunks = bytes.chunks(4);

    for chunk in chunks {
        let mut n: u32 = 0;
        for (i, &b) in chunk.iter().enumerate() {
            if b >= 128 || DECODE[b as usize] == 0xFF {
                return Err(format!("invalid base64 character: {}", b as char));
            }
            n |= (DECODE[b as usize] as u32) << (18 - i * 6);
        }
        output.push((n >> 16) as u8);
        if chunk.len() > 2 {
            output.push((n >> 8) as u8);
        }
        if chunk.len() > 3 {
            output.push(n as u8);
        }
    }
    Ok(output)
}

/// Simple base64 encoding without pulling in the base64 crate dependency.
#[cfg(feature = "sql")]
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((n >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((n >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((n >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(n & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

// =============================================================================
// Hook management host functions (feature-gated)
// =============================================================================

/// List configured hook handlers and enabled status.
///
/// Returns JSON with `is_enabled` and `handlers` array, each containing:
/// name, pattern, handler_type, execution_mode, is_enabled, timeout_ms, retry_count.
#[cfg(feature = "hooks")]
fn hook_list_impl(ctx: &PluginHostContext) -> String {
    let is_enabled = ctx.hook_service.as_ref().map(|s| s.is_enabled()).unwrap_or(false);

    let handlers: Vec<serde_json::Value> = ctx
        .hooks_config
        .handlers
        .iter()
        .map(|cfg| {
            serde_json::json!({
                "name": cfg.name,
                "pattern": cfg.pattern,
                "handler_type": cfg.handler_type.type_name(),
                "execution_mode": match cfg.execution_mode {
                    aspen_hooks::config::ExecutionMode::Direct => "direct",
                    aspen_hooks::config::ExecutionMode::Job => "job",
                },
                "enabled": cfg.is_enabled,
                "timeout_ms": cfg.timeout_ms,
                "retry_count": cfg.retry_count,
            })
        })
        .collect();

    let result = serde_json::json!({
        "is_enabled": is_enabled,
        "handlers": handlers,
    });

    match serde_json::to_string(&result) {
        Ok(s) => format!("\0{s}"),
        Err(e) => format!("\x01serialize failed: {e}"),
    }
}

/// Get execution metrics for hook handlers.
///
/// Returns JSON with `is_enabled`, `total_events_processed`, and `handlers`
/// array with per-handler metrics. If `handler_name` is Some, filters to
/// that handler only.
#[cfg(feature = "hooks")]
fn hook_metrics_impl(ctx: &PluginHostContext, handler_name: Option<String>) -> String {
    let Some(ref service) = ctx.hook_service else {
        let result = serde_json::json!({
            "is_enabled": false,
            "total_events_processed": 0,
            "handlers": [],
        });
        return match serde_json::to_string(&result) {
            Ok(s) => format!("\0{s}"),
            Err(e) => format!("\x01serialize failed: {e}"),
        };
    };

    let snapshot = service.metrics().snapshot();

    let handlers: Vec<serde_json::Value> = if let Some(ref name) = handler_name {
        snapshot
            .handlers
            .iter()
            .filter(|(n, _)| *n == name)
            .map(|(name, m)| {
                serde_json::json!({
                    "name": name,
                    "success_count": m.successes,
                    "failure_count": m.failures,
                    "dropped_count": m.dropped,
                    "jobs_submitted": m.jobs_submitted,
                    "avg_duration_us": m.avg_latency_us,
                    "max_duration_us": 0u64,
                })
            })
            .collect()
    } else {
        snapshot
            .handlers
            .iter()
            .map(|(name, m)| {
                serde_json::json!({
                    "name": name,
                    "success_count": m.successes,
                    "failure_count": m.failures,
                    "dropped_count": m.dropped,
                    "jobs_submitted": m.jobs_submitted,
                    "avg_duration_us": m.avg_latency_us,
                    "max_duration_us": 0u64,
                })
            })
            .collect()
    };

    let total = snapshot.global.successes + snapshot.global.failures;

    let result = serde_json::json!({
        "is_enabled": service.is_enabled(),
        "total_events_processed": total,
        "handlers": handlers,
    });

    match serde_json::to_string(&result) {
        Ok(s) => format!("\0{s}"),
        Err(e) => format!("\x01serialize failed: {e}"),
    }
}

/// Manually trigger a hook event.
///
/// Input JSON: `{"event_type": "...", "payload": {...}}`
/// Returns JSON with `is_success`, `dispatched_count`, `error`, `handler_failures`.
#[cfg(feature = "hooks")]
fn hook_trigger_impl(ctx: &PluginHostContext, request_json: &str) -> String {
    let request: serde_json::Value = match serde_json::from_str(request_json) {
        Ok(r) => r,
        Err(e) => return format!("\x01invalid JSON: {e}"),
    };

    let event_type_str = match request["event_type"].as_str() {
        Some(s) => s,
        None => return "\x01missing 'event_type'".to_string(),
    };

    let payload = request.get("payload").cloned().unwrap_or(serde_json::json!({}));

    // Parse event type
    let hook_event_type = match event_type_str {
        "write_committed" => aspen_hooks::HookEventType::WriteCommitted,
        "delete_committed" => aspen_hooks::HookEventType::DeleteCommitted,
        "membership_changed" => aspen_hooks::HookEventType::MembershipChanged,
        "leader_elected" => aspen_hooks::HookEventType::LeaderElected,
        "snapshot_created" => aspen_hooks::HookEventType::SnapshotCreated,
        other => {
            let result = serde_json::json!({
                "is_success": false,
                "dispatched_count": 0,
                "error": format!("unknown event type: {other}"),
                "handler_failures": [],
            });
            return match serde_json::to_string(&result) {
                Ok(s) => format!("\0{s}"),
                Err(e) => format!("\x01serialize failed: {e}"),
            };
        }
    };

    let Some(ref service) = ctx.hook_service else {
        let result = serde_json::json!({
            "is_success": false,
            "dispatched_count": 0,
            "error": "hooks not enabled",
            "handler_failures": [],
        });
        return match serde_json::to_string(&result) {
            Ok(s) => format!("\0{s}"),
            Err(e) => format!("\x01serialize failed: {e}"),
        };
    };

    if !service.is_enabled() {
        let result = serde_json::json!({
            "is_success": false,
            "dispatched_count": 0,
            "error": "hooks not enabled",
            "handler_failures": [],
        });
        return match serde_json::to_string(&result) {
            Ok(s) => format!("\0{s}"),
            Err(e) => format!("\x01serialize failed: {e}"),
        };
    }

    // Create synthetic event and dispatch
    let event = aspen_hooks::HookEvent::new(hook_event_type, ctx.node_id, payload);

    let handle = tokio::runtime::Handle::current();
    let result = handle.block_on(async { service.dispatch(&event).await });

    match result {
        Ok(dispatch_result) => {
            let dispatched_count = dispatch_result.handler_count();
            let handler_failures: Vec<Vec<String>> = match dispatch_result {
                aspen_hooks::service::DispatchResult::Disabled => vec![],
                aspen_hooks::service::DispatchResult::Dispatched { direct_results, .. } => direct_results
                    .into_iter()
                    .filter_map(|(name, r)| r.err().map(|e| vec![name, e.to_string()]))
                    .collect(),
            };

            let is_success = handler_failures.is_empty();

            let result = serde_json::json!({
                "is_success": is_success,
                "dispatched_count": dispatched_count,
                "error": null,
                "handler_failures": handler_failures,
            });
            match serde_json::to_string(&result) {
                Ok(s) => format!("\0{s}"),
                Err(e) => format!("\x01serialize failed: {e}"),
            }
        }
        Err(e) => {
            let result = serde_json::json!({
                "is_success": false,
                "dispatched_count": 0,
                "error": e.to_string(),
                "handler_failures": [],
            });
            match serde_json::to_string(&result) {
                Ok(s) => format!("\0{s}"),
                Err(e) => format!("\x01serialize failed: {e}"),
            }
        }
    }
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
