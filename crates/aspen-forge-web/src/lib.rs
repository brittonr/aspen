//! Forge web frontend served over HTTP/3 via iroh-h3.
//! No axum — raw h3 request handling.

pub mod routes;
pub mod server;
pub mod state;
pub mod templates;

/// Re-export the h3 compatibility proxy for embedded TCP bridging.
pub use aspen_h3_proxy::H3Proxy;
/// Re-export the h3 compatibility proxy for embedded TCP bridging.
pub use aspen_h3_proxy::ProxyConfig;
