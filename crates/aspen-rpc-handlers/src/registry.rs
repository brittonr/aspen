//! Handler registry for dispatching client RPC requests.
//!
//! This module provides a modular handler architecture that decomposes the monolithic
//! `process_client_request` function into focused, domain-specific handlers.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::debug;

use crate::context::ClientProtocolContext;
use crate::handlers::*;
use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;

/// Trait for domain-specific request handlers.
///
/// Each handler is responsible for a subset of `ClientRpcRequest` variants.
/// The `can_handle` method determines if a handler can process a given request,
/// and `handle` processes the request and returns a response.
///
/// # Tiger Style
///
/// - Handlers must not hold state across requests
/// - All state comes from `ClientProtocolContext`
/// - Errors are sanitized before returning to client
#[async_trait]
pub trait RequestHandler: Send + Sync {
    /// Returns true if this handler can process the given request.
    fn can_handle(&self, request: &ClientRpcRequest) -> bool;

    /// Process the request and return a response.
    ///
    /// # Errors
    ///
    /// Returns an error if the request cannot be processed. The error will be
    /// sanitized before being sent to the client.
    async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse>;

    /// Returns the handler name for logging/debugging.
    fn name(&self) -> &'static str;
}

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

        Self { handlers: Arc::new(handlers) }
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
        Err(anyhow::anyhow!(
            "no handler found for request type: {:?}",
            std::mem::discriminant(&request)
        ))
    }
}

impl std::fmt::Debug for HandlerRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandlerRegistry")
            .field("handler_count", &self.handlers.len())
            .finish()
    }
}