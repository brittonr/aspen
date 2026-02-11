//! Handler registry for dispatching client RPC requests.
//!
//! This module provides a modular handler architecture that decomposes the monolithic
//! `process_client_request` function into focused, domain-specific handlers.

use std::sync::Arc;

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
// Re-export RequestHandler from aspen-rpc-core for handlers to implement
pub use aspen_rpc_core::RequestHandler;
use tracing::debug;

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
    pub fn new(ctx: &ClientProtocolContext) -> Self {
        let mut handlers: Vec<Arc<dyn RequestHandler>> = vec![
            Arc::new(CoreHandler),
            Arc::new(KvHandler),
            Arc::new(ClusterHandler),
            Arc::new(LeaseHandler),
            Arc::new(WatchHandler),
            Arc::new(CoordinationHandler),
            Arc::new(ServiceRegistryHandler),
        ];

        // Add SQL handler if feature is enabled
        #[cfg(feature = "sql")]
        handlers.push(Arc::new(SqlHandler));

        // Add DNS handler if feature is enabled
        #[cfg(feature = "dns")]
        handlers.push(Arc::new(DnsHandler));

        // Add blob handler if blob store is available
        #[cfg(feature = "blob")]
        if ctx.blob_store.is_some() {
            handlers.push(Arc::new(BlobHandler));
        }

        // Add docs handler if docs sync is available
        if ctx.docs_sync.is_some() || ctx.peer_manager.is_some() {
            handlers.push(Arc::new(DocsHandler));
        }

        // Add forge handler if forge node is available
        #[cfg(feature = "forge")]
        if ctx.forge_node.is_some() {
            handlers.push(Arc::new(ForgeHandler));
        }

        // Add pijul handler if pijul store is available
        #[cfg(feature = "pijul")]
        if ctx.pijul_store.is_some() {
            handlers.push(Arc::new(PijulHandler));
        }

        // Add job handler if job manager is available
        if ctx.job_manager.is_some() {
            handlers.push(Arc::new(JobHandler));
        }

        // Add hooks handler if feature is enabled
        // It gracefully handles the case when hook service is unavailable
        #[cfg(feature = "hooks")]
        handlers.push(Arc::new(HooksHandler));

        // Add secrets handler if secrets service is available
        #[cfg(feature = "secrets")]
        if ctx.secrets_service.is_some() {
            handlers.push(Arc::new(SecretsHandler));
        }

        // Add automerge handler if feature is enabled
        // Note: Always registers when feature is enabled since it uses the KV store
        // which is always available
        #[cfg(feature = "automerge")]
        handlers.push(Arc::new(AutomergeHandler));

        // Add CI handler if orchestrator or trigger service is available
        #[cfg(feature = "ci")]
        if ctx.ci_orchestrator.is_some() || ctx.ci_trigger_service.is_some() {
            handlers.push(Arc::new(CiHandler));
        }

        // Add cache handler (always available when ci feature is enabled)
        // Cache queries only need KV store which is always present
        #[cfg(feature = "ci")]
        handlers.push(Arc::new(CacheHandler));

        // Add cache migration handler (always available when ci feature is enabled)
        #[cfg(feature = "ci")]
        handlers.push(Arc::new(CacheMigrationHandler));

        // Add SNIX handler if feature is enabled
        // Handles DirectoryService and PathInfoService operations for ephemeral workers
        #[cfg(feature = "snix")]
        handlers.push(Arc::new(SnixHandler));

        // Add worker coordination handler if job manager is available
        // Handles job polling and completion for ephemeral VM workers
        if let Some(job_manager) = &ctx.job_manager {
            handlers.push(Arc::new(WorkerHandler::new(job_manager.clone())));
        }

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
