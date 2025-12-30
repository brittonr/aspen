//! Domain-specific request handlers for Client RPC.
//!
//! This module provides a modular handler architecture that decomposes the monolithic
//! `process_client_request` function into focused, domain-specific handlers.
//!
//! # Architecture
//!
//! ```text
//! ClientProtocolHandler
//!        │
//!        ▼
//!   handle_client_request()
//!        │
//!        ▼
//!   HandlerRegistry::dispatch()
//!        │
//!        ├──► KvHandler (ReadKey, WriteKey, DeleteKey, ScanKeys, BatchRead, BatchWrite)
//!        ├──► CoordinationHandler (Lock, Counter, Sequence, RateLimiter, Barrier, Semaphore, RWLock, Queue)
//!        ├──► BlobHandler (AddBlob, GetBlob, DownloadBlob, etc.)
//!        ├──► DnsHandler (DnsSetRecord, DnsGetRecord, etc.)
//!        ├──► DocsHandler (DocsSet, DocsGet, PeerCluster ops)
//!        ├──► ForgeHandler (all Forge* operations)
//!        ├──► PijulHandler (all Pijul* operations)
//!        ├──► ClusterHandler (Init, AddLearner, Membership, etc.)
//!        └──► CoreHandler (Ping, Health, Metrics, NodeInfo)
//! ```
//!
//! # Tiger Style
//!
//! - Each handler has a single responsibility
//! - Handlers are stateless; all state comes from `ClientProtocolContext`
//! - Bounded request processing with explicit error handling
//! - Sanitized error messages for security

use std::sync::Arc;

use super::client::ClientProtocolContext;
use crate::client_rpc::ClientRpcRequest;
use crate::client_rpc::ClientRpcResponse;

// Handler modules
pub mod blob;
pub mod cluster;
pub mod coordination;
pub mod core;
pub mod dns;
pub mod docs;
#[cfg(feature = "forge")]
pub mod forge;
pub mod kv;
pub mod lease;
#[cfg(feature = "pijul")]
pub mod pijul;
pub mod service_registry;
pub mod sql;
pub mod watch;

// Re-export handlers
pub use blob::BlobHandler;
pub use cluster::ClusterHandler;
pub use coordination::CoordinationHandler;
pub use core::CoreHandler;
pub use dns::DnsHandler;
pub use docs::DocsHandler;
#[cfg(feature = "forge")]
pub use forge::ForgeHandler;
pub use kv::KvHandler;
pub use lease::LeaseHandler;
#[cfg(feature = "pijul")]
pub use pijul::PijulHandler;
pub use service_registry::ServiceRegistryHandler;
pub use sql::SqlHandler;
pub use watch::WatchHandler;

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
#[async_trait::async_trait]
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
            Arc::new(SqlHandler),
            Arc::new(CoordinationHandler),
            Arc::new(ServiceRegistryHandler),
            Arc::new(DnsHandler),
        ];

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
                tracing::debug!(
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
