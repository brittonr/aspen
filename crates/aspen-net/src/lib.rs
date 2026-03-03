//! Service mesh control plane for Aspen.
//!
//! Provides named service discovery, SOCKS5 proxying, port forwarding,
//! and DNS integration over iroh's P2P QUIC network. Built as a thin
//! control plane on top of `iroh-proxy-utils`' existing data plane.
//!
//! # Architecture
//!
//! - **Registry**: Service CRUD in Raft KV (`/_sys/net/svc/` prefix)
//! - **Resolver**: Name → (endpoint_id, port) with local cache
//! - **SOCKS5**: RFC 1928 proxy resolving `*.aspen` names
//! - **Forward**: TCP port forwarding via `DownstreamProxy`
//! - **Auth**: UCAN capability token verification wrapper
//! - **Daemon**: Orchestrates all components

pub mod auth;
pub mod client_kv;
pub mod constants;
pub mod daemon;
pub mod dns;
pub mod forward;
pub mod handler;
pub mod registry;
pub mod resolver;
pub mod socks5;
pub mod tunnel;
pub mod types;
pub mod verified;

// =============================================================================
// Handler Factory Registration
// =============================================================================

use std::sync::Arc;

use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::HandlerFactory;
use aspen_rpc_core::RequestHandler;

/// Factory for creating `NetHandler` instances.
///
/// Priority 250 (service handler range: 200-499).
/// Creates a `ServiceRegistry` wrapping the node's KV store and passes it
/// to `NetHandler` for dispatching net publish/lookup/list/unpublish RPCs.
pub struct NetHandlerFactory;

impl Default for NetHandlerFactory {
    fn default() -> Self {
        Self
    }
}

impl NetHandlerFactory {
    /// Create a new factory instance.
    pub const fn new() -> Self {
        Self
    }
}

impl HandlerFactory for NetHandlerFactory {
    fn create(&self, ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
        let registry = Arc::new(registry::ServiceRegistry::new(ctx.kv_store.clone()));
        Some(Arc::new(handler::NetHandler::new(registry)))
    }

    fn name(&self) -> &'static str {
        "NetHandler"
    }

    fn priority(&self) -> u32 {
        250
    }
}

aspen_rpc_core::submit_handler_factory!(NetHandlerFactory);
