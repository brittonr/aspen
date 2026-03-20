//! Proxy configuration.

use std::time::Duration;

use iroh::EndpointAddr;

/// Configuration for the TCP-to-h3 proxy.
pub struct ProxyConfig {
    /// TCP bind address (default: "127.0.0.1").
    pub bind_addr: String,

    /// TCP listen port (default: 8080).
    pub port: u16,

    /// Full iroh endpoint address of the target server (includes direct socket addrs).
    pub target_addr: EndpointAddr,

    /// ALPN protocol to use when connecting (e.g., b"aspen/forge-web/1").
    pub alpn: Vec<u8>,

    /// Per-request timeout (default: 30s).
    pub request_timeout: Duration,
}

impl ProxyConfig {
    /// Create a minimal config with just the target and ALPN.
    pub fn new(target_addr: EndpointAddr, alpn: Vec<u8>) -> Self {
        Self {
            bind_addr: "127.0.0.1".into(),
            port: 8080,
            target_addr,
            alpn,
            request_timeout: Duration::from_secs(30),
        }
    }
}
