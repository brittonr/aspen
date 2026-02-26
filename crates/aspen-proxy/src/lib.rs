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

/// Upstream proxy with Aspen cluster authentication.
///
/// Wraps [`UpstreamProxy`] with an [`AspenAuthHandler`] that validates incoming
/// proxy requests against the cluster's shared cookie. Only peers that present
/// the correct cookie (via the cluster membership) are authorized.
///
/// # Example
///
/// ```ignore
/// let proxy = AspenUpstreamProxy::new("my-cluster-cookie".to_string());
/// let handler = proxy.into_handler()?;
/// // Register with router_builder.http_proxy(handler)
/// ```
pub struct AspenUpstreamProxy {
    cluster_cookie: String,
}

impl AspenUpstreamProxy {
    /// Create a new upstream proxy that authenticates against the given cluster cookie.
    pub fn new(cluster_cookie: String) -> Self {
        Self { cluster_cookie }
    }

    /// Build the [`UpstreamProxy`] protocol handler.
    ///
    /// Returns an error if the HTTP client cannot be created.
    pub fn into_handler(self) -> Result<UpstreamProxy, Box<dyn std::error::Error + Send + Sync>> {
        let auth = AspenAuthHandler::new(self.cluster_cookie);
        UpstreamProxy::new(auth).map_err(|e| Box::new(e) as _)
    }
}
