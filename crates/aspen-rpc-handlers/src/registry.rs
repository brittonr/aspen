//! Handler registry for dispatching client RPC requests.
//!
//! This module provides a modular handler architecture that decomposes the monolithic
//! `process_client_request` function into focused, domain-specific handlers.
//!
//! # Plugin Architecture
//!
//! All handlers self-register using the `inventory` crate via `HandlerFactory`.
//! The registry collects all registered factories, checks runtime preconditions,
//! and creates handler instances sorted by priority.
//!
//! # Hot-Reload
//!
//! The handler list is stored behind an [`arc_swap::ArcSwap`] so that WASM plugin
//! handlers can be atomically replaced at runtime without restarting the node.
//! Native handlers (registered via `inventory`) are fixed at startup; only the
//! WASM plugin portion is swappable.

use std::sync::Arc;

use arc_swap::ArcSwap;
use aspen_client_api::CapabilityUnavailableResponse;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_core::app_registry::AppManifest;
pub use aspen_rpc_core::HandlerFactory;
pub use aspen_rpc_core::RequestHandler;
// Re-export RequestHandler from aspen-rpc-core for handlers to implement
pub use aspen_rpc_core::collect_handler_factories;
use tracing::debug;
use tracing::info;
use tracing::trace;
use tracing::warn;

use crate::context::ClientProtocolContext;
use crate::proxy::ProxyService;

/// Registry of request handlers with hot-reload support.
///
/// Dispatches requests to the appropriate handler based on request type.
/// Handlers are tried in order; the first handler that `can_handle` the
/// request will process it.
///
/// The handler list is stored behind [`ArcSwap`] so that WASM plugin handlers
/// can be atomically replaced at runtime via [`swap_plugin_handlers`].
///
/// # Tiger Style
///
/// - Bounded number of handlers (statically known + MAX_PLUGINS)
/// - O(n) dispatch where n is small and fixed
/// - Clear error when no handler matches
/// - Lock-free dispatch via ArcSwap (readers never block)
/// - Clone-friendly for sharing across async tasks
#[derive(Clone)]
pub struct HandlerRegistry {
    /// Combined handler list (native + plugin), atomically swappable.
    ///
    /// Readers load a snapshot via `ArcSwap::load()` — this is lock-free
    /// and wait-free. Writers replace the list atomically via `store()`.
    handlers: Arc<ArcSwap<Vec<Arc<dyn RequestHandler>>>>,
    /// Native handlers cached for rebuilding the list during hot-reload.
    /// These never change after construction.
    native_handlers: Arc<Vec<(Arc<dyn RequestHandler>, u32)>>,
    /// Optional WASM plugin registry for hot-reload support.
    #[cfg(feature = "plugins-rpc")]
    plugin_registry: Option<Arc<aspen_wasm_plugin::LivePluginRegistry>>,
    proxy_service: Option<Arc<ProxyService>>,
}

impl HandlerRegistry {
    /// Create a new handler registry with all domain handlers.
    ///
    /// All handlers self-register via `HandlerFactory` + `submit_handler_factory!`
    /// and are collected via `inventory`. Each factory's `create()` method checks
    /// runtime preconditions (e.g., whether required services are available).
    ///
    /// Handlers are sorted by priority (lower = checked first) before being stored.
    pub fn new(ctx: &ClientProtocolContext) -> Self {
        // Collect handlers with their priorities for sorting
        let mut handlers_with_priority: Vec<(Arc<dyn RequestHandler>, u32)> = Vec::new();

        // All handlers self-register via submit_handler_factory! and are collected here.
        // Each factory checks runtime preconditions in create() and returns None if
        // the handler should not be registered (e.g., required service unavailable).
        let plugin_factories = collect_handler_factories();
        for factory in plugin_factories {
            match factory.create(ctx) {
                Some(handler) => {
                    trace!(factory = factory.name(), priority = factory.priority(), "handler registered via inventory");
                    handlers_with_priority.push((handler, factory.priority()));

                    // Auto-register app capability when handler has an app_id
                    if let Some(app_id) = factory.app_id() {
                        let manifest = AppManifest::new(app_id, env!("CARGO_PKG_VERSION"));
                        ctx.app_registry.register(manifest);
                    }
                }
                None => {
                    trace!(factory = factory.name(), "handler factory skipped (preconditions not met)");
                }
            }
        }

        // Sort by priority (lower = checked first)
        handlers_with_priority.sort_by_key(|(_, priority)| *priority);

        // Extract handlers in sorted order for the initial list
        let handlers: Vec<Arc<dyn RequestHandler>> =
            handlers_with_priority.iter().map(|(h, _)| Arc::clone(h)).collect();

        debug!(handler_count = handlers.len(), "handler registry initialized");

        Self {
            handlers: Arc::new(ArcSwap::from_pointee(handlers)),
            native_handlers: Arc::new(handlers_with_priority),
            #[cfg(feature = "plugins-rpc")]
            plugin_registry: None,
            proxy_service: None,
        }
    }

