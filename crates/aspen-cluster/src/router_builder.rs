//! Fluent builder for configuring Iroh Router protocol handlers.

use std::sync::Arc;

use iroh_gossip::net::GOSSIP_ALPN;
use iroh_gossip::net::Gossip;

/// Fluent builder for configuring Iroh Router protocol handlers.
///
/// This builder eliminates duplication across `spawn_router`, `spawn_router_extended`,
/// and `spawn_router_full` by providing a unified, fluent API for registering
/// protocol handlers.
///
/// # Example
///
/// ```ignore
/// manager.spawn_router_with(|b| b
///     .raft(raft_handler)
///     .auth_raft(auth_handler)
///     .client(client_handler));
/// ```
///
/// Gossip is automatically registered if enabled on the endpoint manager.
pub struct RouterBuilder {
    builder: iroh::protocol::RouterBuilder,
    gossip: Option<Arc<Gossip>>,
}

impl RouterBuilder {
    /// Create a new router builder.
    pub(crate) fn new(builder: iroh::protocol::RouterBuilder, gossip: Option<Arc<Gossip>>) -> Self {
        Self { builder, gossip }
    }

    /// Register the legacy unauthenticated Raft RPC protocol handler.
    ///
    /// ALPN: `raft-rpc`
    ///
    /// # Security Warning
    ///
    /// This method registers an **unauthenticated** Raft handler. Any node that knows
    /// the endpoint address can connect. For production deployments, use `auth_raft()`
    /// which uses the `raft-auth` ALPN with HMAC-SHA256 authentication.
    ///
    /// # Deprecation
    ///
    /// This method is deprecated. Use `auth_raft()` for new deployments.
    #[deprecated(
        since = "0.2.0",
        note = "Use auth_raft() for production deployments. raft() provides no authentication."
    )]
    #[allow(deprecated)]
    pub fn raft<R: iroh::protocol::ProtocolHandler>(mut self, handler: R) -> Self {
        use aspen_transport::RAFT_ALPN;
        self.builder = self.builder.accept(RAFT_ALPN, handler);
        tracing::warn!(
            "registered LEGACY unauthenticated Raft RPC handler (ALPN: raft-rpc) - use auth_raft() for production"
        );
        self
    }

    /// Register the authenticated Raft RPC protocol handler (optional).
    ///
    /// ALPN: `raft-auth`
    pub fn auth_raft<A: iroh::protocol::ProtocolHandler>(mut self, handler: A) -> Self {
        use aspen_transport::RAFT_AUTH_ALPN;
        self.builder = self.builder.accept(RAFT_AUTH_ALPN, handler);
        tracing::info!("registered authenticated Raft RPC protocol handler (ALPN: raft-auth)");
        self
    }

    /// Register the log subscriber protocol handler (optional).
    ///
    /// ALPN: `aspen-logs`
    pub fn log_subscriber<L: iroh::protocol::ProtocolHandler>(mut self, handler: L) -> Self {
        use aspen_transport::LOG_SUBSCRIBER_ALPN;
        self.builder = self.builder.accept(LOG_SUBSCRIBER_ALPN, handler);
        tracing::info!("registered log subscriber protocol handler (ALPN: aspen-logs)");
        self
    }

    /// Register the client RPC protocol handler (optional).
    ///
    /// ALPN: `aspen-tui`
    pub fn client<C: iroh::protocol::ProtocolHandler>(mut self, handler: C) -> Self {
        use aspen_transport::CLIENT_ALPN;
        self.builder = self.builder.accept(CLIENT_ALPN, handler);
        tracing::info!("registered Client RPC protocol handler (ALPN: aspen-tui)");
        self
    }

    /// Register the blobs protocol handler (optional).
    ///
    /// ALPN: `iroh-blobs/0`
    #[cfg(feature = "blob")]
    pub fn blobs<B: iroh::protocol::ProtocolHandler>(mut self, handler: B) -> Self {
        self.builder = self.builder.accept(iroh_blobs::ALPN, handler);
        tracing::info!("registered Blobs protocol handler (ALPN: iroh-blobs/0)");
        self
    }

    /// Register the HTTP proxy protocol handler (optional).
    ///
    /// Enables TCP/HTTP proxying over iroh QUIC connections.
    /// Uses `iroh_proxy_utils` for the proxy implementation.
    ///
    /// ALPN: `iroh-http-proxy/1`
    #[cfg(feature = "proxy")]
    pub fn http_proxy<P: iroh::protocol::ProtocolHandler>(mut self, handler: P) -> Self {
        self.builder = self.builder.accept(aspen_proxy::HTTP_PROXY_ALPN, handler);
        tracing::info!("registered HTTP proxy protocol handler (ALPN: iroh-http-proxy/1)");
        self
    }

    /// Register the Nix cache HTTP/3 gateway protocol handler (optional).
    ///
    /// This serves a Nix binary cache over HTTP/3 using h3-iroh.
    ///
    /// ALPN: `iroh+h3`
    pub fn nix_cache<N: iroh::protocol::ProtocolHandler>(mut self, handler: N) -> Self {
        use aspen_transport::NIX_CACHE_H3_ALPN;
        self.builder = self.builder.accept(NIX_CACHE_H3_ALPN, handler);
        tracing::info!("registered Nix cache HTTP/3 gateway (ALPN: iroh+h3)");
        self
    }

    /// Finalize the router configuration and spawn it.
    ///
    /// Automatically registers gossip if enabled on the endpoint.
    pub(crate) fn spawn_internal(mut self) -> iroh::protocol::Router {
        // Auto-register gossip if enabled
        if let Some(gossip) = self.gossip {
            self.builder = self.builder.accept(GOSSIP_ALPN, gossip);
            tracing::info!("registered Gossip protocol handler (ALPN: gossip)");
        }
        self.builder.spawn()
    }
}
