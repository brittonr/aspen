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

use std::sync::Arc;

use aspen_client_api::CapabilityUnavailableResponse;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_core::app_registry::AppManifest;
pub use aspen_rpc_core::HandlerFactory;
pub use aspen_rpc_core::RequestHandler;
// Re-export RequestHandler from aspen-rpc-core for handlers to implement
pub use aspen_rpc_core::collect_handler_factories;
use tracing::debug;
use tracing::trace;

use crate::context::ClientProtocolContext;

/// Registry of request handlers.
///
/// Dispatches requests to the appropriate handler based on request type.
/// Handlers are tried in order; the first handler that `can_handle` the
/// request will process it.
///
/// # Tiger Style
///
/// - Bounded number of handlers (statically known)
/// - O(n) dispatch where n is small and fixed
/// - Clear error when no handler matches
/// - Clone-friendly for sharing across async tasks
#[derive(Clone)]
pub struct HandlerRegistry {
    handlers: Arc<Vec<Arc<dyn RequestHandler>>>,
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

        // Extract handlers in sorted order
        let handlers: Vec<Arc<dyn RequestHandler>> = handlers_with_priority.into_iter().map(|(h, _)| h).collect();

        debug!(handler_count = handlers.len(), "handler registry initialized");

        Self {
            handlers: Arc::new(handlers),
        }
    }

    /// Dispatch a request to the appropriate handler.
    ///
    /// # Errors
    ///
    /// Returns an error if no handler can process the request.
    pub async fn dispatch(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        for handler in self.handlers.iter() {
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
    /// Must be called before the registry is cloned/shared across async tasks.
    ///
    /// # Tiger Style
    ///
    /// - Bounded: respects MAX_PLUGINS from aspen_constants::plugin
    /// - Handlers are sorted by priority within the new batch
    /// - Logged for observability
    pub fn add_handlers(&mut self, mut new_handlers: Vec<(Arc<dyn RequestHandler>, u32)>) {
        let count = new_handlers.len();
        if count == 0 {
            return;
        }

        // Sort new handlers by priority
        new_handlers.sort_by_key(|(_, p)| *p);

        // Get mutable access to handlers vec - this works because we're called during init
        // before the Arc is cloned
        let existing = std::mem::take(&mut self.handlers);
        let mut all = Arc::try_unwrap(existing).unwrap_or_else(|arc| (*arc).clone());

        // Append new handlers (already sorted, and all have priority >= 900)
        all.extend(new_handlers.into_iter().map(|(h, _)| h));

        self.handlers = Arc::new(all);
        debug!(added = count, total = self.handlers.len(), "dynamic handlers added to registry");
    }

    /// Load WASM plugin handlers from the KV store.
    ///
    /// Scans for plugin manifests, loads enabled plugins, and adds them
    /// to the handler registry. Must be called after `new()` and before
    /// the registry is shared.
    #[cfg(feature = "wasm-plugins")]
    pub async fn load_wasm_plugins(&mut self, ctx: &ClientProtocolContext) -> anyhow::Result<u32> {
        let handlers = aspen_wasm_plugin::PluginRegistry::load_all(ctx).await?;
        let count = handlers.len() as u32;
        self.add_handlers(handlers);
        tracing::info!(plugin_count = count, "WASM plugin handlers loaded");
        Ok(count)
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
            handlers: Arc::new(Vec::new()),
        }
    }

    #[test]
    fn add_handlers_empty_vec_is_noop() {
        let mut registry = empty_registry();
        assert_eq!(registry.handlers.len(), 0);
        registry.add_handlers(vec![]);
        assert_eq!(registry.handlers.len(), 0);
    }

    #[test]
    fn add_handlers_sorts_by_priority() {
        let mut registry = empty_registry();
        let h1: Arc<dyn RequestHandler> = Arc::new(TestHandler { name: "high" });
        let h2: Arc<dyn RequestHandler> = Arc::new(TestHandler { name: "low" });

        // Insert out of order: 950 before 910
        registry.add_handlers(vec![(h1, 950), (h2, 910)]);

        assert_eq!(registry.handlers.len(), 2);
        assert_eq!(registry.handlers[0].name(), "low");
        assert_eq!(registry.handlers[1].name(), "high");
    }

    #[test]
    fn add_handlers_appends_after_existing() {
        let existing: Arc<dyn RequestHandler> = Arc::new(TestHandler { name: "existing" });
        let mut registry = HandlerRegistry {
            handlers: Arc::new(vec![existing]),
        };

        let new_handler: Arc<dyn RequestHandler> = Arc::new(TestHandler { name: "new" });
        registry.add_handlers(vec![(new_handler, 920)]);

        assert_eq!(registry.handlers.len(), 2);
        assert_eq!(registry.handlers[0].name(), "existing");
        assert_eq!(registry.handlers[1].name(), "new");
    }
}

impl std::fmt::Debug for HandlerRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandlerRegistry").field("handler_count", &self.handlers.len()).finish()
    }
}
