//! Docs/Sync service for Aspen.
//!
//! Provides the `DocsServiceExecutor` for typed RPC dispatch and the
//! `DocsHandlerFactory` for explicit registry wiring in `aspen-rpc-handlers`.
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
pub use executor::DocsServiceExecutor;

/// Handler factory for docs service.
///
/// Registered explicitly by `aspen-rpc-handlers::registry`.
pub struct DocsHandlerFactory;

impl Default for DocsHandlerFactory {
    fn default() -> Self {
        Self
    }
}

impl DocsHandlerFactory {
    /// Create a new docs handler factory.
    pub const fn new() -> Self {
        Self
    }
}

impl HandlerFactory for DocsHandlerFactory {
    fn create(&self, ctx: &ClientProtocolContext) -> anyhow::Result<Arc<dyn RequestHandler>> {
        let caps = ctx.docs_handler_context()?;
        let executor = Arc::new(DocsServiceExecutor::new(caps.docs_sync, caps.peer_manager));
        Ok(Arc::new(ServiceHandler::new(executor)))
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