    /// Set the proxy service for cross-cluster request forwarding.
    ///
    /// Must be called after construction and before the registry is cloned/shared.
    pub fn with_proxy_service(&mut self, service: Arc<ProxyService>) {
        self.proxy_service = Some(service);
    }

    /// Dispatch a request to the appropriate handler.
    ///
    /// `proxy_hops` tracks how many times this request has been proxied across
    /// clusters. Pass 0 for direct client requests. The value is extracted from
    /// `AuthenticatedRequest.proxy_hops` by the client protocol handler.
    ///
    /// # Errors
    ///
    /// Returns an error if no handler can process the request.
    pub async fn dispatch(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
        proxy_hops: u8,
    ) -> anyhow::Result<ClientRpcResponse> {
        // Handle plugin reload directly — it requires access to the registry itself,
        // which individual handlers don't have.
        #[cfg(feature = "plugins-rpc")]
        if let ClientRpcRequest::PluginReload { ref name } = request {
            return self.handle_plugin_reload(name.clone(), ctx).await;
        }

        // Load the current handler snapshot (lock-free, wait-free)
        let handlers = self.handlers.load();

        for handler in handlers.iter() {
            if handler.can_handle(&request) {
                debug!(
                    handler = handler.name(),
                    request = ?std::mem::discriminant(&request),
                    "dispatching request to handler"
                );
                return handler.handle(request, ctx).await;
            }
        }

        // No handler found - check if this is an optional app request
        if let Some(app_id) = request.required_app() {
            // Try proxying if enabled and we have a proxy service
            if ctx.proxy_config.enabled {
                if let Some(ref proxy_service) = self.proxy_service {
                    match proxy_service.proxy_request(request.clone(), app_id, proxy_hops, ctx).await {
                        Ok(Some(response)) => return Ok(response),
                        Ok(None) => {
                            // Fall through to CapabilityUnavailable
                        }
                        Err(e) => {
                            warn!(app = app_id, error = %e, "proxy attempt failed");
                            // Fall through to CapabilityUnavailable
                        }
                    }
                }
            }

            // Build hints from federation discovery if available
            let hints = {
                #[cfg(all(feature = "forge", feature = "global-discovery"))]
                {
                    use aspen_client_api::CapabilityHint;
                    use aspen_client_api::MAX_CAPABILITY_HINTS;

                    let mut h = Vec::new();
                    if let Some(ref discovery) = ctx.federation_discovery {
                        let clusters = discovery.find_clusters_with_app(app_id);
                        for cluster in clusters.into_iter().take(MAX_CAPABILITY_HINTS) {
                            let app_version = cluster.get_app(app_id).map(|m| m.version.clone());
                            h.push(CapabilityHint {
                                cluster_key: cluster.cluster_key.to_string(),
                                name: cluster.name.clone(),
                                app_version,
                            });
                        }
                    }
                    h
                }
                #[cfg(not(all(feature = "forge", feature = "global-discovery")))]
                {
                    Vec::new()
                }
            };

            return Ok(ClientRpcResponse::CapabilityUnavailable(CapabilityUnavailableResponse {
                required_app: app_id.to_string(),
                message: format!("the '{}' app is not loaded on this cluster", app_id),
                hints,
            }));
        }

        // Core request with no handler - this shouldn't happen if all request types are covered
        Err(anyhow::anyhow!("no handler found for request type: {:?}", std::mem::discriminant(&request)))
    }

