# Deployment

Aspen supports automated rolling deployments of cluster binaries, driven by CI pipelines or manual CLI commands.

## Architecture

```
git push → Forge → CI auto-trigger → nix build → DeployExecutor → ClusterDeploy → node upgrade
                                                       │
                                                       ├── resolve artifact from build job KV result
                                                       ├── call ClusterDeploy RPC on leader
                                                       └── poll ClusterDeployStatus, emit CI logs
```

The deploy executor runs in-process on the Raft leader, not through the worker queue. The `DeploymentCoordinator` (from `aspen-deploy`) handles the actual node-by-node upgrade with drain, binary swap, restart, and health gating.

## CI Pipeline Configuration

Add a deploy stage to `.aspen/ci.ncl`:

```nickel
{
  name = "my-project",
  stages = [
    {
      name = "build",
      jobs = [
        {
          name = "build-node",
          type = 'nix,
          flake_url = ".",
          flake_attr = "packages.x86_64-linux.aspen-node",
          publish_to_cache = true,
        },
      ],
    },
    {
      name = "test",
      depends_on = ["build"],
      jobs = [
        {
          name = "run-tests",
          type = 'nix,
          flake_url = ".",
          flake_attr = "checks.x86_64-linux.nextest-quick",
        },
      ],
    },
    {
      name = "deploy",
      depends_on = ["test"],
      jobs = [
        {
          name = "deploy-node",
          type = 'deploy,
          artifact_from = "build-node",
          strategy = "rolling",
          max_concurrent = 1,
          health_check_timeout_secs = 120,
          timeout_secs = 600,
        },
      ],
    },
  ],
}
```

### Deploy Job Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `artifact_from` | string | *required* | Name of a build job in a preceding stage |
| `strategy` | string | `"rolling"` | Deployment strategy |
| `max_concurrent` | number | `1` | Max nodes to upgrade simultaneously |
| `health_check_timeout_secs` | number | `120` | Per-node health check timeout |

### Validation Rules

- `artifact_from` is required for deploy jobs
- The referenced job must exist in a preceding stage (not the same stage)
- Deploy stages run after all preceding stages complete

## Manual Deployment via CLI

```bash
# Deploy a specific Nix store path
aspen-cli cluster deploy /nix/store/...-aspen-node

# Check deployment status
aspen-cli cluster deploy-status

# Roll back if needed
aspen-cli cluster rollback
```

## Artifact Resolution

The deploy executor resolves build artifacts in this priority order:

1. `result.Success.data.output_paths[0]` — Nix store path (preferred)
2. `result.Success.data.artifacts[].blob_hash` — iroh blob hash
3. `result.Success.data.uploaded_store_paths[].blob_hash` — blob hash fallback

The artifact is read from KV at `__jobs:{job_id}` where the job ID comes from the pipeline run's stage metadata.

## Safety Guarantees

- **Quorum safety**: The coordinator will not upgrade a node if doing so would break Raft quorum
- **Health gating**: Each node must pass health checks before the next node is upgraded
- **Drain**: Active connections are drained before upgrade begins
- **Rollback**: `cluster rollback` reverts to the previous binary if needed

## Deployment Progress in CI Logs

Deploy jobs emit per-node progress as CI log lines with the `[deploy]` prefix:

```
[deploy] Resolving artifact from job 'build-node'...
[deploy] Starting rolling deployment: /nix/store/...-aspen-node (3 nodes)
[deploy] Node 2: draining
[deploy] Node 2: upgrading
[deploy] Node 2: healthy ✓
[deploy] Node 3: draining
[deploy] Node 3: healthy ✓
[deploy] Node 1 (leader): draining
[deploy] Deployment completed (42s)
```

Use `aspen-cli ci logs --follow <run_id> <deploy_job_id>` to watch in real time. Log following stops only when the shared CI log completion marker (`CI_LOG_COMPLETE_MARKER`) appears in the `_ci:logs:<run_id>:<job_id>:` KV stream, or when polling observes `CiGetJobLogsResponse.is_complete`. That marker means the log stream is closed; it is intentionally separate from CI job/pipeline result labels such as `success`, `failed`, or `cancelled`, which come from `ci status` and `ci receipt`.

## Native CI Run Receipts

Aspen exposes the persisted CI run as a schema-versioned operator receipt. This is the native CI/deploy evidence plane behind dogfood receipts: it is projected from the Raft-backed `PipelineRun` record and keeps job IDs as handles for follow-up `ci logs` / `ci output` inspection.

