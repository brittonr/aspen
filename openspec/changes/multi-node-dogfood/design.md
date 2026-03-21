## Context

The dogfood pipeline has two execution contexts:

1. **`dogfood-local.sh`** — runs on the host, starts N processes, pushes source, waits for CI, deploys. Already supports `ASPEN_NODE_COUNT=N` but the multi-node path (add-learner, change-membership, rolling stop/restart) hasn't been exercised.

2. **NixOS VM tests** — existing single-node dogfood tests (`ci-dogfood-test`, `ci-dogfood-full-loop`, `ci-dogfood-deploy`). Multi-node VM tests exist for other features (`multi-node-proxy` uses 3 nodes + client) but not for the deploy pipeline.

The `DeploymentCoordinator` implements follower-first ordering, quorum safety checks, leadership transfer, and health polling. Unit tests and Verus specs cover the pure logic, but the coordinator has never run against a real multi-node Raft cluster with actual process restarts.

## Goals / Non-Goals

**Goals:**

- NixOS VM test with 3 Raft nodes that exercises: cluster formation, Forge push, CI build, and rolling deploy across all 3 nodes
- Verify follower-first upgrade ordering (node 2 and 3 before node 1/leader)
- Verify cluster health after each node upgrade (Raft still responsive)
- Verify the deployed binary is actually running on all nodes post-deploy
- Fix any bugs discovered in `dogfood-local.sh` multi-node paths or the deploy coordinator

**Non-Goals:**

- Leader failover during deploy (tested separately in unit tests + Verus specs; adding it makes the VM test flaky and slow)
- Multi-machine physical deployment (different from same-host multi-process)
- Parallel/concurrent node upgrades (test with max_concurrent=1 for determinism)
- CI cache gateway in the multi-node test (adds complexity without testing deploy)

## Decisions

### 1. Script-level deploy (not DeploymentCoordinator RPC)

The existing `do_deploy()` in `dogfood-local.sh` does script-level deploy: stop process → start with new binary → wait for health. This works for multi-node too — the script stops and restarts nodes one at a time in order (followers first, leader last).

The in-process `DeploymentCoordinator` sends `NodeUpgrade` RPCs, which trigger self-restart. In a single-node cluster this kills the coordinator. In a multi-node cluster it *could* work (leader upgrades followers via RPC, then transfers leadership and gets upgraded), but the script-level approach is simpler and more debuggable for the VM test.

**Decision**: Use script-level deploy in the VM test. The DeploymentCoordinator's internal logic is already covered by unit tests. The VM test validates the end-to-end flow: binary swap, state persistence across restart, and cluster re-formation.

### 2. Test uses real aspen-constants crate (not cowsay)

Previous dogfood tests used cowsay (tiny flake, fast to build). For multi-node, we still use aspen-constants — it's a real workspace crate, builds fast (~30s), and proves the Nix pipeline works with real Aspen code.

### 3. Three VMs, no separate client

All 3 VMs run `aspen-node`. CLI commands run on node1. No separate client VM — saves RAM and keeps the test focused on the deploy path. The `multi-node-proxy` test already validates client-to-cluster connectivity.

### 4. Deploy verification: version check + KV round-trip

After deploy, verify by:

1. Checking each node's PID changed (new process)
2. KV write on new leader + read from followers (Raft replication works)
3. Cluster health check passes with all 3 nodes

## Risks / Trade-offs

- **[RAM]** 3-node VM test requires ~6GB RAM (2GB per node). The `multi-node-proxy` test already does this, so CI infra can handle it. → Mitigation: Use 1.5GB per node (aspen-node without plugins is lighter).
- **[Flakiness]** Election timeouts and health check races in multi-node tests. → Mitigation: Use generous timeouts (30s for health checks, 60s for cluster formation). Deterministic iroh secret keys for reproducible endpoint IDs.
- **[Build time]** NixOS VM tests are slow to build (~5-10 min). → Mitigation: Reuse `ciVmTestBin`/`ciVmTestCliBin` patterns from existing tests. Share the inner flake pattern from `ci-dogfood-full-loop`.
