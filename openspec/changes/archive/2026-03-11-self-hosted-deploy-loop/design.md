## Context

Aspen's dogfood pipeline currently runs the full Forge → CI → Nix build loop and produces a verified binary. The `do_verify` step in `dogfood-local.sh` downloads the CI-built `aspen-node` from the blob store and compares SHA-256 hashes with the local binary. But the cluster keeps running the old binary. The missing piece is a deployment stage that takes the CI output and rolls it across cluster nodes, replacing the running process.

The cluster already has the building blocks: graceful shutdown via `shutdown_signal()` and `node_mode.shutdown()`, the `Supervisor` with restart/backoff logic in `aspen-raft`, distributed coordination primitives (locks, leader election, queues) in `aspen-coordination`, and the job/worker framework in `aspen-jobs`. What's missing is the orchestration that ties these together into a quorum-safe rolling upgrade.

On NixOS nodes, the natural upgrade path is `nix profile install` or `nix-env --set` to atomically switch the binary, then restart the systemd service. For non-NixOS (bare binary), the node writes the new binary to a staging path, validates it (`--version` + hash check), replaces the old binary, and re-execs. Both paths need the same cluster-level orchestration to maintain quorum.

## Goals / Non-Goals

**Goals:**

- A node can replace its own binary and restart without losing cluster membership
- Rolling upgrades across a cluster maintain Raft quorum at all times
- The deploy stage integrates into the existing CI pipeline as a `type = 'deploy` job
- Automatic rollback if a newly upgraded node fails health checks
- `dogfood-local.sh` exercises the complete build → deploy → verify loop
- A NixOS VM test validates the end-to-end cycle

**Non-Goals:**

- Blue-green or canary deployments with traffic splitting (future work)
- Cross-cluster federation deployment (deploy is single-cluster only)
- Deploying non-node binaries (CLI, TUI, git-remote-aspen) — those don't run persistently
- Windows/macOS support — Nix profile switch and `execve` are Linux-only
- Automatic version compatibility checks between old and new binaries (operator responsibility)
- Zero-downtime for a single node — each node has a brief restart window

## Decisions

### 1. Nix profile switch as primary upgrade mechanism

On NixOS (and any system with Nix), use `nix build` output path directly: the CI job already produces a Nix store path. The node runs `nix-env --profile /nix/var/nix/profiles/aspen-node --set <store-path>` to atomically switch, then signals itself to restart via systemd (`systemctl restart aspen-node`).

**Alternative considered**: `execve` to replace the running process in-place. Rejected because it bypasses systemd supervision, loses journal correlation, and complicates rollback. The systemd approach gets restart tracking, watchdog integration, and `systemctl rollback` for free.

For bare-binary deployments (no Nix), the node downloads the blob, writes to a staging path, validates (`sha256sum` + `--version`), atomically renames over the old binary, and the supervisor restarts the process. This is the fallback path.

### 2. Leader-coordinated rolling deployment

The Raft leader orchestrates deployments. A `DeploymentCoordinator` runs as a state machine in KV:

```
_sys:deploy:current → { deploy_id, artifact_hash, target_version, strategy, status, nodes: [...] }
_sys:deploy:history:{timestamp} → completed deployment records
```

The leader picks one node at a time (or up to `(N-1)/2` for large clusters), sends `NodeUpgrade` RPCs, waits for health confirmation, then proceeds. If any node fails health after upgrade, the coordinator halts and marks the deployment as `failed`, leaving the remaining nodes on the old version.

**Alternative considered**: Peer-to-peer upgrade where each node independently watches for new artifacts and upgrades itself. Rejected because it can't guarantee quorum safety — if multiple nodes restart simultaneously, the cluster loses consensus.

### 3. Drain-before-upgrade to preserve in-flight operations

Before upgrading, the node enters a "draining" state:

1. Stop accepting new client RPC connections (remove from ALPN accept)
2. Stop accepting new Raft proposals (respond with `NOT_LEADER` to writes)
3. Wait for in-flight operations to complete (bounded by `DRAIN_TIMEOUT_SECS = 30`)
4. Continue serving Raft replication (so other nodes can still replicate)
5. After drain completes, proceed with binary replacement and restart

