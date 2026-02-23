//! Plugin registry for discovering and loading WASM handler plugins.
//!
//! Scans the cluster KV store for plugin manifests, fetches WASM binaries
//! from blob storage, creates sandboxed handlers, and validates that the
//! guest's reported identity matches the stored manifest.
//!
//! ## Lifecycle
//!
//! The [`LivePluginRegistry`] maintains the set of loaded plugins and supports
//! hot-reload. During loading, each plugin goes through:
//!
//! 1. **Loading** – WASM binary fetched and sandbox created
//! 2. **Initializing** – guest `plugin_init` export is called
//! 3. **Ready** – plugin is dispatching requests
//!
//! On reload, old plugins receive `plugin_shutdown` before being replaced.

use std::collections::HashMap;
use std::sync::Arc;

use aspen_blob::prelude::*;
use aspen_plugin_api::PluginHealth;
use aspen_plugin_api::PluginManifest;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::RequestHandler;
use tokio::sync::RwLock;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::handler::WasmPluginHandler;
use crate::host::PluginHostContext;
use crate::host::register_plugin_host_functions;

/// Stateful registry that tracks loaded WASM plugins and supports hot-reload.
///
/// Maintains a map of plugin name → handler so that individual plugins can
/// be reloaded, shut down, or health-checked without affecting others.
///
/// # Tiger Style
///
/// - Bounded: respects `MAX_PLUGINS` from constants
/// - Explicit lifecycle: init → ready → shutdown
/// - Graceful degradation: broken plugins are skipped, not fatal
pub struct LivePluginRegistry {
    /// Loaded plugin handlers keyed by plugin name.
    plugins: RwLock<HashMap<String, LoadedPlugin>>,
}

/// A loaded plugin with its handler, priority, and manifest metadata.
struct LoadedPlugin {
    handler: Arc<WasmPluginHandler>,
    priority: u32,
    /// Stored for future version comparison on reload.
    #[allow(dead_code)]
    manifest: PluginManifest,
}

