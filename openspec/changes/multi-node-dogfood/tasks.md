## 1. Scaffold VM Test

- [x] 1.1 Create `nix/tests/multi-node-dogfood.nix` with 3-node NixOS VM config: node1, node2, node3 with deterministic iroh keys, shared cookie, CI + Forge + workers enabled. Follow `multi-node-proxy.nix` pattern for 3-node setup and `ci-dogfood-full-loop.nix` for CI pipeline config.
- [x] 1.2 Wire test into `flake.nix` as `checks.x86_64-linux.multi-node-dogfood-test`
- [x] 1.3 Write cluster formation test phase: node1 init, add-learner for nodes 2 and 3, change-membership, health check with 3 voters

## 2. Forge + CI on Multi-Node

- [x] 2.1 Add Forge repo creation and git push phase (reuse aspen-constants inner flake pattern from ci-dogfood-full-loop)
- [x] 2.2 Add CI pipeline wait phase: poll `ci status` until success, extract build output path
- [x] 2.3 Verify the CI-built binary is runnable on node1

## 3. Rolling Deploy

- [x] 3.1 Write pre-deploy KV data: store a test key before deploy to verify persistence
- [x] 3.2 Implement follower-first rolling deploy: stop node2, restart with CI binary, wait for health; then node3; then node1 (leader last)
- [x] 3.3 Verify quorum during deploy: KV write succeeds while one follower is down
- [x] 3.4 Verify re-election: after node1 stops, confirm nodes 2+3 elect a new leader and accept writes

## 4. Post-Deploy Verification

- [x] 4.1 Verify all 3 nodes are running with new binary (PID check + process alive)
- [x] 4.2 Verify cluster health: 3 healthy voters, KV round-trip (write on leader, read from follower)
- [x] 4.3 Verify pre-deploy KV data survived all restarts
- [ ] 4.4 Build and run the VM test end-to-end, fix any issues discovered
  - Cluster formation, KV ops, Forge push, CI build, CI-built binary: all pass
  - Prefetch timeout fix (adapters.rs): `nix flake archive` no longer hangs in VMs without internet
  - BLOCKED: After systemctl restart, iroh endpoint gets new port but Raft membership has old address.
    Restarted node can't rejoin cluster. Needs address update mechanism (gossip rediscovery or
    explicit re-registration). This is a real cluster issue to fix in aspen-cluster, not a test bug.

## 5. Script Hardening

- [ ] 5.1 Run `ASPEN_NODE_COUNT=3 nix run .#dogfood-local -- full-loop` and fix any issues in the multi-node paths of `dogfood-local.sh`