This mirrors the existing `WorkerService::shutdown()` pattern: signal → drain → cleanup.

### 4. Deploy job type in CI pipeline

A new `type = 'deploy` job type in `.aspen/ci.ncl` references the build stage's artifacts:

```nickel
{
  name = "deploy",
  depends_on = ["build", "test"],
  jobs = [{
    name = "rolling-deploy",
    type = 'deploy,
    artifact_from = "build-node",
    strategy = 'rolling,
    health_check_timeout_secs = 60,
    max_concurrent = 1,
  }],
}
```

The CI executor for deploy jobs calls `ClusterDeploy` RPC with the artifact blob hash from the referenced build job. The deploy coordinator runs the rolling upgrade.

### 5. Health checks gate upgrade progression

After each node restarts, the coordinator waits for:

1. Node re-joins the Raft cluster (appears in membership)
2. Node responds to `GetHealth` RPC with `healthy` status
3. Node's Raft log catches up (log gap < 100 entries)

All three must pass within `DEPLOY_HEALTH_TIMEOUT_SECS` (default 120s). Failure on any node halts the deployment. The operator can then `cluster rollback` to revert upgraded nodes or `cluster deploy-resume` to retry.

### 6. Deployment state machine

```
Pending → Deploying(node_1) → Deploying(node_2) → ... → Completed
                ↓                    ↓
            RollingBack          RollingBack → Failed
```

States are stored in KV under `_sys:deploy:current` with CAS for consistency. Each node transition is a CAS write so concurrent coordinator failover (leader change mid-deploy) picks up exactly where the previous leader left off.

### 7. Artifact resolution via blob store

The deploy coordinator resolves the binary artifact from the CI job's result data:

1. Read `__jobs:{job_id}` from KV to get `result.Success.data.output_paths[0]`
2. The Nix store path is available locally on the build worker — for other nodes, fetch via `nix copy --from` using the cluster's binary cache substituter
3. For non-Nix: read `result.Success.data.artifacts[].blob_hash`, fetch blob to staging path

This reuses the existing CI artifact storage — no new storage mechanism needed.

## Risks / Trade-offs

**[Risk: Leader failover during deployment]** → The deployment state is in KV (linearizable). If the leader crashes mid-deploy, the new leader reads `_sys:deploy:current`, sees which nodes have been upgraded, and resumes. CAS ensures no duplicate upgrades.

**[Risk: Upgraded node can't rejoin cluster]** → Health check timeout catches this. The coordinator marks the deployment as failed. The node's old binary is preserved (Nix keeps previous generations; bare binary has `.bak` copy). Rollback re-installs the previous version.

**[Risk: Wire format incompatibility between old and new binary]** → Not solved in this change. Postcard discriminant stability is already a known concern (see napkin). Operators should test binary compatibility before deploying. Future work: add a version negotiation handshake.

**[Risk: Nix store path not available on target node]** → The binary cache substituter handles this. CI jobs with `publish_to_cache = true` push store paths to the cluster cache. Target nodes fetch from cache during `nix-env --set`. If cache miss, the deploy falls back to blob fetch.

**[Trade-off: One node at a time is slow for large clusters]** → Safety over speed. `max_concurrent` is configurable, defaulting to 1. For a 5-node cluster, `max_concurrent = 2` is safe (keeps 3 nodes running). The formula is: `max_concurrent <= (N - 1) / 2` where N is voter count.

**[Trade-off: Systemd dependency for restart]** → Not all deployments use systemd. The `NodeUpgrade` handler detects the init system and falls back to `execve` + parent PID signal for non-systemd environments.

## Open Questions

1. Should rollback be automatic on first failure, or require operator confirmation? Current design: halt and wait for operator.
2. Should deploys be gated on passing the full test suite in CI, or is the build artifact sufficient? Leaning toward: require the test stage to pass.
3. Do we need a `cluster deploy --dry-run` that shows which nodes would be upgraded without actually doing it?
