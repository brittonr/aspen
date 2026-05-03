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

Use `aspen-cli ci logs --follow <run_id> <deploy_job_id>` to watch in real time.

## Dogfood Script

The self-hosted build pipeline is Aspen's operator acceptance path: source is pushed to Aspen Forge, built by Aspen CI, deployed to an Aspen cluster, verified through cluster health, and then stopped with a durable run receipt. Treat a successful `full` run plus `receipts list/show` output as the canonical evidence that the self-hosted path works under the current tree.

```bash
# Full pipeline: start → push → build → deploy → verify → stop
nix run .#dogfood-local -- full

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

# Emit the validated canonical JSON receipt for machine checks or evidence archival.
nix run .#dogfood-local -- --cluster-dir /tmp/aspen-dogfood receipts show <run-id> --json
```

A successful acceptance receipt has schema `aspen.dogfood.run-receipt.v1`, command `full`, final status `succeeded`, no failure object, and succeeded stages for `start`, `push`, `build`, `deploy`, `verify`, and `stop`. Failure receipts are also persisted so the same commands can be used during incident review without relying on scrollback.

## Troubleshooting

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
