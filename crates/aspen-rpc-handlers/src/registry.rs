//! Handler registry for dispatching client RPC requests.
//!
//! This module provides a modular handler architecture that decomposes the monolithic
//! `process_client_request` function into focused, domain-specific handlers.
//!
//! # Plugin Architecture
//!
//! Handlers self-register using the `inventory` crate via `HandlerFactory`.
//! The registry collects all registered factories and creates handler instances
//! based on the runtime context.
//!
//! Core handlers (Core, KV, Cluster, Lease, Watch, Coordination, ServiceRegistry)
//! and some feature handlers (SQL, Docs, Forge) self-register via inventory.
//! Legacy feature-gated handlers without factories are still registered explicitly.

use std::sync::Arc;

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
pub use aspen_rpc_core::HandlerFactory;
pub use aspen_rpc_core::RequestHandler;
// Re-export RequestHandler from aspen-rpc-core for handlers to implement
pub use aspen_rpc_core::collect_handler_factories;
use tracing::debug;
use tracing::trace;

use crate::context::ClientProtocolContext;
#[allow(unused_imports)]
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
    /// 1. Inventory-registered handlers - self-registered via `HandlerFactory`
    /// 2. Legacy feature-gated handlers - registered based on features and context
    ///
    /// All handlers are sorted by priority before being stored.
    #[allow(unused_mut, unused_variables, clippy::vec_init_then_push)]
    pub fn new(ctx: &ClientProtocolContext) -> Self {
        // Collect handlers with their priorities for sorting
        let mut handlers_with_priority: Vec<(Arc<dyn RequestHandler>, u32)> = Vec::new();

        // =====================================================================
        // Inventory-registered handlers (self-register via submit_handler_factory!)
        //
        // The following handlers self-register and are collected via inventory:
        // - CoreHandler (100)
        // - KvHandler (110)
        // - ClusterHandler (120)
        // - LeaseHandler (200)
        // - WatchHandler (210)
        // - CoordinationHandler (220)
        // - ServiceRegistryHandler (300)
        // - SqlHandler (500) - when sql feature enabled
        // - DocsHandler (530) - when docs feature enabled and sync available
        // - ForgeHandler (540) - when forge feature enabled and node available
        // =====================================================================
        let plugin_factories = collect_handler_factories();
        for factory in plugin_factories {
            match factory.create(ctx) {
                Some(handler) => {
                    trace!(factory = factory.name(), priority = factory.priority(), "handler registered via inventory");
                    handlers_with_priority.push((handler, factory.priority()));
                }
                None => {
                    trace!(factory = factory.name(), "handler factory skipped (preconditions not met)");
                }
            }
        }

        // =====================================================================
        // Legacy feature-gated handlers (priority 500-699)
        // These handlers don't yet have HandlerFactory implementations.
        // TODO: Convert these to inventory-based registration.
        // =====================================================================

        // Add DNS handler if feature is enabled
        #[cfg(feature = "dns")]
        handlers_with_priority.push((Arc::new(DnsHandler), 510));

        // Add blob handler if blob store is available
        #[cfg(feature = "blob")]
        if ctx.blob_store.is_some() {
            handlers_with_priority.push((Arc::new(BlobHandler), 520));
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
