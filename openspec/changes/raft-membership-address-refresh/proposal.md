## Why

When a node restarts, iroh binds to a new port (same endpoint ID via `--iroh-secret-key`). The Raft membership log stores the old `RaftMemberInfo` with stale socket addresses. Gossip discovers the new address and updates the network factory's `peer_addrs` cache, but the authoritative Raft membership is never corrected. This causes a permanent divergence between where Raft thinks peers are and where they actually are — every `new_client()` call must check gossip as a fallback, and any code path that reads membership directly (health checks, metrics, topology sync) returns stale addresses.

## What Changes

- Add a membership address refresh mechanism: when gossip discovers a peer with the same endpoint ID but different socket addresses, the leader updates the Raft membership via `add_learner` with the new `RaftMemberInfo`. This is a no-op for membership topology (voter/learner sets unchanged) but updates the `Node` metadata.
- Wire the refresh into the gossip `on_peer_discovered` callback so it triggers automatically.
- Add leader-only gating: only the leader can write to the Raft log, so followers skip the update (the leader will handle it after it receives the same gossip announcement).
- Add rate limiting: don't spam `add_learner` on every gossip tick — only update when the address actually changed vs what's in the current membership.

## Capabilities

### New Capabilities

- `membership-address-refresh`: Automatic propagation of gossip-discovered address changes into Raft membership state.

### Modified Capabilities

## Impact

- `crates/aspen-raft/src/node/` — new method to update a member's address in the Raft log
- `crates/aspen-cluster/src/gossip_discovery.rs` — extend the `on_peer_discovered` callback to trigger membership updates
- `crates/aspen-cluster/src/bootstrap/` — wire the Raft handle into gossip discovery setup
- `crates/aspen-raft/src/network/factory.rs` — expose method to read current membership addresses for staleness comparison
