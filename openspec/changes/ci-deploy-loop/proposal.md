## Why

Phase 1 (`manual-rolling-deploy`) gives operators `cluster deploy <artifact>` from the CLI. But the self-hosting goal requires the CI pipeline to drive deployment automatically — push code, CI builds it, CI deploys it, cluster is running the new version. No human in the loop.

Three things were explicitly deferred from phase 1:

1. **CI pipeline integration** — after a successful `ci_nix_build` job, automatically trigger `ClusterDeploy` with the built store path
2. **Dogfood script changes** — wire `scripts/dogfood-local.sh` to exercise the full build→deploy loop
3. **NixOS VM integration test** — a VM test that boots a cluster, pushes source to Forge, CI builds it, then deploys the result back to the cluster

This change completes the self-hosted loop: Forge → CI → Build → Deploy.

## What Changes

- Add `Deploy` variant to `JobType` in `aspen-ci-core/src/config/types.rs`
- Add deploy job config fields: `artifact_from`, `strategy`, `health_check_timeout_secs`, `max_concurrent`
- Implement `DeployExecutor` in `aspen-ci` that resolves build artifacts from KV and calls `ClusterDeploy` RPC
- Wire `DeployExecutor` into the pipeline orchestrator dispatch (runs on leader, not via worker queue)
- Update `.aspen/ci.ncl` with a deploy stage
- Extend `scripts/dogfood-local.sh` with `deploy` and `full-loop` commands
- Add `nix/tests/ci-dogfood-deploy.nix` NixOS VM test for the complete cycle
- Documentation for the full self-hosted pipeline

## Capabilities

### New Capabilities

- `deploy-stage`: CI pipeline deploy job type. Covers deploy job configuration in `.aspen/ci.ncl`, artifact-to-deployment binding via `artifact_from`, progress monitoring as CI logs, and deployment trigger from successful build.

### Modified Capabilities

- `ci`: Add `Deploy` variant to `JobType` enum. Deploy jobs reference build artifacts via `artifact_from` and route to the deployment coordinator instead of the worker pool.

## Impact

- **Modified crates**:
  - `aspen-ci-core` — `JobType::Deploy`, deploy fields on `JobConfig`, validation
  - `aspen-ci` — `DeployExecutor` for artifact resolution and deployment monitoring, registered in `PipelineOrchestrator` dispatch
  - `aspen-rpc-handlers` — potentially: deploy executor registration if dispatch lives here
- **Scripts**: `scripts/dogfood-local.sh` gains `deploy` and `full-loop` commands
- **CI config**: `.aspen/ci.ncl` gains a deploy stage (commented out by default)
- **NixOS tests**: New `nix/tests/ci-dogfood-deploy.nix`
- **Docs**: `docs/deploy.md`, updates to `AGENTS.md`
- **Depends on**: `manual-rolling-deploy` (phase 1) must be merged first
