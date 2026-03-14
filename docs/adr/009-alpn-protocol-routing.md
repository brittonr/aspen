# 9. ALPN-Based Protocol Routing

**Status:** accepted

## Context

Aspen runs multiple protocols over a single Iroh QUIC endpoint: Raft consensus, client RPC, blob transfer, gossip, TUI connections, log subscriptions, net tunnels, and Nix cache gateway. Each protocol has different message formats, authentication requirements, and handler logic.

ALPN (Application-Layer Protocol Negotiation) is a TLS/QUIC extension that lets endpoints agree on a protocol during the handshake. Iroh exposes ALPN-based routing: register a handler for an ALPN identifier, and incoming connections with that ALPN are dispatched to that handler.

## Decision

Each Aspen protocol registers its own ALPN identifier. All ALPN constants are defined in `crates/aspen-transport/src/constants.rs`:

- `RAFT_AUTH_ALPN` (`raft-auth`): Authenticated Raft RPC with HMAC-SHA256 challenge-response
- `RAFT_SHARDED_ALPN` (`raft-shard`): Sharded Raft for partitioned state machines
- `CLIENT_ALPN`: Client-to-cluster RPC
- `LOG_SUBSCRIBER_ALPN`: Real-time log streaming
- `NET_TUNNEL_ALPN` (`/aspen/net-tunnel/0`): Service mesh tunnels
- `NIX_CACHE_H3_ALPN` (`iroh+h3`): HTTP/3 gateway for Nix binary cache

Adding a new protocol means: define an ALPN constant, implement the handler, register it with the Iroh endpoint.

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
- All ALPN constants are centralized in one file for discoverability
- The legacy unauthenticated `RAFT_ALPN` is deprecated but retained for migration
