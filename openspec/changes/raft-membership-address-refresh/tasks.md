## Tasks

### 1. Add `update_member_address` to RaftNode

- [x] Add method to `crates/aspen-raft/src/node/cluster_controller.rs` that:
  - Takes `node_id: NodeId` and `new_addr: EndpointAddr`
  - Checks `raft.metrics().current_leader == Some(self.node_id())` — returns early if not leader
  - Reads current membership via `metrics.membership_config.membership().nodes()` to get the existing `RaftMemberInfo` for that node_id
  - Compares `existing.iroh_addr.addrs` with `new_addr.addrs` — returns early if identical
  - Compares `existing.iroh_addr.id` with `new_addr.id` — returns early if different (not the same node)
  - Calls `self.raft().add_learner(node_id, updated_member_info, false)` (non-blocking)
  - Logs at INFO on success, WARN on failure (non-fatal — gossip fallback covers the gap)
- [x] Make it `pub` so it can be called from aspen-cluster's gossip callback
- [x] Add debounce: `HashMap<(NodeId, u64), Instant>` where u64 is a hash of the socket addresses. Skip if last update for that key was < 60s ago. Wrap in `Arc<Mutex<>>` since it's called from async callback.

### 2. Expose membership address lookup on RaftNode

- [x] Add `pub fn get_member_addr(&self, node_id: NodeId) -> Option<EndpointAddr>` to RaftNode
  - Reads `raft.metrics().membership_config.membership().get(&node_id)` to get current `RaftMemberInfo`
  - Returns the `iroh_addr` if the node is in membership, None otherwise
- [x] This lets the gossip callback compare gossip address against membership address without needing the full membership

### 3. Wire into gossip discovery callback

- [x] Modify `spawn_gossip_peer_discovery` in `crates/aspen-cluster/src/gossip_discovery.rs`:
  - Accept an additional `Option<MembershipRefreshSlot>` parameter
  - In the `on_peer_discovered` callback, after calling `factory.add_peer()`, call `raft_node.try_update_member_address(debouncer, peer.node_id, peer.address)` if the slot is populated
- [x] Update all callers of `spawn_gossip_peer_discovery` in bootstrap paths:
  - `crates/aspen-cluster/src/bootstrap/node/discovery_init.rs` — pass membership refresh slot
  - `crates/aspen-cluster/src/bootstrap/node/sharding_init.rs` — pass None (sharded mode, separate mechanism)
  - `crates/aspen-cluster/src/bootstrap/node/mod.rs` — create slot, pass to gossip, fill after Raft init

### 4. Handle bootstrap ordering (Raft not yet available during gossip setup)

- [x] Gossip starts before Raft is initialized during bootstrap. Use a deferred approach:
  - Store an `Arc<RwLock<Option<MembershipRefreshState>>>` (type alias: `MembershipRefreshSlot`)
  - After Raft init (Phase 4), call `set_membership_refresh()` to populate the slot
  - The callback checks `if let Some(ref state) = *guard` before attempting update
- [x] Alternative: use an `Arc<dyn MembershipAddressUpdater>` trait to break the dependency on `RaftNode` type
  - Implemented as `MembershipAddressUpdater` trait in `aspen-core/src/transport.rs`
  - `RaftNode` implements the trait (for external consumers)
  - The gossip callback uses the direct `try_update_member_address` method (with shared debouncer)

### 5. Tests

- [x] Unit test in `crates/aspen-raft/src/node/`: `test_hash_addrs_deterministic` — verifies address hashing is stable
- [x] Unit test: `test_hash_addrs_empty_vs_nonempty` — different addrs produce different hashes
- [x] Unit test: `test_debouncer_allows_first_update` — first call is not debounced
- [x] Unit test: `test_debouncer_blocks_duplicate_within_window` — duplicate within 60s is blocked
- [x] Unit test: `test_debouncer_allows_different_hash` — different address hash not debounced
- [x] Unit test: `test_debouncer_allows_different_node` — different node not debounced
- [x] Full workspace check: 1,034 tests in aspen-raft + aspen-core pass with 0 regressions
