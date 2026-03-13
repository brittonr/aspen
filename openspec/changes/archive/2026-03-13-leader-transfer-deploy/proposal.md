## Why

The `DeploymentCoordinator` upgrades followers over iroh RPC, then tries the same path for the leader — which fails because iroh's `Endpoint::connect()` hard-rejects self-connections (QUIC can't use a single socket as both client and server). The multi-node deploy VM test confirmed: followers 2 and 3 upgrade successfully, then the leader hits "Connecting to ourself is not supported." Today's `prepare_leader_upgrade()` is a stub that logs and hopes. The right fix is to transfer Raft leadership to an already-upgraded follower before upgrading the old leader, so every node takes the same RPC code path.

## What Changes

- Expose openraft's `trigger().transfer_leader()` through `ClusterController` trait and `RaftNode`
- Replace the `prepare_leader_upgrade()` stub in `DeploymentCoordinator` with actual leadership transfer to a healthy upgraded follower
- The new leader's `leader_resume.rs` watcher picks up the in-progress deployment and upgrades the old leader as a regular follower (over iroh RPC — no self-connect)
- Harden `resume_deployment()` to handle the "old leader is the only Pending node" case cleanly
- Update the multi-node deploy VM test to assert the deploy stage succeeds (not fails)

## Capabilities

### New Capabilities

- `leader-transfer`: Raft leadership transfer exposed as a cluster operation, used by the deploy coordinator to hand off leadership before self-upgrade

### Modified Capabilities

- `ci`: Deploy stage completes successfully in multi-node clusters (leader self-upgrade no longer fails)

## Impact

- `crates/aspen-traits/src/lib.rs`: Add `transfer_leader()` to `ClusterController`
- `crates/aspen-raft/src/node/`: Implement via openraft `trigger().transfer_leader()`
- `crates/aspen-deploy/src/coordinator/mod.rs`: Replace `prepare_leader_upgrade()` stub, adjust `run_deployment()` flow
- `crates/aspen-deploy/src/coordinator/mod.rs`: Harden `resume_deployment()` for the transferred-leader scenario
- `crates/aspen-cluster/src/upgrade/leader_resume.rs`: Verify resume fires correctly on new leader
- `nix/tests/ci-dogfood-deploy-multinode.nix`: Assert deploy stage succeeds
- `crates/aspen-core/` in-memory test implementations: Add `transfer_leader` stub
