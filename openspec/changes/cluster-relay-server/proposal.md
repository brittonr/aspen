## Why

Aspen currently depends on n0's public relay infrastructure for NAT traversal and connection facilitation. This creates an external dependency that conflicts with Aspen's self-hosted infrastructure goal (forge, CI, binary cache). Running relay servers on every Raft node eliminates this dependency, provides access control (only cluster members can relay), and improves latency by co-locating relays with the cluster.

## What Changes

- Every Raft node in the cluster spawns an embedded `iroh-relay` server alongside its existing iroh endpoint
- New `relay-server` feature flag gating the `iroh-relay/server` dependency
- Relay server configuration added to `IrohConfig` (bind addresses, TLS, rate limits)
- Cluster-aware access control: only endpoints known to the Raft cluster are allowed to relay
- Dynamic `RelayMap` construction from Raft membership — as nodes join/leave, the relay map updates
- Nodes auto-configure `RelayMode::Custom` pointing at the cluster's own relay URLs
- Relay URLs advertised via gossip/Raft state so clients and new nodes discover them

## Capabilities

### New Capabilities
- `relay-server`: Embedded iroh relay server running on each Raft node with cluster-aware access control, dynamic relay map from membership, and TLS support

### Modified Capabilities
- `transport`: Relay configuration gains a new mode where nodes use the cluster's own relays instead of external infrastructure. Discovery mechanisms propagate relay URLs.

## Impact

- **Dependencies**: `iroh-relay = { version = "0.95.1", features = ["server"] }` added behind `relay-server` feature
- **Ports**: Each node needs 1-2 additional ports (HTTP/HTTPS for relay, optionally QUIC relay port)
- **TLS**: Relay HTTPS requires certificates (Let's Encrypt or manual). Non-TLS HTTP available for dev/testing.
- **Code**: New crate or module for relay lifecycle, config extensions in `aspen-cluster`, relay map integration in `IrohEndpointManager`
- **Config**: New `relay_server` section in node configuration with bind address, TLS mode, rate limits