```bash
# Human summary for a CI run.
aspen-cli ci receipt <run-id>

# Machine-parseable receipt JSON.
aspen-cli --json ci receipt <run-id>
```

The receipt schema is `aspen.ci.run-receipt.v1`. It includes run ID, repository ID, ref, commit hash, pipeline name, terminal/current status, timestamps, stages, and deterministically ordered jobs. The stable CI status labels exposed by RPC responses, receipts, CLI filters, and `ci status --follow` are `initializing`, `checking_out`, `checkout_failed`, `pending`, `running`, `success`, `failed`, and `cancelled`; terminal labels are `checkout_failed`, `success`, `failed`, and `cancelled`. Missing run IDs fail explicitly instead of fabricating evidence. This first slice is a projection of existing run state rather than a separate immutable receipt row.

## Dogfood Script

The self-hosted build pipeline is Aspen's operator acceptance path: source is pushed to Aspen Forge, built by Aspen CI, deployed to an Aspen cluster, verified through cluster health, and then stopped with a durable run receipt. Treat a successful `full` run plus `receipts list/show` output as the canonical evidence that the self-hosted path works under the current tree.

```bash
# Full pipeline: start → push → build → deploy → verify → publish receipt → stop
nix run .#dogfood-local -- full

# Full pipeline, but leave the verified cluster running after receipt publication
# so cluster-backed evidence can be inspected before cleanup.
nix run .#dogfood-local -- full --leave-running

# Build → deploy → verify (cluster already running)
nix run .#dogfood-local -- full-loop

# Deploy only (after a successful build)
nix run .#dogfood-local -- deploy
```

Each `full` run writes a JSON receipt under the sibling receipts directory for the cluster directory, for example `/tmp/aspen-dogfood-receipts/` when using the default `/tmp/aspen-dogfood` cluster directory. Operators should inspect receipts through the dogfood CLI instead of manually opening `/tmp` JSON:

```bash
# List validated receipts, newest first. Missing receipt directories are shown as empty history.
nix run .#dogfood-local -- --cluster-dir /tmp/aspen-dogfood receipts list

# Show a human-readable summary for a run ID from the list.
nix run .#dogfood-local -- --cluster-dir /tmp/aspen-dogfood receipts show <run-id>

# Diagnose the first failed stage and print first-response checks.
nix run .#dogfood-local -- --cluster-dir /tmp/aspen-dogfood receipts diagnose <run-id>

# Emit the validated canonical JSON receipt for machine checks or evidence archival.
nix run .#dogfood-local -- --cluster-dir /tmp/aspen-dogfood receipts show <run-id> --json

# While the dogfood cluster is still running, read back a receipt from Aspen's
# own Raft-backed KV path. A normal successful `full` run auto-publishes before
# cleanup; use `full --leave-running` when you want this readback after `full` exits.
nix run .#dogfood-local -- --cluster-dir /tmp/aspen-dogfood receipts cluster-show <run-id> --json

# Manual publication remains available for a validated local receipt while a
# matching dogfood cluster is running.
nix run .#dogfood-local -- --cluster-dir /tmp/aspen-dogfood receipts publish <run-id>
```

A successful acceptance receipt follows the test-visible `DOGFOOD_FULL_STAGE_CONTRACTS` contract in `crates/aspen-dogfood/src/receipt.rs`. The stage contract is the source of truth for the required stage names in docs and tests:

| Stage | Default `full` | `full --leave-running` | Evidence role |
| --- | --- | --- | --- |
| `start` | required | required | Start the local dogfood cluster and persist its state file. |
| `push` | required | required | Push the current source into Aspen Forge. |
| `build` | required | required | Run native Aspen CI and record the CI run artifact. |
| `deploy` | required | required | Deploy the built artifact through Aspen's deploy path. |
| `verify` | required | required | Verify the deployed cluster health. |
| `publish_receipt` | required | required | Publish the final success receipt into Aspen KV. |
| `stop` | required | omitted | Stop and remove the ephemeral cluster; intentionally omitted when the operator asks to leave the cluster running for live readback. |

