## Context

Aspen uses iroh QUIC for all inter-node communication. Each node has a persistent endpoint ID (derived from `--iroh-secret-key`) but binds to an ephemeral port on each startup. The Raft membership log stores `RaftMemberInfo` containing the node's `EndpointAddr` (endpoint ID + socket addresses). After a node restart, the socket addresses change but the membership retains the old ones.

The current mitigation uses a gossip-populated `peer_addrs` cache in the network factory. The `new_client()` and `send_rpc()` methods check this cache and prefer fresher gossip addresses over stale membership addresses. This works but is a workaround — the authoritative membership state remains stale.

## Goals / Non-Goals

**Goals:**

- Update Raft membership when gossip discovers a peer with changed addresses
- Only the leader performs updates (Raft write semantics)
- Rate-limit updates to avoid log spam during rapid reconnection
- Transparent to existing code — no changes to `ClusterController` trait

**Non-Goals:**

- Changing how gossip discovery works (it already works correctly)
- Removing the gossip fallback from `new_client()` / `send_rpc()` (keep as defense-in-depth)
- Handling endpoint ID changes (that's a different node, not a restart)

## Decisions

**Use `add_learner` for address updates.** openraft's `add_learner` with `ChangeMembers::AddNodes` updates the `Node` metadata for an existing member without changing voter/learner sets. This is the intended mechanism for updating node info. No new Raft command type needed.

**Leader-only gating via metrics check.** Before calling `add_learner`, check `raft.metrics().current_leader == Some(self_id)`. Followers silently skip — the leader will receive the same gossip and perform the update. No error, no retry.

**Debounce with a HashSet of recently-updated nodes.** Track `(node_id, addrs_hash)` pairs that were recently submitted to `add_learner`. Skip if the same pair was submitted within the last 60 seconds. This prevents log spam when gossip announces the same address repeatedly (10s announcement interval).

**Wire into existing gossip callback.** The `spawn_gossip_peer_discovery` function already creates a callback that calls `factory.add_peer()`. Extend this callback to also trigger the membership update. Pass an `Arc<RaftNode>` (or a closure over it) into the callback.

## Risks / Trade-offs

- **Extra Raft log entries**: Each address update is a membership change entry in the log. With debouncing, this is at most one entry per node restart — negligible.
- **Leader must be reachable**: If the leader itself restarted and can't be reached, its address update waits until a new leader is elected. This is fine — the gossip fallback covers the gap.
- **Race between gossip and first heartbeat**: A node's first heartbeat may arrive before its gossip announcement. The `new_client()` gossip fallback handles this. After the gossip arrives, the leader updates membership, and subsequent `new_client()` calls get the correct address from membership directly.
