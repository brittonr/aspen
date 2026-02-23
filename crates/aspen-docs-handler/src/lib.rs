//! Docs/Sync service for Aspen.
//!
//! Provides the `DocsServiceExecutor` for typed RPC dispatch and the
//! `DocsHandlerFactory` for registering the handler with the RPC framework.
//!
//! Handles docs operations via ClientRpcRequest:
//! - set/get/delete/list: Document CRUD
//! - status: Sync namespace status
//! - get_key_origin: Origin cluster for a key
//! - add_peer/remove_peer/list_peers: Peer federation
//! - get_peer_status/update_filter/update_priority/set_enabled: Peer config

pub mod executor;

use std::sync::Arc;

use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::HandlerFactory;
use aspen_rpc_core::RequestHandler;
use aspen_rpc_core::ServiceHandler;
use aspen_rpc_core::submit_handler_factory;
pub use executor::DocsServiceExecutor;

/// Handler factory for docs service.
///
/// Registers the docs handler with the RPC framework via inventory.
pub struct DocsHandlerFactory;

impl DocsHandlerFactory {
    /// Create a new docs handler factory.
    pub const fn new() -> Self {
        Self
    }
}

impl HandlerFactory for DocsHandlerFactory {
    fn create(&self, ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
        let docs_sync = ctx.docs_sync.as_ref()?.clone();
        let peer_manager = ctx.peer_manager.clone();
        let executor = Arc::new(DocsServiceExecutor::new(docs_sync, peer_manager));
        Some(Arc::new(ServiceHandler::new(executor)))
    }

    fn name(&self) -> &'static str {
        "DocsHandler"
    }

    fn priority(&self) -> u32 {
        530
    }

    fn app_id(&self) -> Option<&'static str> {
        Some("docs")
    }
}

submit_handler_factory!(DocsHandlerFactory);
