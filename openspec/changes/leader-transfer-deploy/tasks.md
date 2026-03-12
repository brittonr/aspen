## 1. Expose leadership transfer in ClusterController

- [ ] 1.1 Add `transfer_leader(&self, target: u64) -> Result<(), ControlPlaneError>` to `ClusterController` trait in `crates/aspen-traits/src/lib.rs`, with blanket `Arc<T>` impl
- [ ] 1.2 Implement in `RaftNode` (in `crates/aspen-raft/src/node/cluster_controller.rs`) via `self.raft().trigger().transfer_leader(target.into())`
- [ ] 1.3 Add no-op `transfer_leader` to in-memory test implementations of `ClusterController` (search for existing test impls)
- [ ] 1.4 Write unit test: call `transfer_leader` on a 3-node `RaftNode` cluster, verify leadership changes

## 2. Wire transfer into DeploymentCoordinator

- [ ] 2.1 Add `DeployError::LeadershipTransferred` variant to the deploy error type
- [ ] 2.2 Give `DeploymentCoordinator` access to `ClusterController` (add field if not already present, or pass via constructor)
- [ ] 2.3 Replace `prepare_leader_upgrade()` stub: find first `Healthy` follower, persist record, call `transfer_leader()`, poll metrics until leadership transfers, return `LeadershipTransferred`
- [ ] 2.4 Update `run_deployment()`: catch `LeadershipTransferred` from `prepare_leader_upgrade()`, log it, and return the current record without calling `upgrade_single_node` for the leader or `finalize_deployment`

## 3. Harden resume path

- [ ] 3.1 Review `resume_deployment()` in `coordinator/mod.rs`: verify it upgrades `Pending` nodes via `upgrade_single_node()` and calls `finalize_deployment()` after all nodes are `Healthy`
- [ ] 3.2 Add test: mock scenario where new leader resumes a deployment with one `Pending` node (the old leader) and two `Healthy` nodes — verify the pending node gets `send_upgrade` called and deployment finalizes

## 4. Update multi-node deploy VM test

- [ ] 4.1 Update `nix/tests/ci-dogfood-deploy-multinode.nix`: assert deploy stage `status == "success"` (not just present)
- [ ] 4.2 Add log assertions: verify coordinator logs show leadership transfer ("transferring leadership", "leadership transferred") and new leader resuming the deployment
- [ ] 4.3 Verify all 3 nodes end up healthy and KV data survives

## 5. Build and validate

- [ ] 5.1 Run `cargo nextest run -E 'test(/deploy/)' -E 'test(/transfer/)' -E 'test(/coordinator/)'` and verify all pass
- [ ] 5.2 Run `nix build .#checks.x86_64-linux.ci-dogfood-deploy-multinode-test --impure --option sandbox false` and verify deploy stage succeeds
- [ ] 5.3 Review test logs: confirm follower-first ordering, leadership transfer, resume on new leader, old leader upgraded as follower
