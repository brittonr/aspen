## Context

The deploy pipeline (Forge → CI → Build → Deploy) is wired end-to-end, but the `DeploymentCoordinator`'s rolling upgrade path has only been tested in a 1-node cluster where it fails because iroh can't open a connection to itself. The `dogfood-local.sh` script works around this with a process-level binary swap. The coordinator's core logic — follower-first ordering, quorum checks via `can_upgrade_node()`, drain → upgrade → health per node — has never been exercised in a VM integration test.

Existing multi-node VM tests (`multi-node-cluster.nix`, `multi-node-kv.nix`) prove 3-node cluster formation works. The 1-node deploy test (`ci-dogfood-deploy.nix`) proves the Forge → CI → deploy-stage pipeline works. This change combines both patterns.

## Goals / Non-Goals

**Goals:**

- Prove `DeploymentCoordinator` rolling upgrade works across a 3-node cluster in a VM
- Exercise follower-first ordering (followers upgraded before leader)
- Verify quorum is maintained throughout (cluster never loses majority)
- Verify data written before deploy survives the upgrade
- Deploy stage in CI pipeline succeeds (not fails-as-expected)

**Non-Goals:**

- Self-upgrade (node upgrading itself) — that's a separate change
- Testing with different binary versions (both sides run the same build)
- Canary or blue-green strategies (only rolling)
- Performance benchmarks for upgrade speed

## Decisions

### 1. Use cowsay flake as the deploy artifact (not aspen-node itself)

The test pushes a trivial Nix flake (cowsay wrapper) to Forge, CI builds it, and the deploy stage runs with that artifact. We don't deploy an actual aspen-node binary because:

- Building aspen-node inside a VM would take too long and too much RAM
- The coordinator doesn't care what the artifact is — it resolves the store path and dispatches `NodeUpgrade` RPCs
- The existing `ci-dogfood-deploy.nix` already uses this pattern

The test verifies the *coordinator path* (dispatch, ordering, health gating), not the *upgrade executor* (binary swap, restart).

**Alternative considered**: Deploy a real aspen-node binary pre-built outside the VM. Rejected because the NodeUpgrade executor would restart the node, killing the Raft leader mid-test. The coordinator's job is to orchestrate — the upgrade executor is tested separately.

### 2. Three VMs, one CI-capable node

Only node1 runs CI workers (`enableCi = true`, `enableWorkers = true`). Nodes 2 and 3 are plain Raft voters. This matches production topology where not every node runs CI.

Node1 is also the initial leader and the bootstrap node. The deploy coordinator runs on the leader, so node1 dispatches upgrades to nodes 2 and 3 first (follower-first), then itself last.

### 3. Reuse multi-node-cluster.nix patterns for cluster formation

Follow the exact same cluster bootstrap sequence: `cluster init` → `add-learner` × 2 → `change-membership 1 2 3`. Use deterministic iroh secret keys, shared cookie, and `get_endpoint_addr_json()` helper.

### 4. Validate deploy state transitions via KV, not just final status

The coordinator writes deployment state to `_sys:deploy:current` and per-node status. The test should check:

- Pipeline status transitions: `running` → `succeeded`
- Deploy stage status: `succeeded` (not `failed` like the 1-node test)
- Node count in deployment matches cluster size

### 5. Write KV data before deploy, read after

Write a sentinel KV key before the deploy pipeline starts. After the deploy completes and all nodes are healthy, read it back. This proves data survives the coordinator's drain/upgrade cycle.

## Risks / Trade-offs

**[RAM usage ~12GB]** → Three VMs at 4GB each. Acceptable for CI-profile tests. Tag the test so it doesn't run in `quick` profile.

**[Nix build inside VM is slow]** → The cowsay flake is tiny (fetches from cache). Build time should be under 30s. Set pipeline poll timeout to 300s as safety margin.

**[Deploy stage "succeeds" but doesn't actually restart nodes]** → Because cowsay isn't an aspen-node binary, the NodeUpgrade executor will fail to find/replace the running binary. The coordinator marks the node as "upgraded" based on the RPC response, not on an actual restart. This is fine — we're testing the orchestration, not the binary swap. → If the executor rejects the upgrade, the test should still verify the coordinator handled the rejection gracefully and the deploy stage status reflects reality.

**[Flaky cluster formation]** → Multi-node tests occasionally hit timing issues with learner promotion. Use `time.sleep()` between membership changes and `wait_until_succeeds` for health checks, matching existing patterns.
