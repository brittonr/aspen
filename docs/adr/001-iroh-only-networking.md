# 1. Iroh-Only Networking

**Status:** accepted

## Context

Aspen needs a networking layer for all inter-node communication (Raft consensus, client RPC, blob transfer, real-time sync) and client-to-cluster communication. The standard approach for distributed systems is HTTP/REST or gRPC over TCP.

Iroh provides P2P QUIC networking with built-in NAT traversal, relay servers for connectivity behind firewalls, and connection multiplexing via ALPN (Application-Layer Protocol Negotiation). It handles peer discovery, hole-punching, and encrypted transport without requiring infrastructure like load balancers or TLS certificate management.

## Decision

All network communication in Aspen uses Iroh QUIC. There is no HTTP API. Each protocol (Raft, client RPC, blob transfer, gossip, TUI) gets its own ALPN identifier and handler.

Specifically:

- Client APIs use `CLIENT_ALPN` via `ClientProtocolHandler`
- Node-to-node Raft uses `RAFT_AUTH_ALPN` with HMAC-SHA256 challenge-response
- Blob transfer uses the iroh-blobs protocol
- Real-time document sync uses iroh-docs CRDT replication
- Additional protocols (sharded Raft, log subscriber, net tunnels, Nix cache) each have dedicated ALPNs

Alternatives considered:

- (+) HTTP/REST: universal client support, standard tooling (curl, browsers)
- (-) HTTP/REST: requires TLS certificate management, load balancers, no built-in NAT traversal
- (+) gRPC: strong typing, code generation, streaming
- (-) gRPC: still TCP-based, same NAT/TLS issues, adds protobuf dependency
- (+) Iroh QUIC: built-in NAT traversal, encryption, multiplexing, peer discovery
- (-) Iroh QUIC: non-standard, requires custom clients, no browser access

## Consequences

- Nodes work behind NAT without port forwarding — relay servers handle connectivity
- Single Iroh endpoint multiplexes all protocols — no port management
- Adding a new protocol means registering an ALPN handler, not standing up a new server
- Standard HTTP tools (curl, Prometheus scrapers) cannot talk to Aspen directly
- The CLI and TUI must use the Iroh client library, not HTTP requests
- Third-party integrations require an Iroh-speaking client or a gateway proxy
