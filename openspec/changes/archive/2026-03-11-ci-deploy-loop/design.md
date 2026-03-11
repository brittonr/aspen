## Context

Phase 1 (`manual-rolling-deploy`) provides `ClusterDeploy`, `ClusterDeployStatus`, and `ClusterRollback` RPCs, the `DeploymentCoordinator` on the Raft leader, node-level binary swap (Nix profile switch or blob-based), graceful drain, health-gated progression, and quorum safety invariants. The CLI has `cluster deploy <artifact>`, `cluster deploy-status`, and `cluster rollback` with progress polling.

The CI pipeline already produces Nix store paths as build artifacts. Job results are stored in KV at `__jobs:{job_id}` with `result.Success.data.output_paths` containing the built store paths. The `PipelineOrchestrator` in `aspen-ci/src/orchestrator/pipeline/` coordinates stage execution and status sync. The trigger system in `aspen-ci/src/trigger/` auto-starts pipelines on git push via forge gossip.

This phase connects deployment to the CI pipeline. A `type = 'deploy` job references a build job's artifacts and calls `ClusterDeploy`.

## Goals / Non-Goals

**Goals:**

- CI pipeline can include a `type = 'deploy` job that triggers rolling deployment after build
- Deploy jobs resolve artifacts from referenced build job results automatically
- Dogfood script runs the full push → build → deploy → verify loop
- NixOS VM test validates the full cycle in isolation

**Non-Goals:**

- Deployment approval gates (manual approval before deploy — future)
- Multi-environment deploys (staging → production — future)
- Wire format version negotiation between old and new binaries (acknowledged risk, not solved)
- Deploy notifications (Slack/webhook — future)

## Decisions

### 1. DeployExecutor runs on the leader, not via worker queue

Deploy jobs don't run on worker nodes — they run on the leader where the `DeploymentCoordinator` lives. When the `PipelineOrchestrator` encounters a `JobType::Deploy`, it runs the `DeployExecutor` in-process instead of enqueueing to the job queue.

This avoids an unnecessary hop: the orchestrator already runs on the leader, and `ClusterDeploy` must be called on the leader. Routing through the worker queue would mean a worker calls an RPC back to the leader.

**Alternative**: Route deploy jobs through the job queue to a `DeployWorker`. Rejected — adds latency and complexity for no benefit. The leader already has the `DeploymentCoordinator`.

### 2. Artifact resolution via job result KV lookup

The deploy job config has `artifact_from = "build-node"` (the name of a build job in a preceding stage). The `DeployExecutor`:

1. Finds the job ID for `"build-node"` in the current pipeline run's stage/job metadata
2. Reads `__jobs:{job_id}` from KV
3. Extracts `result.Success.data.output_paths[0]` as a `DeployArtifact::NixStorePath`
4. Falls back to `result.Success.data.artifacts[].blob_hash` as `DeployArtifact::BlobHash`
5. Fails with `NO_ARTIFACTS_FOUND` if neither exists

This reuses existing job result storage — no new artifact registry needed.

### 3. Progress emitted as CI job logs

The `DeployExecutor` polls `ClusterDeployStatus` every `DEPLOY_STATUS_POLL_INTERVAL_SECS` (5s) and emits per-node status changes as job log lines:

```
[deploy] Starting rolling deployment: /nix/store/...-aspen-node (3 nodes)
[deploy] Node 2: draining
[deploy] Node 2: upgrading
[deploy] Node 2: healthy ✓
[deploy] Node 3: draining
[deploy] Node 3: healthy ✓
[deploy] Node 1 (leader): draining
[deploy] Deployment completed (42s)
```

Log lines flow through the existing CI log infrastructure (`_ci:logs:{run_id}:{job_id}:{chunk}`), so `ci logs --follow` works for deploy jobs the same way it works for build jobs.

### 4. Validation: artifact_from must reference a prior stage job

`JobConfig::validate()` gains a new check: if `job_type == Deploy`, then `artifact_from` must be `Some(name)` where `name` matches a job name in a preceding stage. This is a static check at config load time — doesn't wait for runtime.

