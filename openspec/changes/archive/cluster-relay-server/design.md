## Context

Aspen currently uses n0's public relay infrastructure (`RelayMode::Default`) for NAT traversal. Nodes configure relay mode in `IrohEndpointConfig` and the endpoint manager resolves it into iroh's `RelayMode` enum. The `iroh-relay` crate (v0.95.1) is already a transitive dependency — its `server` feature provides a complete relay server implementation with HTTP/HTTPS, QUIC (QAD), TLS (Let's Encrypt or manual), access control, and rate limiting.

The node startup sequence in `aspen_node/main.rs` is: config → bootstrap → extract components → setup client protocol → setup router → wait for shutdown. The relay server needs to be spawned early (before the iroh endpoint's relay mode is configured) so the node can point its own endpoint at the cluster's relays.

## Goals / Non-Goals

**Goals:**

- Every Raft node runs an embedded iroh relay server
- Cluster members auto-discover each other's relay URLs via Raft membership state
- Access control restricts relaying to known cluster endpoints only
- Nodes use the cluster's own relays instead of n0's public infrastructure
- Clean integration with existing config, startup, and shutdown sequences
- Feature-gated behind `relay-server` so it's opt-in

**Non-Goals:**

- Public relay service (not open to arbitrary internet users)
- Relay server as a standalone binary (it's embedded in aspen-node)
- Automatic TLS provisioning via Let's Encrypt in the initial implementation (manual certs or plain HTTP first, ACME later)
- Custom relay protocol modifications (we use iroh-relay's standard protocol as-is)
- Relay load balancing or smart relay selection (iroh handles this via its RelayMap)

## Decisions

### 1. Embed relay in aspen-cluster, not a new crate

**Decision**: Add the relay server lifecycle to `aspen-cluster` behind the `relay-server` feature, rather than creating a separate `aspen-relay` crate.

**Rationale**: The relay server is tightly coupled to cluster membership (for access control and relay map construction). It shares the same lifecycle as the iroh endpoint. A separate crate would need to reach back into cluster state anyway. Keeping it in `aspen-cluster` avoids circular dependencies and keeps the relay as a natural extension of the endpoint manager.

**Alternative considered**: New `aspen-relay` crate — rejected because the relay needs direct access to Raft membership for access control, which would create coupling between the crates anyway.

### 2. All Raft nodes run a relay

**Decision**: Every node in the Raft cluster spawns a relay server. No leader-only or role-based relay designation.

**Rationale**: Maximizes redundancy (no SPOF), eliminates the need for relay role assignment or failover logic, and keeps the design simple. The overhead is minimal — relay servers are lightweight packet forwarders. iroh's `RelayMap` naturally supports multiple relays and clients pick the best one.

**Alternative considered**: Leader-only relay — rejected because leader transitions would cause relay disruptions, and the relay should be the most stable part of the infrastructure.

### 3. HTTP-only for initial implementation, HTTPS as follow-up

**Decision**: Start with plain HTTP relay (no TLS) suitable for development and trusted networks. Add TLS (manual certs, then ACME) as a follow-up.

**Rationale**: The relay protocol itself is encrypted end-to-end (iroh uses QUIC with encryption). TLS on the relay is defense-in-depth, not strictly required for security. Starting without TLS reduces initial complexity (no cert management). Production deployments on untrusted networks should add TLS later.

**Alternative considered**: Require TLS from day one — rejected for initial implementation simplicity. The relay traffic is already encrypted by iroh.

### 4. Store relay URLs in Raft node metadata

**Decision**: Each node's relay URL is stored as part of its Raft node metadata (the `Node` type in openraft). When membership changes, all nodes can reconstruct the `RelayMap` from the current membership.

**Rationale**: Raft membership is the source of truth for "who is in the cluster." Adding relay URLs to node metadata means relay discovery piggybacks on existing membership propagation — no separate discovery mechanism needed. This is consistent with how `iroh_addr` is already stored in node metadata.

**Alternative considered**: Gossip-based relay URL propagation — rejected because Raft membership is already the canonical source, and gossip would add eventual consistency delays for something that should be immediately consistent.

### 5. Relay server binds on a separate port from the iroh QUIC endpoint

**Decision**: The relay HTTP server binds on a configurable port (default 3340), separate from the iroh QUIC endpoint port. The optional QUIC relay (QAD) binds on port 7842 (iroh default).

**Rationale**: The relay server speaks HTTP/WebSocket (for relay protocol) and optionally QUIC (for QAD — QUIC Address Discovery). These are different protocols from the Aspen iroh endpoint's QUIC ALPN-based communication. Sharing a port would conflict.

### 6. Relay server spawns before iroh endpoint relay mode configuration

**Decision**: The relay server is spawned during node bootstrap, before the iroh endpoint is configured with its relay mode. The node's own relay URL is included in the `RelayMap` only after the server is confirmed to be listening.

**Rationale**: The iroh endpoint needs to know the relay URLs at creation time (they're set via `RelayMode::Custom` in the builder). The relay server must be running first so we know the actual bound address. During initial bootstrap of a fresh cluster, the first node uses its own relay. Subsequent nodes discover existing relays from the cluster ticket or gossip.

### 7. Access control via cluster membership check

**Decision**: Use `iroh_relay::server::AccessConfig::Restricted` with a callback that checks whether the connecting endpoint ID is a known cluster member (present in Raft membership or known peer list).

**Rationale**: Prevents abuse of the relay by unauthorized nodes. The check is lightweight (membership lookup) and integrates naturally with the relay server's built-in access control. For the initial implementation, allow all peers that are in the gossip peer set or Raft membership.

## Risks / Trade-offs

- **[Port management]** Each node needs 1-2 additional ports (HTTP relay + optional QUIC QAD). → Mitigation: Configurable ports with sensible defaults (3340, 7842). Document port requirements.

- **[Bootstrap chicken-and-egg]** The first node in a new cluster has no other relays to use. → Mitigation: First node starts with its own relay only (or `RelayMode::Default` fallback). Once other nodes join, the relay map expands. Optionally fall back to n0 relays if cluster relays are unreachable.

- **[Relay URL stability]** Relay URLs contain IP:port which change if nodes move. → Mitigation: For production, use DNS names that resolve to node IPs. For dev/testing, direct IPs work fine. This is a general infrastructure concern, not specific to this feature.

- **[No TLS initially]** Plain HTTP relay on untrusted networks could allow traffic analysis. → Mitigation: Relay traffic is already encrypted by iroh (end-to-end QUIC encryption). TLS is defense-in-depth, planned as follow-up. Document the security model clearly.

- **[Memory overhead]** Each relay server has a key cache (default 1M entries = ~56MB). → Mitigation: Use a much smaller cache for embedded relays (e.g., 1024 entries). Cluster relays serve far fewer clients than public relays.

## Open Questions

- Should the relay server also expose the QUIC relay (QAD) endpoint, or is HTTP-only sufficient for initial implementation?
- Should there be a config option to disable the relay server on specific nodes (e.g., resource-constrained nodes)?
- How should the cluster ticket encode relay URLs for new nodes joining? Extend the existing ticket format?
