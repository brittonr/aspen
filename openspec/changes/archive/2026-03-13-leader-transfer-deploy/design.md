## Context

The `DeploymentCoordinator::run_deployment()` upgrades followers over iroh RPC, then calls `upgrade_single_node(self.node_id)` for the leader. This fails because iroh's `Endpoint::connect()` rejects self-connections at the QUIC level. The `prepare_leader_upgrade()` method exists as a stub (just logs). The `leader_resume.rs` watcher already detects leader transitions and calls `check_and_resume()`, which has logic to walk node states and resume pending upgrades. OpenRaft has `trigger().transfer_leader(to)` fully implemented but Aspen doesn't expose it.

## Goals / Non-Goals

**Goals:**

- Leader self-upgrade works in multi-node clusters via leadership transfer
- Every node upgrade goes through the same iroh RPC path (no local bypass)
- Coordinator is always alive and in control during the entire deployment
- Multi-node deploy VM test's deploy stage passes (not fails)

**Non-Goals:**

- Single-node cluster self-upgrade (fundamentally can't transfer leadership with no followers)
- CLI command for manual leadership transfer (can be added later)
- Changing the deploy executor's binary swap / restart logic

## Decisions

### 1. Transfer leadership to the first healthy upgraded follower

In `prepare_leader_upgrade()`, pick the first node in the deployment record with status `Healthy` (meaning it was already upgraded and health-checked). Call `cluster_controller.transfer_leader(target_node_id)`. Wait for the transfer to complete by polling `get_metrics()` until `current_leader != self.node_id`.

After transfer, `run_deployment()` returns early — it no longer calls `upgrade_single_node()` for the old leader. The deployment record stays in `Deploying` state with the old leader's node status as `Pending`.

**Alternative considered**: Transfer then continue upgrading the old leader from the same coordinator. Rejected because after transfer, the old leader can no longer do linearizable KV writes to update deployment state. The new leader must take over.

### 2. New leader resumes via existing `leader_resume.rs` → `check_and_resume()`

When the transferred follower becomes leader, `leader_resume.rs` fires and calls `check_and_resume()`. This finds the deployment in `Deploying` state and calls `resume_deployment()`. The resume logic walks node states: `Healthy` nodes are skipped, `Pending` nodes are upgraded via `upgrade_single_node()`. The old leader is the only `Pending` node. The new leader sends `NodeUpgrade` RPC to it as a regular follower — no self-connect, just a normal iroh RPC.

The existing `resume_deployment()` logic handles this correctly: it iterates nodes, skips Healthy ones, and calls `upgrade_single_node()` for Pending ones. The only gap is it should call `finalize_deployment()` after all nodes are Healthy.

### 3. `ClusterController` gets `transfer_leader()` method

Add `transfer_leader(&self, target: u64) -> Result<(), ControlPlaneError>` to the trait. Implement in `RaftNode` by calling `self.raft().trigger().transfer_leader(target.into())`. The in-memory test implementation returns `Ok(())`.

The coordinator already has access to `ClusterController` via `self.cluster_controller` (it uses `current_state()` and `get_metrics()`). No new dependencies needed.

### 4. `prepare_leader_upgrade()` drives the transfer

Replace the stub with:

1. Find first `Healthy` node in the record (an upgraded follower)
2. If none found (single-node cluster), return error — can't transfer
3. Persist the deployment record (so the new leader can find it)
4. Call `cluster_controller.transfer_leader(target_id)`
5. Poll `get_metrics()` until `current_leader != self.node_id` (bounded timeout)
6. Return a sentinel error (e.g., `DeployError::LeadershipTransferred`) so `run_deployment()` returns early without calling `upgrade_single_node()` or `finalize_deployment()`

### 5. `run_deployment()` catches the transfer sentinel

When `prepare_leader_upgrade()` returns `LeadershipTransferred`, `run_deployment()` logs it and returns the current record. The deployment is not finalized — that's the new leader's job after it upgrades the old leader.

## Risks / Trade-offs

**[Leadership transfer latency]** → OpenRaft's transfer is fast (one round-trip to the target). The target is already a voter with an up-to-date log. Expect <100ms in practice.

**[Transfer fails]** → The target follower might be unreachable between being marked Healthy and the transfer call. Mitigation: try each Healthy follower in sequence until one succeeds. If all fail, the deployment fails cleanly — the coordinator is still alive.

**[New leader's resume fires too early]** → The new leader might try to resume before the old leader has finished persisting the deployment record. Mitigation: the record is already persisted before `transfer_leader()` is called (step 3 above). The new leader will see the correct state.

**[Single-node clusters]** → No followers to transfer to. Return a clear error from `prepare_leader_upgrade()`. The dogfood-local.sh script already handles single-node deploy at the script level (stop → swap → restart).
