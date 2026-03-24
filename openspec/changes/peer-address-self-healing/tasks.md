## 1. ClusterDiscovery implementation

- [x] 1.1 Create `crates/aspen-cluster/src/cluster_discovery.rs` with `ClusterDiscovery` struct holding `data_dir: PathBuf`, `cluster_data_dir: Option<PathBuf>`, and `membership: Arc<dyn Fn(NodeId) -> Option<EndpointAddr> + Send + Sync>`
- [x] 1.2 Implement `Discovery::publish()`: atomic write of `NodeAddr` JSON to `<data_dir>/discovery/<public_key_hex>.json` (write to `.tmp`, rename)
- [x] 1.3 Implement `Discovery::resolve()`: scan `<cluster_data_dir>/*/discovery/<peer_key>.json` for matching peer, return as `DiscoveryItem` stream; fall back to membership addresses
- [x] 1.4 Unit tests: publish writes file, resolve reads it back, resolve falls back to membership when file missing, cookie isolation

## 2. Startup address seeding

- [x] 2.1 In `bootstrap_node()`, after iroh endpoint creation but before Raft init, iterate Raft membership and discovery files to collect peer addresses
- [x] 2.2 Call `endpoint.add_node_addr()` for each peer with the best available address (discovery file > peer_addrs.json > membership)
- [x] 2.3 Log which source each peer address came from (discovery file, cache, membership)

## 3. Wire into endpoint builder

- [x] 3.1 Add `ClusterDiscovery` to endpoint builder via `.address_lookup()` in `IrohEndpointManager::new()`
- [x] 3.2 Derive `cluster_data_dir` from `data_dir.parent()` — the parent directory containing sibling node dirs
- [x] 3.3 Pass Raft membership lookup closure so `resolve()` can fall back to stored addresses

## 4. Shutdown persistence

- [x] 4.1 In `NodeResources::shutdown()`, call `ClusterDiscovery::publish()` with current endpoint addresses before closing the endpoint
- [x] 4.2 Flush cluster discovery in both shutdown paths (NodeResources and CommonShutdownResources)
- [ ] 4.3 Test: clean shutdown produces discovery file, restart reads it

## 5. Integration test

- [ ] 5.1 Write `scripts/test-cluster-discovery.sh`: 3-node cluster with relay disabled and mDNS disabled, all on same machine. Stop all, restart all, assert quorum restores within 10s without CLI intervention
- [ ] 5.2 Verify test: confirm discovery files exist in data dirs after first run, confirm they're read on restart, confirm new addresses are published after restart
