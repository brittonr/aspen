//! Plugin registry for discovering and loading WASM handler plugins.
//!
//! Scans the cluster KV store for plugin manifests, fetches WASM binaries
//! from blob storage, creates sandboxed handlers, and validates that the
//! guest's reported identity matches the stored manifest.

use std::sync::Arc;

use aspen_blob::prelude::*;
use aspen_plugin_api::PluginManifest;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::RequestHandler;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::handler::WasmPluginHandler;
use crate::host::PluginHostContext;
use crate::host::register_plugin_host_functions;

/// Registry that discovers and loads WASM handler plugins from the cluster store.
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
        let blob_store =
            ctx.blob_store.as_ref().ok_or_else(|| anyhow::anyhow!("blob store required for WASM plugins"))?;

        let mut handlers: Vec<(Arc<dyn RequestHandler>, u32)> = Vec::new();

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
            return Ok(handlers);
        }

        info!(manifest_count = scan_result.entries.len(), "found WASM plugin manifests");

        let secret_key = ctx.endpoint_manager.endpoint().secret_key().clone();
        let hlc = Arc::new(aspen_core::hlc::create_hlc(&ctx.node_id.to_string()));

        for entry in scan_result.entries {
            match load_plugin(ctx, blob_store, &entry.key, &entry.value, &secret_key, &hlc).await {
                Ok(Some((handler, priority))) => {
                    handlers.push((handler, priority));
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

        Ok(handlers)
    }
}

/// Load a single WASM plugin from a manifest stored in the KV store.
///
/// Returns `Ok(None)` if the plugin is disabled. Returns `Err` if loading fails.
async fn load_plugin(
    ctx: &ClientProtocolContext,
    blob_store: &Arc<aspen_blob::IrohBlobStore>,
    key: &str,
    manifest_json: &str,
    secret_key: &iroh::SecretKey,
    hlc: &Arc<aspen_core::HLC>,
) -> anyhow::Result<Option<(Arc<dyn RequestHandler>, u32)>> {
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

    // Register host functions
    let host_ctx = Arc::new(
        PluginHostContext::new(
            ctx.kv_store.clone(),
            Arc::clone(blob_store) as Arc<dyn BlobStore>,
            ctx.controller.clone(),
            ctx.node_id,
            manifest.name.clone(),
        )
        .with_secret_key(secret_key.clone())
        .with_hlc(Arc::clone(hlc)),
    );
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

    let handler = WasmPluginHandler::new(manifest.name.clone(), manifest.handles.clone(), loaded);

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

    info!(
        plugin = %manifest.name,
        version = %manifest.version,
        priority,
        handles = ?manifest.handles,
        app_id = ?manifest.app_id,
        "WASM plugin loaded successfully"
    );

    Ok(Some((Arc::new(handler) as Arc<dyn RequestHandler>, priority)))
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
