//! Tiger Style resource bounds for the service mesh.
//!
//! All constants are fixed and immutable. Each has explicit bounds
//! to prevent unbounded resource allocation.

/// Maximum number of services registered in a cluster.
pub const MAX_NET_SERVICES: u32 = 10_000;

/// Maximum number of tags per service entry.
pub const MAX_NET_TAGS_PER_SERVICE: u32 = 32;

/// Maximum length of a service name (matches DNS label limit).
pub const MAX_NET_SERVICE_NAME_LEN: u32 = 253;

/// Maximum number of loopback addresses for DNS A records.
pub const MAX_NET_DNS_LOOPBACK_ADDRS: u32 = 254;

/// Maximum concurrent SOCKS5 connections.
pub const MAX_SOCKS5_CONNECTIONS: u32 = 1_000;

/// Timeout for SOCKS5 handshake in seconds.
pub const SOCKS5_HANDSHAKE_TIMEOUT_SECS: u64 = 10;

/// Idle timeout for SOCKS5 tunneled connections in seconds.
pub const SOCKS5_IDLE_TIMEOUT_SECS: u64 = 300;

/// TTL for DNS records created by the service mesh.
pub const NET_DNS_TTL_SECS: u32 = 5;

/// Interval for polling the service registry for changes.
pub const NET_REGISTRY_POLL_INTERVAL_SECS: u64 = 10;

/// Interval for refreshing the token revocation list.
pub const NET_REVOCATION_POLL_INTERVAL_SECS: u64 = 60;

/// Timeout for graceful shutdown of the daemon.
pub const NET_SHUTDOWN_TIMEOUT_SECS: u64 = 30;

/// KV prefix for service entries.
pub const NET_SVC_PREFIX: &str = "/_sys/net/svc/";

/// KV prefix for node entries.
pub const NET_NODE_PREFIX: &str = "/_sys/net/node/";

// Compile-time assertions for all constants.
const _: () = {
    assert!(MAX_NET_SERVICES > 0);
    assert!(MAX_NET_SERVICES <= 100_000);
    assert!(MAX_NET_TAGS_PER_SERVICE > 0);
    assert!(MAX_NET_TAGS_PER_SERVICE <= 256);
    assert!(MAX_NET_SERVICE_NAME_LEN > 0);
    assert!(MAX_NET_SERVICE_NAME_LEN <= 253);
    assert!(MAX_NET_DNS_LOOPBACK_ADDRS > 0);
    assert!(MAX_NET_DNS_LOOPBACK_ADDRS <= 254);
    assert!(MAX_SOCKS5_CONNECTIONS > 0);
    assert!(MAX_SOCKS5_CONNECTIONS <= 100_000);
    assert!(SOCKS5_HANDSHAKE_TIMEOUT_SECS > 0);
    assert!(SOCKS5_HANDSHAKE_TIMEOUT_SECS <= 60);
    assert!(SOCKS5_IDLE_TIMEOUT_SECS > 0);
    assert!(NET_DNS_TTL_SECS > 0);
    assert!(NET_REGISTRY_POLL_INTERVAL_SECS > 0);
    assert!(NET_REVOCATION_POLL_INTERVAL_SECS > 0);
    assert!(NET_SHUTDOWN_TIMEOUT_SECS > 0);
};
