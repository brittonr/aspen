## Why

After a rolling deploy where all nodes restart with new iroh ports, the Raft network factory holds stale peer addresses. With relay enabled, iroh reconnects via relay, but this depends on external infrastructure and adds latency. With relay disabled, the cluster permanently loses quorum — no node can reach any other. The `cluster update-peer` CLI command also can't reach nodes since it relies on the same iroh transport.

Iroh already has a pluggable discovery system (`Discovery` trait) with `publish` (announce own addresses) and `resolve` (find a peer's addresses). The endpoint calls `publish` automatically when addresses change. We need a `Discovery` implementation that works without relay, DNS, or internet — just the local/cluster network.

## What Changes

- Implement `ClusterDiscovery` as an `iroh::Discovery` trait impl that persists peer addresses to a shared file and resolves them on demand
- `publish()`: writes own address to `<data_dir>/peer_addrs/<node_public_key>.json` when addresses change
- `resolve()`: reads `<data_dir>/peer_addrs/<node_public_key>.json` for any peer, falls back to Raft membership addresses
- Wire `ClusterDiscovery` into the endpoint builder via `.add_discovery()` alongside existing mDNS/DNS/Pkarr services
- On startup, seed iroh's address book by calling `endpoint.add_node_addr()` for all peers from Raft membership
- Persist factory's `peer_addrs.json` on clean shutdown with all current addresses

## Capabilities

### New Capabilities

- `cluster-discovery`: An `iroh::Discovery` implementation that persists and resolves peer addresses using the local filesystem and Raft membership, enabling address recovery after restart without relay or external services.

### Modified Capabilities

## Impact

- `crates/aspen-cluster/src/`: New `cluster_discovery.rs` module implementing `iroh::Discovery`
- `crates/aspen-cluster/src/endpoint_manager.rs`: Add `ClusterDiscovery` to endpoint builder
- `crates/aspen-cluster/src/bootstrap/node/mod.rs`: Seed `endpoint.add_node_addr()` on startup from Raft membership
- `crates/aspen-raft/src/network/factory.rs`: Flush peer cache on shutdown