impl Default for LivePluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl LivePluginRegistry {
    /// Create a new empty plugin registry.
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
        }
    }

    /// Load all enabled WASM handler plugins from the cluster KV store.
    ///
    /// Scans `PLUGIN_KV_PREFIX` for manifests, fetches WASM bytes from blob store,
    /// creates sandboxed handlers, and calls `plugin_init` on each.
    ///
    /// Returns handler/priority pairs suitable for `HandlerRegistry::add_handlers`.
    ///
    /// Plugins that fail to load or initialize are logged and skipped rather
    /// than causing a fatal error.
    pub async fn load_all(&self, ctx: &ClientProtocolContext) -> anyhow::Result<Vec<(Arc<dyn RequestHandler>, u32)>> {
        let blob_store =
            ctx.blob_store.as_ref().ok_or_else(|| anyhow::anyhow!("blob store required for WASM plugins"))?;

        let scan_request = aspen_kv_types::ScanRequest {
            prefix: aspen_constants::plugin::PLUGIN_KV_PREFIX.to_string(),
            limit_results: Some(aspen_constants::plugin::MAX_PLUGINS),
            continuation_token: None,
        };

        let scan_result = ctx
            .kv_store
            .scan(scan_request)
            .await
            .map_err(|e| anyhow::anyhow!("failed to scan plugin manifests: {e}"))?;

        if scan_result.entries.is_empty() {
            debug!("no WASM plugin manifests found");
            return Ok(Vec::new());
        }

        info!(manifest_count = scan_result.entries.len(), "found WASM plugin manifests");

        let secret_key = ctx.endpoint_manager.endpoint().secret_key().clone();
        let hlc = Arc::new(aspen_core::hlc::create_hlc(&ctx.node_id.to_string()));

        let mut result_handlers: Vec<(Arc<dyn RequestHandler>, u32)> = Vec::new();
        let mut new_plugins: HashMap<String, LoadedPlugin> = HashMap::new();

        for entry in scan_result.entries {
            match load_plugin(ctx, blob_store, &entry.key, &entry.value, &secret_key, &hlc).await {
                Ok(Some((handler, priority, manifest))) => {
                    // Call plugin_init lifecycle hook
                    match handler.call_init().await {
                        Ok(()) => {
                            info!(
                                plugin = %handler.plugin_name(),
                                "plugin initialized successfully"
                            );
                            let name = handler.plugin_name().to_string();
                            result_handlers.push((Arc::clone(&handler) as Arc<dyn RequestHandler>, priority));
                            new_plugins.insert(name, LoadedPlugin {
                                handler,
                                priority,
                                manifest,
                            });
                        }
                        Err(e) => {
                            warn!(
                                plugin = %handler.plugin_name(),
                                error = %e,
                                "plugin init failed, skipping"
                            );
                        }
                    }
                }
                Ok(None) => {
                    // Plugin disabled or skipped
                }
                Err(e) => {
                    warn!(
                        key = %entry.key,
                        error = %e,
                        "failed to load WASM plugin, skipping"
                    );
                }
            }
        }

        // Store loaded plugins
        let mut plugins = self.plugins.write().await;
        *plugins = new_plugins;

        Ok(result_handlers)
    }

    /// Reload all WASM plugins by shutting down old ones and loading new ones.
    ///
    /// Returns the new handler/priority pairs for `HandlerRegistry::swap_plugin_handlers`.
    ///
    /// Old plugins receive `plugin_shutdown` before being replaced.
    /// This is the hot-reload entry point.
    pub async fn reload_all(&self, ctx: &ClientProtocolContext) -> anyhow::Result<Vec<(Arc<dyn RequestHandler>, u32)>> {
        info!("reloading all WASM plugins");

        // Shut down existing plugins
        self.shutdown_all().await;

        // Load fresh set
        self.load_all(ctx).await
    }

    /// Reload a single plugin by name.
    ///
    /// Shuts down the old instance (if any), reloads from the KV store,
    /// and returns the new handler/priority pair.
    ///
    /// Returns `Ok(None)` if the plugin is disabled or no longer exists.
    pub async fn reload_one(
        &self,
        name: &str,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<Option<(Arc<dyn RequestHandler>, u32)>> {
        info!(plugin = %name, "reloading single WASM plugin");

        let blob_store =
            ctx.blob_store.as_ref().ok_or_else(|| anyhow::anyhow!("blob store required for WASM plugins"))?;

        // Read the manifest from KV
        let key = format!("{}{}", aspen_constants::plugin::PLUGIN_KV_PREFIX, name);
        let read_request = aspen_kv_types::ReadRequest::new(&key);
        let read_result = ctx
            .kv_store
            .read(read_request)
            .await
            .map_err(|e| anyhow::anyhow!("failed to read plugin manifest: {e}"))?;

        let manifest_json = match read_result.kv {
            Some(entry) => entry.value,
            None => {
                // Plugin removed from KV — shut down old instance
                let mut plugins = self.plugins.write().await;
                if let Some(old) = plugins.remove(name)
                    && let Err(e) = old.handler.call_shutdown().await
                {
                    warn!(plugin = %name, error = %e, "error shutting down removed plugin");
                }
                return Ok(None);
            }
        };

        // Shut down old instance
        {
            let mut plugins = self.plugins.write().await;
            if let Some(old) = plugins.remove(name) {
                info!(plugin = %name, "shutting down old plugin instance");
                if let Err(e) = old.handler.call_shutdown().await {
                    warn!(plugin = %name, error = %e, "error shutting down old plugin");
                }
            }
        }

        let secret_key = ctx.endpoint_manager.endpoint().secret_key().clone();
        let hlc = Arc::new(aspen_core::hlc::create_hlc(&ctx.node_id.to_string()));

        match load_plugin(ctx, blob_store, &key, &manifest_json, &secret_key, &hlc).await {
            Ok(Some((handler, priority, manifest))) => {
                // Call plugin_init
                handler.call_init().await?;

                info!(
                    plugin = %handler.plugin_name(),
                    "plugin reloaded and initialized successfully"
                );

                let result = (Arc::clone(&handler) as Arc<dyn RequestHandler>, priority);

                // Store in registry
                let mut plugins = self.plugins.write().await;
                plugins.insert(name.to_string(), LoadedPlugin {
                    handler,
                    priority,
                    manifest,
                });

                Ok(Some(result))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Shut down all loaded plugins gracefully.
    ///
    /// Calls `plugin_shutdown` on each loaded plugin. Errors are logged
    /// but do not prevent other plugins from shutting down.
    pub async fn shutdown_all(&self) {
        let mut plugins = self.plugins.write().await;
        for (name, loaded) in plugins.drain() {
            info!(plugin = %name, "shutting down plugin");
            if let Err(e) = loaded.handler.call_shutdown().await {
                warn!(plugin = %name, error = %e, "error during plugin shutdown");
            }
        }
    }

    /// Get health status for all loaded plugins.
    pub async fn health_all(&self) -> Vec<(String, PluginHealth)> {
        let plugins = self.plugins.read().await;
        let mut results = Vec::with_capacity(plugins.len());
        for (name, loaded) in plugins.iter() {
            let health = loaded.handler.call_health().await;
            results.push((name.clone(), health));
        }
        results
    }

    /// Get health status for a specific plugin.
    pub async fn health_one(&self, name: &str) -> Option<PluginHealth> {
        let plugins = self.plugins.read().await;
        if let Some(loaded) = plugins.get(name) {
            Some(loaded.handler.call_health().await)
        } else {
            None
        }
    }

    /// Get a snapshot of all loaded handler/priority pairs.
    ///
    /// Used by `HandlerRegistry` to rebuild the handler list during hot-reload.
    pub async fn handler_snapshot(&self) -> Vec<(Arc<dyn RequestHandler>, u32)> {
        let plugins = self.plugins.read().await;
        plugins
            .values()
            .map(|loaded| (Arc::clone(&loaded.handler) as Arc<dyn RequestHandler>, loaded.priority))
            .collect()
    }

    /// Number of loaded plugins.
    pub async fn len(&self) -> usize {
        self.plugins.read().await.len()
    }

    /// Whether any plugins are loaded.
    pub async fn is_empty(&self) -> bool {
        self.plugins.read().await.is_empty()
    }
}

/// Legacy stateless registry for backward compatibility.
///
/// Prefer [`LivePluginRegistry`] for new code — it supports lifecycle
/// management and hot-reload.
pub struct PluginRegistry;

impl PluginRegistry {
    /// Load all enabled WASM handler plugins from the cluster KV store.
    ///
    /// Scans `PLUGIN_KV_PREFIX` for manifests, fetches WASM bytes from blob store,
    /// and creates sandboxed handlers. Returns a vec of `(handler, priority)` pairs.
    ///
    /// Plugins that fail to load are logged and skipped rather than causing a
    /// fatal error. This ensures one broken plugin does not prevent the node
    /// from starting.
    pub async fn load_all(ctx: &ClientProtocolContext) -> anyhow::Result<Vec<(Arc<dyn RequestHandler>, u32)>> {
        let registry = LivePluginRegistry::new();
        registry.load_all(ctx).await
    }
}

/// Load a single WASM plugin from a manifest stored in the KV store.
///
/// Returns `Ok(None)` if the plugin is disabled. Returns `Err` if loading fails.
/// On success returns the handler, priority, and parsed manifest.
///
/// **Note:** The returned handler is in `Loading` state. The caller must call
/// `handler.call_init()` to transition it to `Ready` before use.
async fn load_plugin(
    ctx: &ClientProtocolContext,
    blob_store: &Arc<aspen_blob::IrohBlobStore>,
    key: &str,
    manifest_json: &str,
    secret_key: &iroh::SecretKey,
    hlc: &Arc<aspen_core::HLC>,
) -> anyhow::Result<Option<(Arc<WasmPluginHandler>, u32, PluginManifest)>> {
    let manifest: PluginManifest =
        serde_json::from_str(manifest_json).map_err(|e| anyhow::anyhow!("invalid manifest at '{key}': {e}"))?;

    if !manifest.enabled {
        debug!(plugin = %manifest.name, "plugin disabled, skipping");
        return Ok(None);
    }

    info!(
        plugin = %manifest.name,
        version = %manifest.version,
        "loading WASM plugin"
    );

    // Fetch WASM bytes from blob store
    let blob_hash = manifest
        .wasm_hash
        .parse::<iroh_blobs::Hash>()
        .map_err(|e| anyhow::anyhow!("invalid wasm_hash '{}': {e}", manifest.wasm_hash))?;

    let bytes = blob_store
        .get_bytes(&blob_hash)
        .await
        .map_err(|e| anyhow::anyhow!("failed to fetch WASM blob: {e}"))?
        .ok_or_else(|| anyhow::anyhow!("WASM blob not found: {}", manifest.wasm_hash))?;

    // Validate size
    if bytes.len() as u64 > aspen_constants::wasm::MAX_WASM_COMPONENT_SIZE {
        return Err(anyhow::anyhow!(
            "WASM plugin '{}' too large: {} bytes (max {})",
            manifest.name,
            bytes.len(),
            aspen_constants::wasm::MAX_WASM_COMPONENT_SIZE
        ));
    }

    let memory_limit = manifest
        .memory_limit
        .unwrap_or(aspen_plugin_api::PLUGIN_DEFAULT_MEMORY)
        .min(aspen_constants::wasm::MAX_WASM_MEMORY_LIMIT);

    // Create sandbox
    let mut proto = hyperlight_wasm::SandboxBuilder::new()
        .with_guest_heap_size(memory_limit)
        .build()
        .map_err(|e| anyhow::anyhow!("failed to create sandbox for '{}': {e}", manifest.name))?;

    // Create shared request queues for host↔handler communication
    let scheduler_requests = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let subscription_requests = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

    // Register host functions
    #[allow(unused_mut)]
    let mut host_ctx_val = PluginHostContext::new(
        ctx.kv_store.clone(),
        Arc::clone(blob_store) as Arc<dyn BlobStore>,
        ctx.controller.clone(),
        ctx.node_id,
        manifest.name.clone(),
    )
    .with_kv_prefixes(manifest.kv_prefixes.clone())
    .with_secret_key(secret_key.clone())
    .with_hlc(Arc::clone(hlc))
    .with_scheduler_requests(Arc::clone(&scheduler_requests))
    .with_subscription_requests(Arc::clone(&subscription_requests))
    .with_permissions(manifest.permissions.clone());

    // Wire SQL executor if available (feature-gated by sql on rpc-core)
    #[cfg(feature = "sql")]
    {
        host_ctx_val.sql_executor = Some(ctx.sql_executor.clone());
    }

    // Wire hook service + config if available (feature-gated by hooks)
    #[cfg(feature = "hooks")]
    {
        if let Some(ref hook_service) = ctx.hook_service {
            host_ctx_val = host_ctx_val.with_hook_service(Arc::clone(hook_service));
        }
        host_ctx_val = host_ctx_val.with_hooks_config(ctx.hooks_config.clone());
    }

    let host_ctx = Arc::new(host_ctx_val);
    register_plugin_host_functions(&mut proto, host_ctx)?;

    // Load runtime and module
    let wasm_sb = proto
        .load_runtime()
        .map_err(|e| anyhow::anyhow!("failed to load WASM runtime for '{}': {e}", manifest.name))?;

    let mut loaded = wasm_sb
        .load_module_from_buffer(&bytes)
        .map_err(|e| anyhow::anyhow!("failed to load WASM module for '{}': {e}", manifest.name))?;

    // Verify plugin info matches manifest
    let info_bytes: Vec<u8> = loaded
        .call_guest_function("plugin_info", Vec::<u8>::new())
        .map_err(|e| anyhow::anyhow!("failed to call plugin_info for '{}': {e}", manifest.name))?;
    let info: aspen_plugin_api::PluginInfo = serde_json::from_slice(&info_bytes)
        .map_err(|e| anyhow::anyhow!("invalid plugin_info from '{}': {e}", manifest.name))?;

    if info.name != manifest.name {
        return Err(anyhow::anyhow!(
            "plugin name mismatch: manifest says '{}', guest says '{}'",
            manifest.name,
            info.name
        ));
    }

    let priority = manifest
        .priority
        .clamp(aspen_constants::plugin::MIN_PLUGIN_PRIORITY, aspen_constants::plugin::MAX_PLUGIN_PRIORITY);

    let execution_timeout = {
        let secs = manifest
            .execution_timeout_secs
            .unwrap_or(aspen_constants::wasm::DEFAULT_WASM_EXECUTION_TIMEOUT_SECS)
            .min(aspen_constants::wasm::MAX_WASM_EXECUTION_TIMEOUT_SECS);
        std::time::Duration::from_secs(secs)
    };

    let handler = Arc::new(WasmPluginHandler::new_with_scheduler(
        manifest.name.clone(),
        manifest.handles.clone(),
        loaded,
        execution_timeout,
        scheduler_requests,
        subscription_requests,
    ));

    // Register app capability with the federation app registry
    if let Some(ref app_id) = manifest.app_id {
        let app_manifest = aspen_core::app_registry::AppManifest::new(app_id, &manifest.version);
        ctx.app_registry.register(app_manifest);
        info!(
            plugin = %manifest.name,
            app_id = %app_id,
            "registered WASM plugin app capability"
        );
    }

    // Log resolved KV prefixes for observability. The host_ctx has already
    // resolved empty kv_prefixes to the default `__plugin:{name}:` prefix.
    let resolved_prefixes = &manifest.kv_prefixes;
    info!(
        plugin = %manifest.name,
        version = %manifest.version,
        priority,
        handles = ?manifest.handles,
        app_id = ?manifest.app_id,
        kv_prefixes = ?resolved_prefixes,
        execution_timeout_secs = execution_timeout.as_secs(),
        "WASM plugin loaded successfully"
    );

    Ok(Some((handler, priority, manifest)))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use aspen_core::app_registry::AppManifest;
    use aspen_core::app_registry::AppRegistry;
    use aspen_plugin_api::PluginManifest;

    #[test]
    fn plugin_manifest_with_app_id_registers_capability() {
        let manifest = PluginManifest {
            name: "forge".to_string(),
            version: "1.0.0".to_string(),
            wasm_hash: String::new(),
            handles: vec!["ForgeCreateRepo".to_string()],
            priority: 950,
            fuel_limit: None,
            memory_limit: None,
            enabled: true,
            app_id: Some("forge".to_string()),
            execution_timeout_secs: None,
            kv_prefixes: vec!["forge:".to_string()],
            permissions: aspen_plugin_api::PluginPermissions::all(),
            signature: None,
        };

        let registry = Arc::new(AppRegistry::new());

        // Mirror the registration logic from load_plugin
        if let Some(ref app_id) = manifest.app_id {
            let app_manifest = AppManifest::new(app_id, &manifest.version);
            registry.register(app_manifest);
        }

        assert!(registry.has_app("forge"), "forge app should be registered");
        let app = registry.get_app("forge").expect("forge app should exist");
        assert_eq!(app.version, "1.0.0");
    }

    #[test]
    fn plugin_manifest_without_app_id_skips_registration() {
        let manifest = PluginManifest {
            name: "echo-plugin".to_string(),
            version: "0.1.0".to_string(),
            wasm_hash: String::new(),
            handles: vec!["Ping".to_string()],
            priority: 950,
            fuel_limit: None,
            memory_limit: None,
            enabled: true,
            app_id: None,
            execution_timeout_secs: None,
            kv_prefixes: vec![],
            permissions: aspen_plugin_api::PluginPermissions::default(),
            signature: None,
        };

        let registry = Arc::new(AppRegistry::new());

        // Mirror the registration logic from load_plugin
        if let Some(ref app_id) = manifest.app_id {
            let app_manifest = AppManifest::new(app_id, &manifest.version);
            registry.register(app_manifest);
        }

        assert!(registry.is_empty(), "registry should remain empty when app_id is None");
    }
}