A successful default acceptance receipt has schema `aspen.dogfood.run-receipt.v1`, command `full`, aggregate final status `succeeded`, no failure object, and every stage above marked `required` for default `full`. A successful `full --leave-running` receipt omits only stages marked `omitted` for `full --leave-running`; run `nix run .#dogfood-local -- --cluster-dir /tmp/aspen-dogfood stop` after inspection. Failure receipts are also persisted so the same commands can be used during incident review without relying on scrollback; in `receipts list`, any failed stage makes the aggregate final status `failed` even if cleanup later records a successful `stop` stage. In-cluster publication stores the same canonical receipt JSON at `dogfood/receipts/<run-id>.json`; it is live cluster evidence and does not replace the local receipt archive for ephemeral `full` runs that stop and remove the cluster.

## Troubleshooting

### Interpreting failed dogfood receipts

Use receipts first when a `full` dogfood run fails. They preserve the failed stage, normalized category, and error message even after terminal scrollback is gone:

```bash
# Find the newest failed run.
nix run .#dogfood-local -- --cluster-dir /tmp/aspen-dogfood receipts list

# Read the operator summary.
nix run .#dogfood-local -- --cluster-dir /tmp/aspen-dogfood receipts show <run-id>

# Print deterministic first-response guidance for the failed stage/category.
nix run .#dogfood-local -- --cluster-dir /tmp/aspen-dogfood receipts diagnose <run-id>

# Export validated JSON for incident evidence or scripts.
nix run .#dogfood-local -- --cluster-dir /tmp/aspen-dogfood receipts show <run-id> --json > dogfood-receipt.json
```

Read the first stage whose status is `failed`; later stages are either absent or skipped because the orchestrator stops at the first blocking failure. The stage gives the subsystem boundary, and `failure.category` gives the first triage path:

| Failed stage | Common categories | First checks |
| --- | --- | --- |
| `start` | `process_spawn`, `node_crash`, `health_check`, `state_file` | Check that the dogfood binaries exist, inspect node logs under the cluster directory, and verify the cluster can elect a healthy node. Do not copy or preserve `cluster-ticket.txt`; it is credential material. |
| `push` | `git_push`, `forge`, `client_rpc` | Inspect the receipt message for captured git stdout/stderr, then check Forge RPC reachability and whether the pushed tree exceeded protocol/message limits. |
| `build` | `ci_pipeline`, `timeout`, `client_rpc` | Use the CI run/artifact fields from the receipt, then inspect `aspen-cli ci status`/logs for the exact run ID. If the category is `timeout`, distinguish a still-running local Nix build from a stuck worker before changing source. |
| `deploy` | `deploy_failed`, `client_rpc`, `timeout` | Check deployment status and whether the receipt's CI run artifact is the one being deployed. Confirm the target node still has quorum and the built store path/artifact is available. |
| `verify` | `health_check`, `client_rpc` | Treat this as a post-deploy health failure: inspect cluster health, node uptime, and recent node logs before rerunning. |
| `publish_receipt` | `receipt`, `client_rpc`, `state_file` | The workload verified but self-hosted evidence publication failed. Use the local receipt as fallback evidence, confirm the cluster was still running, and rerun with `full --leave-running` if cluster-backed readback is required. |
| `stop` | `stop_node`, `state_file` | The acceptance result is incomplete because cleanup failed. Inspect local process state and cluster directory ownership before deleting files manually. |

For categories not listed above, use the receipt message as the primary clue and preserve the JSON receipt as evidence. If a run emitted public relay or DNS warnings but the receipt is `succeeded`, those warnings are not acceptance blockers; only a failed receipt should drive incident triage.

### Wire format incompatibility

Postcard serialization uses enum discriminant positions. If the new binary adds or reorders enum variants in `ClientRpcRequest`/`ClientRpcResponse`, existing nodes will misinterpret messages. There is no version negotiation — ensure wire compatibility before deploying.

### Store path not available on target node

For multi-node clusters, the Nix store path must be available on each node before it can be upgraded. Options:

- Build on all nodes (same flake, deterministic)
- Publish to the Aspen binary cache (`publish_to_cache = true`)
- Use `nix copy` to transfer store paths between nodes

If a node can't find the store path, its upgrade fails and the deployment halts.

### Deploy stage rejected

If `ClusterDeploy` returns "rejected", check:

- Another deployment may already be in progress
- The cluster may not have the `deploy` feature enabled
- The node may not be the Raft leader

### VM test resources

When running deploy tests in NixOS VMs:

- Use `writableStoreUseTmpfs = false` (nix builds need real disk)
- Set `diskSize = 20480` (20GB for Nix store)
- Set `memorySize = 4096` (redb + nix build headroom)
- Disable Nix sandbox (`nix.settings.sandbox = false`)
