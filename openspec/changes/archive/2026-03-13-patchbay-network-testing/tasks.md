## 1. Crate Scaffolding

- [x] 1.1 Create `crates/aspen-testing-patchbay/` with `Cargo.toml` pinning `patchbay` to a specific git revision, depending on `aspen-raft`, `aspen-core`, `aspen-cluster`, `iroh`, and `tokio`
- [x] 1.2 Add `aspen-testing-patchbay` to workspace `Cargo.toml` members list
- [x] 1.3 Add `nft` and `tc` to the nix devshell `buildInputs` (iproute2, nftables)
- [x] 1.4 Create `src/lib.rs` with module structure: `harness`, `topologies`, `node`, `skip`

## 2. Runtime Detection and Skip Logic

- [x] 2.1 Implement `skip::userns_available()` — check sysctl `kernel.unprivileged_userns_clone` and attempt a trial `patchbay::init_userns()`, returning bool
- [x] 2.2 Create `skip_unless_patchbay!()` macro that prints diagnostic and returns early from test if userns unavailable

## 3. Node Spawning

- [x] 3.1 Implement `node::bootstrap_node()` that creates a stripped-down Aspen node: iroh endpoint, Raft with in-memory storage, KV store, and Raft RPC server
- [x] 3.2 Implement `transport::TestTransport` lightweight NetworkTransport wrapper for bare iroh endpoints
- [x] 3.3 Implement `node::NodeHandle` channel-based proxy for `init_cluster()`, `write_kv()`, `read_kv()`, `get_leader()`, `applied_index()`, `shutdown()`

## 4. PatchbayHarness Core

- [x] 4.1 Implement `PatchbayHarness` struct holding a `Lab`, a vec of devices, and node handles
- [x] 4.2 Implement `PatchbayHarness::new()` — `Lab::new()`, return harness (init_userns handled by ctor)
- [x] 4.3 Implement `harness.add_node(name, router_idx, link_condition)` — add device, spawn node in namespace via channel handoff
- [x] 4.4 Implement `harness.init_cluster()` — collect addrs, distribute peers, init Raft, wait for leader
- [x] 4.5 Implement `harness.check_leader()` / `wait_for_leader()` — query all nodes, verify consensus
- [x] 4.6 Implement `harness.write_kv(key, value)` and `harness.read_kv(node_idx, key)` — proxy to leader/specific node

## 5. Topology Presets

- [x] 5.1 Implement `PatchbayHarness::three_node_public()` — one `RouterPreset::Public` router, three devices, no NAT
- [x] 5.2 Implement `PatchbayHarness::three_node_home_nat()` — three separate `RouterPreset::Home` routers, one device each
- [x] 5.3 Implement `PatchbayHarness::mixed_nat(public, home, corporate)` — mix of router presets with one device per router
- [x] 5.4 Implement `PatchbayHarness::two_region(eu_nodes, us_nodes, latency_ms)` — two regions with configurable inter-region latency

## 6. NAT Integration Tests

- [x] 6.1 Write test `test_public_baseline` — `three_node_public()`, form cluster, write/read one KV pair
- [x] 6.2 Write test `test_home_nat_cluster_formation` — `three_node_home_nat()`, form cluster, verify leader election within 60s
- [x] 6.3 Write test `test_home_nat_kv_replication` — write KV to leader, read from all followers
- [x] 6.4 Write test `test_corporate_nat_cluster` — three nodes behind Corporate NAT, verify relay-based cluster formation
- [x] 6.5 Write test `test_cgnat_cluster` — three nodes behind CGNAT, verify cluster formation and batch KV writes (100 entries)
- [x] 6.6 Write test `test_mixed_nat_cluster` — one public, one home, one corporate node, verify cluster forms
- [x] 6.7 Write test `test_mixed_nat_leader_failover` — kill leader in mixed topology, verify re-election across NAT boundaries

## 7. Fault Injection Tests

- [x] 7.1 Write test `test_region_partition_majority_quorum` — 2 EU + 1 US, break link, verify EU partition maintains writes, US node loses leader
- [x] 7.2 Write test `test_region_partition_heal_catchup` — break link for 30s, restore, verify isolated node catches up
- [x] 7.3 Write test `test_region_partition_minority_rejects_writes` — verify isolated node rejects writes
- [x] 7.4 Write test `test_latency_200ms_no_false_election` — inject 200ms latency, verify leader stability
- [x] 7.5 Write test `test_latency_exceeds_election_timeout` — inject 5000ms on leader link, verify re-election
- [x] 7.6 Write test `test_packet_loss_10pct_replication` — 10% loss, 50 KV writes, verify all replicate
- [x] 7.7 Write test `test_packet_loss_50pct_no_deadlock` — 50% loss, verify no deadlock, recovery on removal
- [x] 7.8 Write test `test_link_down_follower` — link down on follower, verify quorum maintained
- [x] 7.9 Write test `test_link_up_follower_catchup` — restore follower link, verify log catchup
- [x] 7.10 Write test `test_link_down_leader_failover` — link down on leader, verify re-election
- [x] 7.11 Write test `test_dynamic_nat_change` — switch router from Public to Home NAT mid-operation, verify recovery

## 8. CI Integration

- [x] 8.1 Add `patchbay` nextest profile to `.config/nextest.toml` with 120s timeout and filter expression `test(/patchbay/)`
- [x] 8.2 Skip nix flake check integration — patchbay tests require unprivileged userns which the nix sandbox blocks; tests self-skip via `skip_unless_patchbay!()` macro
- [x] 8.3 Document CI runner requirements (unprivileged userns, nft, tc) in `crates/aspen-testing-patchbay/README.md`
