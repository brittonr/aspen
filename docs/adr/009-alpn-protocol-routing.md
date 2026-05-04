# 9. ALPN-Based Protocol Routing

**Status:** accepted

## Context

Aspen runs multiple protocols over a single Iroh QUIC endpoint: Raft consensus, client RPC, blob transfer, gossip, TUI connections, log subscriptions, net tunnels, and Nix cache gateway. Each protocol has different message formats, authentication requirements, and handler logic.

ALPN (Application-Layer Protocol Negotiation) is a TLS/QUIC extension that lets endpoints agree on a protocol during the handshake. Iroh exposes ALPN-based routing: register a handler for an ALPN identifier, and incoming connections with that ALPN are dispatched to that handler.

## Decision

Each Aspen protocol registers its own ALPN identifier. Transport-owned ALPN constants are defined in `crates/aspen-transport/src/constants.rs` and must appear in the `TRANSPORT_ALPN_PROTOCOLS` namespace registry. App-specific protocol crates may own additional ALPN constants next to their handlers, but transport-owned constants use this registry as the source of truth for uniqueness, formatting, and documentation tests.

| Namespace | Constant | ALPN | Purpose |
|-----------|----------|------|---------|
| `raft-legacy` | `RAFT_ALPN` | `raft-rpc` | Legacy unauthenticated Raft RPC retained only for migration |
| `raft` | `RAFT_AUTH_ALPN` | `raft-auth` | Authenticated Raft RPC with HMAC-SHA256 challenge-response |
| `raft-sharded` | `RAFT_SHARDED_ALPN` | `raft-shard` | Sharded Raft for partitioned state machines |
| `client-rpc` | `CLIENT_ALPN` | `aspen-client` | Client-to-cluster RPC |
| `log-subscriber` | `LOG_SUBSCRIBER_ALPN` | `aspen-logs` | Real-time log streaming |
| `net-tunnel` | `NET_TUNNEL_ALPN` | `/aspen/net-tunnel/0` | Service mesh tunnels |
| `nix-cache-h3` | `NIX_CACHE_H3_ALPN` | `iroh+h3` | HTTP/3 gateway for Nix binary cache |
| `forge-web` | `FORGE_WEB_ALPN` | `aspen/forge-web/1` | Forge web frontend HTTP/3 |
| `nostr-ws` | `NOSTR_WS_ALPN` | `/aspen/nostr-ws/1` | Nostr relay over QUIC |
| `trust` | `TRUST_ALPN` | `/aspen/trust/1` | Trust share collection |
| `dag-sync` | `DAG_SYNC_ALPN` | `/aspen/dag-sync/1` | DAG sync |

Adding a new transport-owned protocol means: define an ALPN constant, add it to `TRANSPORT_ALPN_PROTOCOLS`, implement the handler, register it with the Iroh endpoint, and update this table in the same change.

Alternatives considered:

- (+) Port-based routing: simple, standard (HTTP on 80, gRPC on 443)
- (-) Port-based: requires managing multiple ports, firewall rules, NAT mappings
- (+) Path-based routing (like HTTP paths): single port, flexible
- (-) Path-based: requires an HTTP layer, doesn't work for non-HTTP protocols
- (+) ALPN-based: single endpoint, protocol-level multiplexing, no port management
- (-) ALPN-based: less familiar pattern, requires ALPN-aware clients

## Consequences

- Single Iroh endpoint handles all protocols — one port, one NAT mapping, one relay connection
- Protocol handlers are isolated — a bug in the TUI handler can't corrupt Raft traffic
- Authentication is per-protocol — Raft uses HMAC challenge-response, client RPC uses capability tokens
- New protocols are additive — registering a new ALPN doesn't affect existing handlers
- Transport-owned ALPN constants are centralized in one registry for discoverability; app-specific ALPNs stay near their handlers
- The legacy unauthenticated `RAFT_ALPN` is deprecated but retained for migration