    /// Add dynamically-loaded handlers (e.g., WASM plugins) to the registry.
    ///
    /// New handlers are appended after existing handlers and sorted by priority
    /// among themselves. Since WASM plugins use priority >= 900 and native handlers
    /// use priority < 900, appending maintains the global priority order.
    ///
    /// # Tiger Style
    ///
    /// - Bounded: respects MAX_PLUGINS from aspen_constants::plugin
    /// - Handlers are sorted by priority within the new batch
    /// - Logged for observability
    pub fn add_handlers(&self, mut new_handlers: Vec<(Arc<dyn RequestHandler>, u32)>) {
        let count = new_handlers.len();
        if count == 0 {
            return;
        }

        // Sort new handlers by priority
        new_handlers.sort_by_key(|(_, p)| *p);

        // Build new handler list: native + new plugin handlers
        let current = self.handlers.load();
        let mut all = (**current).clone();

        // Append new handlers (already sorted, and all have priority >= 900)
        all.extend(new_handlers.into_iter().map(|(h, _)| h));

        // Atomically swap the handler list
        self.handlers.store(Arc::new(all));
        debug!(added = count, "dynamic handlers added to registry");
    }

    /// Atomically replace all plugin handlers while preserving native handlers.
    ///
    /// Rebuilds the handler list from the fixed native handlers plus the
    /// new plugin handlers. All existing clones of this registry will see
    /// the new handlers on their next `dispatch()` call.
    ///
    /// This is the hot-reload mechanism: the caller loads new plugin handlers
    /// (via `LivePluginRegistry::reload_all`) and passes them here for atomic
    /// swap.
    ///
    /// # Tiger Style
    ///
    /// - Lock-free swap: readers never block during handler replacement
    /// - Deterministic ordering: native first (sorted by priority), then plugins
    pub fn swap_plugin_handlers(&self, mut plugin_handlers: Vec<(Arc<dyn RequestHandler>, u32)>) {
        // Sort plugin handlers by priority
        plugin_handlers.sort_by_key(|(_, p)| *p);

        // Rebuild: native handlers (already sorted) + plugin handlers (just sorted)
        let mut all: Vec<Arc<dyn RequestHandler>> = self.native_handlers.iter().map(|(h, _)| Arc::clone(h)).collect();
        all.extend(plugin_handlers.into_iter().map(|(h, _)| h));

        let total = all.len();

        // Atomic swap — all clones will see new handlers on next dispatch
        self.handlers.store(Arc::new(all));
        info!(native_count = self.native_handlers.len(), total_count = total, "handler list swapped (hot-reload)");
    }

    /// Load WASM plugin handlers from the KV store.
    ///
    /// Scans for plugin manifests, loads enabled plugins with lifecycle init,
    /// and adds them to the handler registry.
    ///
    /// The [`LivePluginRegistry`] is stored internally for subsequent hot-reload
    /// operations via [`reload_wasm_plugins`].
    #[cfg(feature = "plugins-rpc")]
    pub async fn load_wasm_plugins(&mut self, ctx: &ClientProtocolContext) -> anyhow::Result<u32> {
        let registry = Arc::new(aspen_wasm_plugin::LivePluginRegistry::new());
        // Store the registry BEFORE load_all so that reload_wasm_plugins works
        // even if the initial load fails (e.g., cluster not yet initialized).
        self.plugin_registry = Some(Arc::clone(&registry));
        let handlers = registry.load_all(ctx).await?;
        let count = handlers.len() as u32;
        self.add_handlers(handlers);
        info!(plugin_count = count, "WASM plugin handlers loaded");
        Ok(count)
    }

    /// Hot-reload all WASM plugins.
    ///
    /// Shuts down existing plugins, re-scans the KV store for manifests,
    /// loads and initializes new plugins, and atomically swaps the handler
    /// list. In-flight requests continue using the old handlers until they
    /// complete; new requests use the new handlers.
    ///
    /// Returns the number of plugins loaded after reload.
    ///
    /// # Errors
    ///
    /// Returns an error if the plugin registry was not initialized (i.e.,
    /// `load_wasm_plugins` was never called).
    #[cfg(feature = "plugins-rpc")]
    pub async fn reload_wasm_plugins(&self, ctx: &ClientProtocolContext) -> anyhow::Result<u32> {
        let registry = self
            .plugin_registry
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("plugin registry not initialized — call load_wasm_plugins first"))?;

