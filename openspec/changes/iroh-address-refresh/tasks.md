## 1. Gossip Address Override

- [ ] 1.1 In `crates/aspen-raft/src/network/factory.rs` `new_client()`: before using `node.iroh_addr`, check `peer_addrs` cache. If cache has an entry for `target` with matching endpoint ID but different socket addrs, use the cache address. Log at INFO when overriding.
- [ ] 1.2 Add endpoint ID mismatch guard: if cache endpoint ID differs from `RaftMemberInfo` endpoint ID, log a WARN and fall back to Raft membership address.
- [ ] 1.3 Unit test: mock `peer_addrs` with updated address, verify `new_client()` uses it over `RaftMemberInfo`.
- [ ] 1.4 Unit test: mock `peer_addrs` with different endpoint ID, verify `new_client()` falls back to `RaftMemberInfo`.

## 2. Eager Announcement on Startup

- [ ] 2.1 In the bootstrap sequence (`crates/aspen-cluster/src/bootstrap/`), after gossip setup completes, call `broadcast_announcement()` immediately. Find where `spawn_gossip_peer_discovery()` returns and add the call.
- [ ] 2.2 Verify the announcement fires before Raft starts processing RPCs by checking log ordering in an existing multi-node test.

## 3. Integration Verification

- [ ] 3.1 Run the multi-node-dogfood VM test (`nix build .#checks.x86_64-linux.multi-node-dogfood-test`). The rolling restart subtests should now pass — restarted nodes rejoin within 30s.
- [ ] 3.2 Fix any issues discovered during the VM test run.
- [ ] 3.3 Mark multi-node-dogfood task 4.4 complete if the full test passes end-to-end.
