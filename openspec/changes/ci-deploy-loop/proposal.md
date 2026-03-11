## Why

Phase 1 (`manual-rolling-deploy`) gives operators `cluster deploy <artifact>` from the CLI. But the self-hosting goal requires the CI pipeline to drive deployment automatically — push code, CI builds it, CI deploys it, cluster is running the new version. No human in the loop.

This phase wires the deployment machinery into the CI pipeline, updates the dogfood script to exercise the full loop, and adds a VM test that validates everything end to end.

## What Changes

- Add `Deploy` variant to `JobType` in CI config types
- Add deploy job config fields: `artifact_from`, `strategy`, `health_check_timeout_secs`, `max_concurrent`
- Implement `DeployExecutor` that resolves build artifacts and calls `ClusterDeploy` RPC
- Update `.aspen/ci.ncl` with a stage 4 deploy block
- Extend `dogfood-local.sh` with `deploy` and `full-loop` commands
- Add `ci-dogfood-deploy.nix` NixOS VM test for the complete cycle
- Documentation for the full self-hosted pipeline

## Capabilities

### New Capabilities

- `deploy-stage`: CI pipeline deploy stage type. Covers deploy job configuration in `.aspen/ci.ncl`, artifact-to-deployment binding, progress monitoring as CI logs, and deployment trigger from pipeline completion.

### Modified Capabilities

- `ci`: Add `Deploy` variant to `JobType` enum. Deploy jobs reference build artifacts via `artifact_from` and route to the cluster deployment coordinator instead of worker nodes.

## Impact

- **Modified crates**:
  - `aspen-ci-core` — `JobType::Deploy`, `DeployConfig` struct, deploy validation in `JobConfig::validate()`
  - `aspen-ci` — `DeployExecutor` for artifact resolution and deployment monitoring
  - `aspen-rpc-handlers` — deploy executor registration in CI worker dispatch
- **Scripts**: `scripts/dogfood-local.sh` gains `deploy` and `full-loop` commands
- **CI config**: `.aspen/ci.ncl` gains stage 4 deploy block
- **NixOS tests**: New `nix/tests/ci-dogfood-deploy.nix`
- **Docs**: `docs/deploy.md`, updates to `AGENTS.md` and `README.md`
- **Depends on**: `manual-rolling-deploy` (phase 1) must be merged first
