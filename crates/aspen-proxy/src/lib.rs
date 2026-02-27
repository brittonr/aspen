//! HTTP proxy bridging for Aspen over iroh QUIC connections.
//!
//! This crate provides HTTP proxying primitives that bridge TCP/HTTP traffic
//! over Aspen's iroh peer-to-peer QUIC network. Built on [`iroh_proxy_utils`],
//! it integrates Aspen's authentication layer with iroh's proxy architecture.
//!
//! # Architecture
//!
//! Two proxy components work in tandem:
//!
//! - **Upstream proxy** ([`AspenUpstreamProxy`]): Registers as an iroh [`ProtocolHandler`] on the
//!   Aspen node's router. Accepts proxied streams from remote peers, authorizes them against
//!   Aspen's auth system, and forwards to local TCP origin servers.
//!
//! - **Downstream proxy** ([`DownstreamProxy`]): Accepts local TCP connections and tunnels them
//!   over iroh to a remote upstream proxy. Supports forward proxy (client specifies destination)
//!   and reverse proxy (fixed upstream) modes.
//!
//! # Protocol
//!
//! Uses HTTP/1.1 over iroh QUIC bidirectional streams with ALPN `iroh-http-proxy/1`.
//! Supports both CONNECT tunneling (for opaque TCP tunnels) and absolute-form HTTP
//! requests (for HTTP forward proxying).
//!
//! # Usage
//!
//! ## Upstream (server-side)
//!
//! ```ignore
//! use aspen_proxy::AspenUpstreamProxy;
//!
//! // Create upstream proxy with Aspen auth
//! let proxy = AspenUpstreamProxy::new(cluster_cookie.clone());
//!
//! // Register on the iroh router
//! router_builder.http_proxy(proxy.into_handler()?);
//! ```
//!
//! ## Downstream (client-side)
//!
//! ```ignore
//! use aspen_proxy::DownstreamProxy;
//! use iroh_proxy_utils::downstream::PoolOpts;
//!
//! let proxy = DownstreamProxy::new(endpoint, PoolOpts::default());
//! proxy.forward_tcp_listener(listener, mode).await?;
//! ```
//!
//! [`ProtocolHandler`]: iroh::protocol::ProtocolHandler

pub mod auth;

// Re-export the proxy ALPN for router registration.
pub use iroh_proxy_utils::ALPN as HTTP_PROXY_ALPN;
// Re-export downstream proxy (used as-is, no Aspen wrapper needed).
pub use iroh_proxy_utils::Authority;
pub use iroh_proxy_utils::downstream::DownstreamProxy;
pub use iroh_proxy_utils::downstream::EndpointAuthority;
pub use iroh_proxy_utils::downstream::HttpProxyOpts;
pub use iroh_proxy_utils::downstream::PoolOpts;
pub use iroh_proxy_utils::downstream::ProxyMode;
pub use iroh_proxy_utils::downstream::StaticForwardProxy;
pub use iroh_proxy_utils::downstream::StaticReverseProxy;
pub use iroh_proxy_utils::downstream::TunnelClientStreams;
// Re-export upstream types for custom auth implementations.
pub use iroh_proxy_utils::upstream::{AcceptAll, AuthHandler, DenyAll, UpstreamProxy};

use crate::auth::AspenAuthHandler;
pub use crate::auth::TrustedPeers;

/// Upstream proxy with Aspen cluster authentication.
///
/// Wraps [`UpstreamProxy`] with an [`AspenAuthHandler`] that validates incoming
/// proxy requests against the cluster's shared cookie and optional trusted-peers
/// allowlist.
///
/// # Example
///
/// ```ignore
/// // Cookie-only (legacy)
/// let proxy = AspenUpstreamProxy::new("my-cluster-cookie".to_string());
///
/// // Cookie + trusted-peers allowlist (production)
/// let trusted = TrustedPeers::default();
/// let proxy = AspenUpstreamProxy::with_trusted_peers("cookie".into(), trusted);
///
/// let handler = proxy.into_handler()?;
/// // Register with router_builder.http_proxy(handler)
/// ```
pub struct AspenUpstreamProxy {
    auth: AspenAuthHandler,
}

impl AspenUpstreamProxy {
    /// Create a new upstream proxy with cookie-only authentication.
    ///
    /// Without a trusted-peers allowlist, any iroh-authenticated peer that
    /// doesn't send a wrong cookie is accepted. Use [`Self::with_trusted_peers`]
    /// for production deployments.
    pub fn new(cluster_cookie: String) -> Self {
        Self {
            auth: AspenAuthHandler::new(cluster_cookie),
        }
    }

    /// Create a new upstream proxy with cookie + trusted-peers allowlist.
    ///
    /// Peers without a cookie header are checked against the allowlist.
    /// The allowlist can be updated dynamically via its `Arc<RwLock<_>>` handle
    /// (e.g., from a cluster membership watcher).
    pub fn with_trusted_peers(cluster_cookie: String, trusted_peers: TrustedPeers) -> Self {
        Self {
            auth: AspenAuthHandler::with_trusted_peers(cluster_cookie, trusted_peers),
        }
    }

    /// Build the [`UpstreamProxy`] protocol handler.
    ///
    /// Returns an error if the HTTP client cannot be created.
    pub fn into_handler(self) -> Result<UpstreamProxy, Box<dyn std::error::Error + Send + Sync>> {
        UpstreamProxy::new(self.auth).map_err(|e| Box::new(e) as _)
    }
}
