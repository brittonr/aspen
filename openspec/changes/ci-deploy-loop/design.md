## Context

Phase 1 (`manual-rolling-deploy`) provides the `ClusterDeploy`, `ClusterDeployStatus`, and `ClusterRollback` RPCs plus the node-level upgrade machinery. This phase connects those RPCs to the CI pipeline so deployments happen automatically after a successful build+test cycle.

The existing CI executor architecture has `ShellExecutor` and `NixBuildWorker`. Deploy needs a third executor type that doesn't run a build — it resolves an artifact from a previous build job and calls the deployment RPC.

## Goals / Non-Goals

**Goals:**

- CI pipeline can include a `type = 'deploy` job that triggers rolling deployment
- Deploy jobs resolve artifacts from build job results automatically
- Dogfood script runs the complete push → build → test → deploy → verify loop
- NixOS VM test validates the full cycle in isolation

**Non-Goals:**

- Deployment approval gates (manual approval before deploy — future work)
- Deploy-on-tag-only (trigger filtering is already in TriggerConfig, just needs config)
- Multi-environment deploys (staging → production — future work)
- Deploy notifications (Slack, email — future work)

## Decisions

### 1. DeployExecutor as a CI executor, not a worker

Deploy jobs don't run on worker nodes — they run on the leader where the `DeploymentCoordinator` lives. The `DeployExecutor` is registered directly in the CI pipeline orchestrator's dispatch, not in the worker pool. When the orchestrator encounters a `JobType::Deploy`, it runs the executor in-process on the leader.

**Alternative**: Route deploy jobs through the job queue to a worker that calls `ClusterDeploy` RPC. Rejected — adds an unnecessary hop. The orchestrator already runs on the leader.

### 2. Artifact resolution via job result KV lookup

The deploy job config has `artifact_from = "build-node"`. The executor:

1. Finds the job ID for "build-node" in the current pipeline run
2. Reads `__jobs:{job_id}` from KV
3. Extracts `result.Success.data.output_paths[0]` (Nix store path) or `result.Success.data.artifacts[].blob_hash` (blob)
4. Passes the artifact to `ClusterDeploy` RPC

This reuses the existing job result storage — no new artifact registry.

### 3. Progress as CI job logs

The executor polls `ClusterDeployStatus` every 5 seconds and emits per-node status changes as CI job log lines:

```
[deploy] Starting rolling deployment (3 nodes)
[deploy] Node 2: draining...
[deploy] Node 2: restarting
[deploy] Node 2: healthy ✓
[deploy] Node 3: draining...
...
[deploy] Node 1 (leader): draining...
[deploy] Deployment completed successfully
```

This gives operators visibility in the CI log stream without needing a separate monitoring channel.

### 4. Dogfood script structure

`dogfood-local.sh` gains:

- `do_deploy`: calls `cli cluster deploy` with the CI-built artifact, polls status, prints progress
- `do_full_loop`: `start → push → build → deploy → verify` (verify checks running node version matches)
- Updates `do_verify` to check `cluster status` reports the new git hash

The existing `full` command stays as-is (build only). `full-loop` is the new end-to-end command.

## Risks / Trade-offs

**[Risk: Deploy stage runs before binary cache propagation]** → The build job's `publish_to_cache = true` uploads store paths. But cache propagation may lag. Mitigation: the deploy executor waits up to 60s for the store path to appear in the cache before giving up.

**[Risk: VM test flakiness from timing]** → Multi-node deploy involves restart windows. The VM test must use generous timeouts (180s health checks, 600s total). Use `retry_count = 1` on the deploy job for one retry.

**[Trade-off: No approval gate]** → The pipeline deploys immediately after test passes. This is fine for the dogfood use case (single operator, development cluster). Production clusters should add a `when: "refs/tags/release/*"` filter on the deploy stage.

## Open Questions

1. Should the deploy stage be optional in the default `.aspen/ci.ncl`? Leaning yes — commented out by default, operator uncomments when ready.
2. Should the VM test do a full workspace build or use a minimal flake? Full workspace takes 15+ minutes — maybe use a pre-built binary injected into the VM.
