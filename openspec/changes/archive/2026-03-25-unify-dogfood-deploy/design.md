## Context

The dogfood script (`scripts/dogfood-local.sh`) has a `do_deploy` function (~200 lines) that implements rolling deployment in bash: extract artifact from CI pipeline → stop each node → restart with CI binary → wait for Raft rejoin → sweep peer addresses. The Rust codebase has a parallel implementation: `DeploymentCoordinator` (aspen-deploy) orchestrates rolling upgrades via `NodeUpgradeExecutor` (aspen-cluster), with KV-persisted state, quorum safety proofs, and drain lifecycle. The CLI already has `cluster deploy` and `cluster deploy-status` commands. The RPC path handles follower-first ordering, leadership transfer, health polling, and rollback — everything the bash script reimplements.

The script's deploy works for local development, but it diverges from the RPC path: no drain phase, no KV state persistence, no quorum safety checks, and address sweep logic that exists nowhere in the Rust code. Bugs fixed in one path don't get fixed in the other.

## Goals / Non-Goals

**Goals:**

- `do_deploy` calls `aspen-cli cluster deploy --wait` instead of manual node management
- `--wait` flag on CLI deploy polls status and streams per-node transitions
- `--timeout` flag caps wait duration with clear error on expiry
- Remove ~160 lines of bash deploy orchestration (keep artifact extraction, which is pre-deploy)
- The full-loop dogfood path exercises the same deploy code path as production

**Non-Goals:**

- Changing the `DeploymentCoordinator` internals — it already works
- Adding new deploy strategies (canary, blue-green)
- Modifying the NixOS VM deploy tests — they already use the RPC path
- Changing how `do_verify` works (it's orthogonal)

## Decisions

### 1. `--wait` polls from the CLI process, not server-side streaming

The CLI polls `ClusterDeployStatus` every 5 seconds (matching `DEPLOY_STATUS_POLL_INTERVAL_SECS`). Each poll returns the full `DeployStatusResult` including per-node statuses. The CLI diffs against the previous snapshot and prints changes.

Alternative: server-side SSE/streaming via a new RPC. Rejected — Iroh QUIC doesn't have an SSE equivalent, and polling at 5s is fine for deploys that take minutes. The polling approach matches what `DeployMonitor` already does internally.

### 2. Exit codes for `--wait`

- 0: deployment completed
- 1: deployment failed or rolled back
- 2: timeout expired
- Non-wait mode: 0 if accepted, 1 if rejected (unchanged)

This lets the dogfood script use `if ! cli cluster deploy ... --wait; then` directly.

### 3. Artifact extraction stays in bash

The pre-deploy logic (read pipeline status, find job ID, extract store path) stays in the script. It's read-only KV queries that don't need to be in Rust. The script passes the resolved artifact string to `cluster deploy`.

### 4. Remove address sweep from script

The script's post-deploy address sweep (grep iroh logs → parse endpoints → `update-peer`) is a workaround for nodes getting new iroh ports on restart. The `DeploymentCoordinator`'s health polling already handles reconnection: after a node restarts, the coordinator's health RPC attempts use the cluster's gossip-based peer discovery to find the new address. If address discovery fails, the deploy fails — which is the correct behavior (surface the problem rather than papering over it with log parsing).

If address refresh turns out to be needed, it belongs in the `NodeUpgradeExecutor`'s post-restart hook, not in bash.

## Risks / Trade-offs

**[Single-node deploy requires self-restart]** → The `DeploymentCoordinator` sends `NodeUpgrade` RPCs to nodes, but for a single-node cluster the leader must upgrade itself. The current `NodeUpgradeExecutor` handles this via systemd restart or execve. The bash script handled it by just stopping and restarting the process externally. With the RPC path, the node triggers its own restart — if the restart fails, there's no external process to recover it. Mitigation: the dogfood script can check that the node comes back healthy after `--wait` returns, and fall back to manual restart on failure.

**[CLI must handle connection loss during deploy]** → When the node the CLI is connected to restarts (it's being upgraded), the `deploy-status` poll will fail temporarily. The CLI must retry on connection errors, rotating to other nodes in the cluster ticket. This is the same pattern the `AspenClient` already uses for NOT_LEADER rotation.

**[Timeout default must cover nix profile switch]** → `nix-env --set` profile switch can take a few seconds, but the health check timeout (120s default per node) already accounts for this. The overall `--timeout` default of 3600s is generous enough for large clusters.
