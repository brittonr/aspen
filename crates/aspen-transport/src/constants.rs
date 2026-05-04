//! Protocol handler constants.
//!
//! ALPN identifiers and resource limits for protocol handlers.

/// Transport-owned ALPN protocol registration metadata.
///
/// This is the source of truth for Aspen protocol identifiers owned by
/// `aspen-transport`. Application crates may own additional ALPNs next to their
/// handlers, but transport-owned identifiers must be listed here so uniqueness,
/// formatting, and documentation stay testable.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AlpnProtocolContract {
    /// Symbolic constant name for the protocol ALPN.
    pub name: &'static str,
    /// ALPN bytes offered to Iroh/QUIC.
    pub alpn: &'static [u8],
    /// Owning protocol namespace.
    pub namespace: &'static str,
}

/// ALPN protocol identifier for Raft RPC (legacy, no authentication).
///
/// # Security Warning
///
/// This ALPN provides **no authentication**. Any node that knows the endpoint
/// address can connect and participate in Raft consensus. This allows:
/// - Unauthorized nodes to join the cluster
/// - Potential data exfiltration via Raft log replication
/// - Cluster disruption by malicious nodes
///
/// **Use `RAFT_AUTH_ALPN` for production deployments.**
///
/// This constant is retained for backward compatibility during migration.
/// New deployments should always use authenticated Raft (`raft-auth` ALPN).
#[deprecated(
    since = "0.2.0",
    note = "Use RAFT_AUTH_ALPN for production deployments. RAFT_ALPN provides no authentication."
)]
pub const RAFT_ALPN: &[u8] = b"raft-rpc";

/// ALPN protocol identifier for authenticated Raft RPC.
///
/// Uses HMAC-SHA256 challenge-response authentication based on the cluster cookie.
/// This is the recommended ALPN for production deployments.
pub const RAFT_AUTH_ALPN: &[u8] = b"raft-auth";

/// ALPN protocol identifier for sharded Raft RPC.
///
/// Handles RPC for multiple Raft shards over a single connection. Each message
/// is prefixed with a 4-byte big-endian shard ID that routes to the appropriate
/// Raft core. This enables horizontal scaling to hundreds of shards without
/// requiring per-shard ALPN registration.
pub const RAFT_SHARDED_ALPN: &[u8] = b"raft-shard";

/// ALPN protocol identifier for Client RPC.
/// Canonical source: aspen-constants::network::CLIENT_ALPN
pub use aspen_constants::network::CLIENT_ALPN;

/// ALPN identifier for log subscription protocol.
pub const LOG_SUBSCRIBER_ALPN: &[u8] = b"aspen-logs";

/// ALPN protocol identifier for net service mesh tunnels.
///
/// Used by the SOCKS5 proxy (client side) and TunnelAcceptor (server side)
/// to establish bidirectional TCP tunnels through iroh QUIC.
pub const NET_TUNNEL_ALPN: &[u8] = b"/aspen/net-tunnel/0";

/// ALPN protocol identifier for Nix binary cache HTTP/3 gateway.
///
/// Uses the standard iroh-h3 ALPN to enable HTTP/3 semantics over Iroh QUIC.
/// This allows standard HTTP/3 clients to connect while satisfying Aspen's
/// "Iroh-only networking" architecture constraint.
pub const NIX_CACHE_H3_ALPN: &[u8] = b"iroh+h3";

/// ALPN protocol identifier for Forge web frontend HTTP/3.
///
/// Serves the Forge web UI over HTTP/3 via iroh QUIC. Used by the h3
/// compatibility proxy to bridge TCP HTTP/1.1 clients to the forge frontend.
pub const FORGE_WEB_ALPN: &[u8] = b"aspen/forge-web/1";

/// ALPN protocol identifier for Nostr relay over QUIC.
///
/// Accepts iroh connections and processes Nostr protocol messages using
/// length-prefixed framing (4-byte BE length + UTF-8 JSON payload).
/// Runs alongside the TCP WebSocket listener, sharing the same event
/// store and subscription registry.
pub const NOSTR_WS_ALPN: &[u8] = b"/aspen/nostr-ws/1";

/// ALPN protocol identifier for trust share collection.
///
/// Used by the trust reconfiguration coordinator to request epoch shares
/// from current voting members before proposing a rotated configuration.
pub const TRUST_ALPN: &[u8] = b"/aspen/trust/1";

/// ALPN protocol identifier for DAG sync.
///
/// Used for streaming deterministic DAG traversal between nodes.
/// Supports full traversal, partial sync (known heads), stem/leaf split,
/// and inline predicates for selective data transfer.
///
/// Canonical source: `aspen_dag::protocol::DAG_SYNC_ALPN`
pub const DAG_SYNC_ALPN: &[u8] = b"/aspen/dag-sync/1";

