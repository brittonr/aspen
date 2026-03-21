## Why

When an aspen-node restarts (systemctl restart, deploy, crash recovery), its iroh endpoint binds a new port. The endpoint ID stays the same (derived from the persisted secret key), but the socket address changes. Raft membership stores the original `EndpointAddr` from `add_learner` and never updates it. Other nodes keep trying the stale address, so the restarted node can't rejoin the cluster.

This blocks multi-node rolling deploys: after restarting a follower, the cluster can't replicate to it. The gossip discovery system already broadcasts address updates via `PeerAnnouncement`, and the network factory has a fallback `peer_addrs` cache, but `new_client()` always reads from `RaftMemberInfo` first — the stale address wins.

## What Changes

- **Gossip-informed address override in Raft network factory**: When `new_client()` creates a connection, check the gossip-populated `peer_addrs` cache first. If it has a more recent address for the target node (same endpoint ID, different socket addresses), use that instead of the stale `RaftMemberInfo` address.
- **Eager re-announcement on startup**: After a node starts and joins gossip, immediately broadcast a `PeerAnnouncement` with its current addresses. This ensures other nodes update their caches before the next Raft heartbeat.
- **Periodic Raft membership address sync** (optional): A leader-only background task that detects when a node's gossip-discovered address differs from its `RaftMemberInfo` and issues an `add_learner` to update it in the Raft state. This is a consistency fix — the gossip override handles the immediate reconnection, this keeps the persisted state accurate.

## Capabilities

### New Capabilities

- `address-refresh`: Mechanism for cluster nodes to reconnect after address changes (port rebind, network reconfiguration) without manual intervention or membership removal/re-addition

### Modified Capabilities

## Impact

- `crates/aspen-raft/src/network/factory.rs`: `new_client()` checks gossip cache before Raft membership
- `crates/aspen-cluster/src/gossip/discovery/lifecycle.rs`: Eager announcement on startup
- `crates/aspen-cluster/src/bootstrap/`: Trigger gossip announcement after cluster join
- `nix/tests/multi-node-dogfood.nix`: Unblocks the rolling restart test
