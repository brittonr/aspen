//! Configuration types for the DNS server.

use std::net::SocketAddr;

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// DNS protocol server configuration.
///
/// Controls the embedded DNS server that serves records from the Aspen DNS layer.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DnsServerConfig {
    /// Enable the DNS protocol server.
    ///
    /// When enabled, the node will start a DNS server that responds to
    /// queries for records stored in the Aspen DNS layer.
    ///
    /// Default: false
    #[serde(default)]
    pub enabled: bool,

    /// Bind address for DNS server (UDP and TCP).
    ///
    /// Use port 5353 for user-space operation (no root required).
    /// Use port 53 for standard DNS (requires root or CAP_NET_BIND_SERVICE).
    ///
    /// Default: "127.0.0.1:5353"
    #[serde(default = "default_dns_bind_addr")]
    pub bind_addr: SocketAddr,

    /// Zones to serve from Aspen DNS layer.
    ///
    /// Queries for these zones are resolved from local cache.
    /// Other queries are forwarded to upstream if forwarding_enabled is true.
    ///
    /// Tiger Style: Max 64 zones.
    ///
    /// Default: ["aspen.local"]
    #[serde(default = "default_dns_zones")]
    pub zones: Vec<String>,

    /// Upstream DNS servers for forwarding unknown queries.
    ///
    /// When forwarding_enabled is true, queries for domains not in
    /// configured zones are forwarded to these servers.
    ///
    /// Tiger Style: Max 8 upstream servers.
    ///
    /// Default: ["8.8.8.8:53", "8.8.4.4:53"]
    #[serde(default = "default_dns_upstreams")]
    pub upstreams: Vec<SocketAddr>,

    /// Forward queries for unknown domains to upstream.
    ///
    /// When true, queries for domains not in configured zones are
    /// forwarded to upstream DNS servers.
    /// When false, NXDOMAIN is returned for unknown domains.
    ///
    /// Default: true
    #[serde(default = "default_dns_forwarding")]
    pub forwarding_enabled: bool,
}

impl Default for DnsServerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bind_addr: default_dns_bind_addr(),
            zones: default_dns_zones(),
            upstreams: default_dns_upstreams(),
            forwarding_enabled: default_dns_forwarding(),
        }
    }
}

fn default_dns_bind_addr() -> SocketAddr {
    SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)), 5353)
}

fn default_dns_zones() -> Vec<String> {
    vec!["aspen.local".to_string()]
}

fn default_dns_upstreams() -> Vec<SocketAddr> {
    vec![
        SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::new(8, 8, 8, 8)), 53),
        SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::new(8, 8, 4, 4)), 53),
    ]
}

fn default_dns_forwarding() -> bool {
    true
}