/// Transport-owned ALPN namespace contract.
#[allow(deprecated)]
pub const TRANSPORT_ALPN_PROTOCOLS: &[AlpnProtocolContract] = &[
    AlpnProtocolContract {
        name: "RAFT_ALPN",
        alpn: RAFT_ALPN,
        namespace: "raft-legacy",
    },
    AlpnProtocolContract {
        name: "RAFT_AUTH_ALPN",
        alpn: RAFT_AUTH_ALPN,
        namespace: "raft",
    },
    AlpnProtocolContract {
        name: "RAFT_SHARDED_ALPN",
        alpn: RAFT_SHARDED_ALPN,
        namespace: "raft-sharded",
    },
    AlpnProtocolContract {
        name: "CLIENT_ALPN",
        alpn: CLIENT_ALPN,
        namespace: "client-rpc",
    },
    AlpnProtocolContract {
        name: "LOG_SUBSCRIBER_ALPN",
        alpn: LOG_SUBSCRIBER_ALPN,
        namespace: "log-subscriber",
    },
    AlpnProtocolContract {
        name: "NET_TUNNEL_ALPN",
        alpn: NET_TUNNEL_ALPN,
        namespace: "net-tunnel",
    },
    AlpnProtocolContract {
        name: "NIX_CACHE_H3_ALPN",
        alpn: NIX_CACHE_H3_ALPN,
        namespace: "nix-cache-h3",
    },
    AlpnProtocolContract {
        name: "FORGE_WEB_ALPN",
        alpn: FORGE_WEB_ALPN,
        namespace: "forge-web",
    },
    AlpnProtocolContract {
        name: "NOSTR_WS_ALPN",
        alpn: NOSTR_WS_ALPN,
        namespace: "nostr-ws",
    },
    AlpnProtocolContract {
        name: "TRUST_ALPN",
        alpn: TRUST_ALPN,
        namespace: "trust",
    },
    AlpnProtocolContract {
        name: "DAG_SYNC_ALPN",
        alpn: DAG_SYNC_ALPN,
        namespace: "dag-sync",
    },
];

/// Maximum concurrent Client connections.
///
/// Set high enough to handle bursts of short-lived CLI connections.
/// Each CLI invocation creates a new QUIC connection that persists briefly
/// on the server after the client exits.
pub const MAX_CLIENT_CONNECTIONS: u32 = 200;

/// Maximum concurrent streams per Client connection.
pub const MAX_CLIENT_STREAMS_PER_CONNECTION: u32 = 10;

// ============================================================================
// Compile-Time Constant Assertions
// ============================================================================

// Connection limits must be positive
const _: () = assert!(MAX_CLIENT_CONNECTIONS > 0);
const _: () = assert!(MAX_CLIENT_STREAMS_PER_CONNECTION > 0);

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn transport_alpn_protocols_are_unique_and_well_formed() {
        let mut names = BTreeSet::new();
        let mut alpns = BTreeSet::new();
        let mut namespaces = BTreeSet::new();

        for protocol in TRANSPORT_ALPN_PROTOCOLS {
            assert!(!protocol.name.is_empty(), "ALPN contract names must be nonempty");
            assert!(!protocol.namespace.is_empty(), "ALPN namespaces must be nonempty");
            assert!(!protocol.alpn.is_empty(), "ALPN `{}` must be nonempty", protocol.name);
            assert!(names.insert(protocol.name), "duplicate ALPN constant name `{}`", protocol.name);
            assert!(namespaces.insert(protocol.namespace), "duplicate ALPN namespace `{}`", protocol.namespace);
            assert!(alpns.insert(protocol.alpn), "duplicate ALPN bytes for `{}`", protocol.name);
            assert!(
                protocol.alpn.iter().all(|byte| byte.is_ascii_graphic()),
                "ALPN `{}` must use printable non-whitespace ASCII bytes",
                protocol.name
            );
        }
    }

    #[test]
    fn transport_alpn_contract_is_documented() {
        let adr = include_str!("../../../docs/adr/009-alpn-protocol-routing.md");

        assert!(adr.contains("TRANSPORT_ALPN_PROTOCOLS"));
        assert!(adr.contains("transport-owned"));

        for protocol in TRANSPORT_ALPN_PROTOCOLS {
            assert!(adr.contains(protocol.name), "ADR must document `{}`", protocol.name);
            assert!(adr.contains(protocol.namespace), "ADR must document ALPN namespace `{}`", protocol.namespace);
        }
    }
}
