//! Proxy configuration.

use std::time::Duration;

use iroh::PublicKey;

/// Configuration for the TCP-to-h3 proxy.
pub struct ProxyConfig {
    /// TCP bind address (default: "127.0.0.1").
    pub bind_addr: String,

    /// TCP listen port (default: 8080).
    pub port: u16,

    /// Iroh endpoint ID (public key) of the target server.
    pub endpoint_id: PublicKey,

    /// ALPN protocol to use when connecting (e.g., b"aspen/forge-web/1").
    pub alpn: Vec<u8>,

    /// Per-request timeout (default: 30s).
    pub request_timeout: Duration,
}

impl ProxyConfig {
    /// Create a minimal config with just the target and ALPN.
    pub fn new(endpoint_id: PublicKey, alpn: Vec<u8>) -> Self {
        Self {
            bind_addr: "127.0.0.1".into(),
            port: 8080,
            endpoint_id,
            alpn,
            request_timeout: Duration::from_secs(30),
        }
    }
}