        let handlers = registry.reload_all(ctx).await?;
        let count = handlers.len() as u32;
        self.swap_plugin_handlers(handlers);
        info!(plugin_count = count, "WASM plugins hot-reloaded");
        Ok(count)
    }

    /// Hot-reload a single WASM plugin by name.
    ///
    /// Shuts down the old plugin instance (if any), reloads from KV store,
    /// initializes, and atomically swaps the handler list.
    ///
    /// Returns `true` if the plugin was loaded, `false` if it was disabled
    /// or removed.
    #[cfg(feature = "plugins-rpc")]
    pub async fn reload_wasm_plugin(&self, name: &str, ctx: &ClientProtocolContext) -> anyhow::Result<bool> {
        let registry = self
            .plugin_registry
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("plugin registry not initialized — call load_wasm_plugins first"))?;

        let _result = registry.reload_one(name, ctx).await?;

        // Rebuild the full plugin handler list from the registry snapshot
        let all_plugin_handlers = registry.handler_snapshot().await;
        self.swap_plugin_handlers(all_plugin_handlers);

        let loaded = !registry.is_empty().await;
        info!(plugin = %name, loaded, "WASM plugin hot-reloaded");
        Ok(loaded)
    }

    /// Shut down all WASM plugins gracefully.
    ///
    /// Calls `plugin_shutdown` on each loaded plugin. Should be called
    /// during node shutdown.
    #[cfg(feature = "plugins-rpc")]
    pub async fn shutdown_wasm_plugins(&self) {
        if let Some(ref registry) = self.plugin_registry {
            registry.shutdown_all().await;
            // Remove plugin handlers from the dispatch list
            self.swap_plugin_handlers(Vec::new());
            info!("all WASM plugins shut down");
        }
    }

    /// Handle a `PluginReload` request.
    ///
    /// Dispatched directly from `dispatch()` because it requires access
    /// to the registry itself (not available to individual handlers).
    #[cfg(feature = "plugins-rpc")]
    async fn handle_plugin_reload(
        &self,
        name: Option<String>,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        use aspen_client_api::PluginReloadResultResponse;

        match name {
            Some(ref plugin_name) => {
                info!(plugin = %plugin_name, "handling PluginReload request (single)");
                match self.reload_wasm_plugin(plugin_name, ctx).await {
                    Ok(loaded) => {
                        let message = if loaded {
                            format!("plugin '{}' reloaded successfully", plugin_name)
                        } else {
                            format!("plugin '{}' disabled or removed", plugin_name)
                        };
                        Ok(ClientRpcResponse::PluginReloadResult(PluginReloadResultResponse {
                            is_success: true,
                            plugin_count: if loaded { 1 } else { 0 },
                            error: None,
                            message,
                        }))
                    }
                    Err(e) => Ok(ClientRpcResponse::PluginReloadResult(PluginReloadResultResponse {
                        is_success: false,
                        plugin_count: 0,
                        error: Some(e.to_string()),
                        message: format!("failed to reload plugin '{}'", plugin_name),
                    })),
                }
            }
            None => {
                info!("handling PluginReload request (all)");
                match self.reload_wasm_plugins(ctx).await {
                    Ok(count) => Ok(ClientRpcResponse::PluginReloadResult(PluginReloadResultResponse {
                        is_success: true,
                        plugin_count: count,
                        error: None,
                        message: format!("{} plugin(s) reloaded successfully", count),
                    })),
                    Err(e) => Ok(ClientRpcResponse::PluginReloadResult(PluginReloadResultResponse {
                        is_success: false,
                        plugin_count: 0,
                        error: Some(e.to_string()),
                        message: "failed to reload plugins".to_string(),
                    })),
                }
            }
        }
    }

    /// Get health status for all loaded WASM plugins.
    #[cfg(feature = "plugins-rpc")]
    pub async fn plugin_health(&self) -> Vec<(String, aspen_wasm_plugin::PluginHealth)> {
        if let Some(ref registry) = self.plugin_registry {
            registry.health_all().await
        } else {
            Vec::new()
        }
    }

    /// Get metrics snapshots for all loaded WASM plugins.
    ///
    /// Returns (plugin_name, metrics_snapshot) pairs for every active plugin.
    /// Metrics include request counts, latency, error rates, and active
    /// in-flight requests. All data is collected lock-free via atomics.
    #[cfg(feature = "plugins-rpc")]
    pub async fn plugin_metrics(&self) -> Vec<(String, aspen_wasm_plugin::PluginMetricsSnapshot)> {
        if let Some(ref registry) = self.plugin_registry {
            registry.metrics_all().await
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal handler for testing the registry's `add_handlers` method.
    struct TestHandler {
        name: &'static str,
    }

    #[async_trait::async_trait]
    impl RequestHandler for TestHandler {
        fn name(&self) -> &'static str {
            self.name
        }

        fn can_handle(&self, _request: &ClientRpcRequest) -> bool {
            false
        }

        async fn handle(
            &self,
            _request: ClientRpcRequest,
            _ctx: &ClientProtocolContext,
        ) -> anyhow::Result<ClientRpcResponse> {
            Err(anyhow::anyhow!("not implemented"))
        }
    }

    fn empty_registry() -> HandlerRegistry {
        HandlerRegistry {
            handlers: Arc::new(ArcSwap::from_pointee(Vec::new())),
            native_handlers: Arc::new(Vec::new()),
            #[cfg(feature = "plugins-rpc")]
            plugin_registry: None,
            proxy_service: None,
        }
    }

    #[test]
    fn add_handlers_empty_vec_is_noop() {
        let registry = empty_registry();
        assert_eq!(registry.handlers.load().len(), 0);
        registry.add_handlers(vec![]);
        assert_eq!(registry.handlers.load().len(), 0);
    }

    #[test]
    fn add_handlers_sorts_by_priority() {
        let registry = empty_registry();
        let h1: Arc<dyn RequestHandler> = Arc::new(TestHandler { name: "high" });
        let h2: Arc<dyn RequestHandler> = Arc::new(TestHandler { name: "low" });

        // Insert out of order: 950 before 910
        registry.add_handlers(vec![(h1, 950), (h2, 910)]);

        let handlers = registry.handlers.load();
        assert_eq!(handlers.len(), 2);
        assert_eq!(handlers[0].name(), "low");
        assert_eq!(handlers[1].name(), "high");
    }

    #[tokio::test]
    async fn dispatch_app_request_without_handler_returns_capability_unavailable() {
        use aspen_core::EndpointProvider;

        let mock_endpoint = Arc::new(aspen_rpc_core::test_support::MockEndpointProvider::with_seed(42).await)
            as Arc<dyn EndpointProvider>;
        let ctx = aspen_rpc_core::test_support::TestContextBuilder::new().with_endpoint_manager(mock_endpoint).build();

        let registry = empty_registry();
        let request = ClientRpcRequest::ForgeCreateRepo {
            name: "test-repo".to_string(),
            description: None,
            default_branch: None,
        };

        let response = registry.dispatch(request, &ctx, 0).await.expect("dispatch should not error");
        match response {
            ClientRpcResponse::CapabilityUnavailable(ref cap) => {
                assert_eq!(cap.required_app, "forge");
                assert!(cap.hints.is_empty());
            }
            other => panic!("expected CapabilityUnavailable, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn dispatch_core_request_without_handler_returns_error() {
        use aspen_core::EndpointProvider;

        let mock_endpoint = Arc::new(aspen_rpc_core::test_support::MockEndpointProvider::with_seed(43).await)
            as Arc<dyn EndpointProvider>;
        let ctx = aspen_rpc_core::test_support::TestContextBuilder::new().with_endpoint_manager(mock_endpoint).build();

        let registry = empty_registry();
        let request = ClientRpcRequest::Ping;

        let result = registry.dispatch(request, &ctx, 0).await;
        assert!(result.is_err(), "core request with no handler should return Err, not CapabilityUnavailable");
    }

    #[tokio::test]
    async fn dispatch_routes_to_matching_handler() {
        use aspen_core::EndpointProvider;

        let mock_endpoint = Arc::new(aspen_rpc_core::test_support::MockEndpointProvider::with_seed(44).await)
            as Arc<dyn EndpointProvider>;
        let ctx = aspen_rpc_core::test_support::TestContextBuilder::new().with_endpoint_manager(mock_endpoint).build();

        // Create a handler that accepts Ping
        struct PingHandler;

        #[async_trait::async_trait]
        impl RequestHandler for PingHandler {
            fn name(&self) -> &'static str {
                "ping-handler"
            }

            fn can_handle(&self, request: &ClientRpcRequest) -> bool {
                matches!(request, ClientRpcRequest::Ping)
            }

            async fn handle(
                &self,
                _request: ClientRpcRequest,
                _ctx: &ClientProtocolContext,
            ) -> anyhow::Result<ClientRpcResponse> {
                Ok(ClientRpcResponse::Pong)
            }
        }

        let registry = empty_registry();
        let handler: Arc<dyn RequestHandler> = Arc::new(PingHandler);
        registry.add_handlers(vec![(handler, 100)]);

        let response = registry.dispatch(ClientRpcRequest::Ping, &ctx, 0).await.expect("dispatch should succeed");
        assert!(matches!(response, ClientRpcResponse::Pong), "expected Pong response");
    }

    #[test]
    fn add_handlers_appends_after_existing() {
        let existing: Arc<dyn RequestHandler> = Arc::new(TestHandler { name: "existing" });
        let registry = HandlerRegistry {
            handlers: Arc::new(ArcSwap::from_pointee(vec![existing])),
            native_handlers: Arc::new(Vec::new()),
            #[cfg(feature = "plugins-rpc")]
            plugin_registry: None,
            proxy_service: None,
        };

        let new_handler: Arc<dyn RequestHandler> = Arc::new(TestHandler { name: "new" });
        registry.add_handlers(vec![(new_handler, 920)]);

        let handlers = registry.handlers.load();
        assert_eq!(handlers.len(), 2);
        assert_eq!(handlers[0].name(), "existing");
        assert_eq!(handlers[1].name(), "new");
    }

    #[test]
    fn swap_plugin_handlers_replaces_plugins_keeps_native() {
        // Setup: native handler at priority 100
        let native: Arc<dyn RequestHandler> = Arc::new(TestHandler { name: "native" });
        let registry = HandlerRegistry {
            handlers: Arc::new(ArcSwap::from_pointee(vec![Arc::clone(&native)])),
            native_handlers: Arc::new(vec![(native, 100)]),
            #[cfg(feature = "plugins-rpc")]
            plugin_registry: None,
            proxy_service: None,
        };

        // Add initial plugin handlers
        let plugin_a: Arc<dyn RequestHandler> = Arc::new(TestHandler { name: "plugin-a" });
        registry.swap_plugin_handlers(vec![(plugin_a, 950)]);

        let handlers = registry.handlers.load();
        assert_eq!(handlers.len(), 2);
        assert_eq!(handlers[0].name(), "native");
        assert_eq!(handlers[1].name(), "plugin-a");

        // Hot-reload: replace plugin-a with plugin-b
        let plugin_b: Arc<dyn RequestHandler> = Arc::new(TestHandler { name: "plugin-b" });
        registry.swap_plugin_handlers(vec![(plugin_b, 950)]);

        let handlers = registry.handlers.load();
        assert_eq!(handlers.len(), 2);
        assert_eq!(handlers[0].name(), "native");
        assert_eq!(handlers[1].name(), "plugin-b");
    }

    #[test]
    fn swap_plugin_handlers_empty_removes_all_plugins() {
        let native: Arc<dyn RequestHandler> = Arc::new(TestHandler { name: "native" });
        let registry = HandlerRegistry {
            handlers: Arc::new(ArcSwap::from_pointee(vec![Arc::clone(&native)])),
            native_handlers: Arc::new(vec![(native, 100)]),
            #[cfg(feature = "plugins-rpc")]
            plugin_registry: None,
            proxy_service: None,
        };

        // Add a plugin
        let plugin: Arc<dyn RequestHandler> = Arc::new(TestHandler { name: "plugin" });
        registry.swap_plugin_handlers(vec![(plugin, 950)]);
        assert_eq!(registry.handlers.load().len(), 2);

        // Remove all plugins
        registry.swap_plugin_handlers(Vec::new());
        let handlers = registry.handlers.load();
        assert_eq!(handlers.len(), 1);
        assert_eq!(handlers[0].name(), "native");
    }
}

impl std::fmt::Debug for HandlerRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandlerRegistry").field("handler_count", &self.handlers.load().len()).finish()
    }
}
