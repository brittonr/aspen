## Why

The dogfood script (`scripts/dogfood-local.sh`) has its own 200-line rolling deploy implementation (`do_deploy`) that manually stops nodes, restarts them with the CI-built binary, waits for Raft rejoin, and sweeps peer addresses. Meanwhile, the actual `cluster deploy` RPC path — `DeploymentCoordinator` + `NodeUpgradeExecutor` — does the same thing through proper state-machine-backed Rust code with KV persistence, quorum safety proofs, and drain/health lifecycle. Two code paths for the same operation means bugs get fixed in one but not the other, and the RPC deploy path never gets exercised in the primary dogfood flow.

## What Changes

- Replace the bash rolling deploy logic in `do_deploy` with calls to `aspen-cli cluster deploy` and `aspen-cli cluster deploy-status` polling.
- Remove the manual node stop/restart/rejoin/address-sweep loop from the script.
- Add a `--wait` flag to `aspen-cli cluster deploy` that blocks until the deployment reaches a terminal state (completed/failed/rolled_back), polling `deploy-status` internally.
- Add a `--timeout` flag to cap how long `--wait` blocks.
- Adapt the verify step to work with the RPC-deployed cluster (the CI binary is now installed via Nix profile switch or blob staging, not a manual process restart).
- Keep the script's pre-deploy validation (pipeline success check, artifact extraction) since that's orthogonal to the deploy mechanism.

## Capabilities

### New Capabilities

- `cli-deploy-wait`: `aspen-cli cluster deploy --wait` blocks until deployment completes, with `--timeout` and exit code reflecting success/failure.

### Modified Capabilities

- `ci`: The dogfood script's deploy step changes from manual bash orchestration to CLI-driven deployment. No spec-level CI behavior changes — the `DeploymentCoordinator` already handles the deploy stage via RPC.

## Impact

- `scripts/dogfood-local.sh`: `do_deploy` rewritten from ~200 lines of bash orchestration to ~40 lines of CLI calls.
- `crates/aspen-cli/src/bin/aspen-cli/commands/cluster.rs`: `Deploy` subcommand gains `--wait` and `--timeout` args.
- `crates/aspen-cli/src/bin/aspen-cli/output.rs`: Deploy output may need a streaming/polling display mode.
- NixOS VM tests (`ci-dogfood-deploy.nix`, `ci-dogfood-deploy-multinode.nix`): Should continue to pass since they already use the RPC path. The script-based dogfood tests may need adjustment.
- `nix run .#dogfood-local -- deploy` and `nix run .#dogfood-local -- full-loop`: behavior changes from script-managed to RPC-managed deploy.
