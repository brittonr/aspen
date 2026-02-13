//! Handler registry for dispatching client RPC requests.
//!
//! This module provides a modular handler architecture that decomposes the monolithic
//! `process_client_request` function into focused, domain-specific handlers.
//!
//! # Plugin Architecture
//!
//! Handlers can self-register using the `inventory` crate via `HandlerFactory`.
//! The registry collects all registered factories and creates handler instances
//! based on the runtime context.
//!
//! Handlers registered via inventory are combined with built-in handlers and
//! sorted by priority before dispatch.

use std::sync::Arc;

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
// Re-export RequestHandler from aspen-rpc-core for handlers to implement
pub use aspen_rpc_core::HandlerFactory;
pub use aspen_rpc_core::RequestHandler;
pub use aspen_rpc_core::collect_handler_factories;
use tracing::debug;
use tracing::trace;

use crate::context::ClientProtocolContext;
use crate::handlers::*;

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
    /// This method combines:
    /// 1. Built-in handlers (Core, KV, Cluster, etc.) - always registered
    /// 2. Feature-gated handlers (SQL, DNS, Blob, etc.) - registered based on features and context
    /// 3. Plugin handlers - self-registered via `inventory` and `HandlerFactory`
    ///
    /// All handlers are sorted by priority before being stored.
    #[allow(unused_mut, unused_variables, clippy::vec_init_then_push)]
    pub fn new(ctx: &ClientProtocolContext) -> Self {
        // Collect handlers with their priorities for sorting
        let mut handlers_with_priority: Vec<(Arc<dyn RequestHandler>, u32)> = Vec::new();

        // =====================================================================
        // Built-in handlers (always registered)
        // Priority 100-299 for core functionality
        // =====================================================================
        handlers_with_priority.push((Arc::new(CoreHandler), 100));
        handlers_with_priority.push((Arc::new(KvHandler), 110));
        handlers_with_priority.push((Arc::new(ClusterHandler), 120));
        handlers_with_priority.push((Arc::new(LeaseHandler), 200));
        handlers_with_priority.push((Arc::new(WatchHandler), 210));
        handlers_with_priority.push((Arc::new(CoordinationHandler), 220));
        handlers_with_priority.push((Arc::new(ServiceRegistryHandler), 300));

        // =====================================================================
        // Feature-gated handlers (priority 500-599)
        // These are registered based on compile-time features and runtime context
        // =====================================================================

        // Add SQL handler if feature is enabled
        #[cfg(feature = "sql")]
        handlers_with_priority.push((Arc::new(SqlHandler), 500));

        // Add DNS handler if feature is enabled
        #[cfg(feature = "dns")]
        handlers_with_priority.push((Arc::new(DnsHandler), 510));

        // Add blob handler if blob store is available
        #[cfg(feature = "blob")]
        if ctx.blob_store.is_some() {
            handlers_with_priority.push((Arc::new(BlobHandler), 520));
        }

        // Add docs handler if docs sync is available AND feature enabled
        #[cfg(feature = "docs")]
        if ctx.docs_sync.is_some() || ctx.peer_manager.is_some() {
            handlers_with_priority.push((Arc::new(DocsHandler), 530));
        }

        // Add forge handler if forge node is available
        #[cfg(feature = "forge")]
        if ctx.forge_node.is_some() {
            handlers_with_priority.push((Arc::new(ForgeHandler), 540));
        }

        // Add pijul handler if pijul store is available
        #[cfg(feature = "pijul")]
        if ctx.pijul_store.is_some() {
            handlers_with_priority.push((Arc::new(PijulHandler), 550));
        }

        // Add job handler if job manager is available AND feature enabled
        #[cfg(feature = "jobs")]
        if ctx.job_manager.is_some() {
            handlers_with_priority.push((Arc::new(JobHandler), 560));
        }

        // Add hooks handler if feature is enabled
        // It gracefully handles the case when hook service is unavailable
        #[cfg(feature = "hooks")]
        handlers_with_priority.push((Arc::new(HooksHandler), 570));

        // Add secrets handler if secrets service is available
        #[cfg(feature = "secrets")]
        if ctx.secrets_service.is_some() {
            handlers_with_priority.push((Arc::new(SecretsHandler), 580));
        }

        // Add automerge handler if feature is enabled
        // Note: Always registers when feature is enabled since it uses the KV store
        // which is always available
        #[cfg(feature = "automerge")]
        handlers_with_priority.push((Arc::new(AutomergeHandler), 590));

        // Add CI handler if orchestrator or trigger service is available
        #[cfg(feature = "ci")]
        if ctx.ci_orchestrator.is_some() || ctx.ci_trigger_service.is_some() {
            handlers_with_priority.push((Arc::new(CiHandler), 600));
        }

        // Add cache handler (always available when ci feature is enabled)
        // Cache queries only need KV store which is always present
        #[cfg(feature = "ci")]
        handlers_with_priority.push((Arc::new(CacheHandler), 610));

        // Add cache migration handler (always available when ci feature is enabled)
        #[cfg(feature = "ci")]
        handlers_with_priority.push((Arc::new(CacheMigrationHandler), 620));

        // Add SNIX handler if feature is enabled
        // Handles DirectoryService and PathInfoService operations for ephemeral workers
        #[cfg(feature = "snix")]
        handlers_with_priority.push((Arc::new(SnixHandler), 630));

        // Add worker coordination handler if job manager is available AND feature enabled
        // Handles job polling and completion for ephemeral VM workers
        #[cfg(feature = "jobs")]
        if let Some(job_manager) = &ctx.job_manager {
            handlers_with_priority.push((Arc::new(WorkerHandler::new(job_manager.clone())), 640));
        }

        // =====================================================================
        // Plugin handlers (registered via inventory)
        // These self-register using HandlerFactory and submit_handler_factory!
        // =====================================================================
        let plugin_factories = collect_handler_factories();
        for factory in plugin_factories {
            match factory.create(ctx) {
                Some(handler) => {
                    trace!(
                        factory = factory.name(),
                        priority = factory.priority(),
                        "plugin handler registered via inventory"
                    );
                    handlers_with_priority.push((handler, factory.priority()));
                }
                None => {
                    trace!(factory = factory.name(), "plugin handler factory skipped (preconditions not met)");
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

        // No handler found - this shouldn't happen if all request types are covered
        Err(anyhow::anyhow!("no handler found for request type: {:?}", std::mem::discriminant(&request)))
    }
}

impl std::fmt::Debug for HandlerRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandlerRegistry").field("handler_count", &self.handlers.len()).finish()
    }
}