Cross-stage reference validation needs the full `PipelineConfig` context, so this check lives in `PipelineConfig::validate()`, not `JobConfig::validate()`.

### 5. Dogfood script: deploy + full-loop commands

`scripts/dogfood-local.sh` gains:

- `do_deploy`: reads the pipeline run ID from `$CLUSTER_DIR/run_id`, extracts the CI-built artifact (Nix store path from job result), calls `cli cluster deploy <store-path>`, polls `cli cluster deploy-status` with progress output
- `do_full_loop`: `start → push → build → deploy → verify` (the complete self-hosted cycle)
- `do_verify` update: additionally checks `cluster status` reports the expected version

The existing `full` command stays as-is (build + verify only). `full-loop` adds deployment.

### 6. VM test: single-node, pre-built binary, generous timeouts

The NixOS VM test (`ci-dogfood-deploy.nix`) uses a **single-node** cluster, not 3-node. Reasons:

- A single-node cluster still exercises the full deploy path (drain → swap → restart → health)
- Multi-node tests need 3× the RAM and add flakiness from quorum timing during leader restart
- The quorum safety invariant is already unit-tested in `aspen-deploy`

The test pushes a minimal flake that produces a versioned wrapper binary (not the full Aspen build). This keeps the nix build under 60s and avoids the 15+ minute full workspace compile.

Key VM test configuration (from napkin lessons):

- `writableStoreUseTmpfs = false` — nix builds download large deps that overflow tmpfs
- `diskSize = 20480` — enough for nix store with build deps
- `memorySize = 4096` — hyperlight/redb need headroom
- Use absolute paths in `systemd-run` (NixOS transient units don't inherit `environment.systemPackages` PATH)
- Use `pkgs.writeText` for multi-line config files (not bash heredocs in Python `succeed()`)
- Poll with `wait_until_succeeds` + generous timeout, not tight loops
- Probe for binary with `test -x` before assuming output_paths\[0\] is the binary (skip -man, -doc outputs)

## Risks / Trade-offs

**[Risk: Wire format incompatibility between old and new binary]** → Phase 1 design doc explicitly calls this out as a non-goal. Postcard discriminant stability is a known concern (napkin has 5+ incidents). The deploy machinery doesn't solve this — operators must ensure the new binary is wire-compatible. Version negotiation is future work.

**[Risk: Store path not available on target node]** → The build job runs `nix build` on the leader node, so the store path exists there. For multi-node clusters, target nodes need to fetch from cache or have the store path already. If fetch fails, the per-node upgrade fails and deployment halts (phase 1 behavior). The deploy executor logs which path it's deploying so the operator can debug.

**[Risk: Deploy stage runs before build artifacts are fully written]** → The `PipelineOrchestrator` enforces stage ordering — deploy stage only starts after all preceding stages complete. The job result KV write happens before the job is marked complete, so the artifact is always readable when the deploy executor runs.

**[Risk: VM test flakiness from nix store disk space]** → `writableStoreUseTmpfs = false` + `diskSize = 20480` (20GB) prevents the "No space left on device" failure that killed earlier tests. The minimal test flake (wrapping cowsay) only needs ~200MB of deps.

**[Trade-off: No approval gate]** → The pipeline deploys immediately after tests pass. For the dogfood use case (single operator, dev cluster) this is correct. Production clusters should filter the deploy stage to tagged releases: `when = "refs/tags/release/*"`.

**[Trade-off: Single-node VM test]** → Doesn't test leader failover during deploy or quorum safety under real restarts. Those properties are verified by unit tests and the quorum safety Verus proofs. A multi-node deploy VM test can be added later.

## Open Questions

1. Should the deploy stage be commented out in the default `.aspen/ci.ncl`? Leaning yes — operators opt in when their cluster is ready.
2. Should the `DeployExecutor` wait for the store path to appear in the binary cache before calling `ClusterDeploy`? Probably not for single-node — the path is local. Matters for multi-node, but cache propagation is an existing concern, not specific to this change.
