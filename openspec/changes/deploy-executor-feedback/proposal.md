## Why

The deploy executor's background task failure is invisible to the coordinator. When `NodeUpgradeExecutor` fails (e.g., binary validation), the coordinator's health poll still sees a healthy node (it never restarted) and marks the deploy successful. This masks real failures — the deploy "succeeds" but nothing was upgraded. Additionally, the binary validation hardcodes `bin/aspen-node`, preventing testing with non-aspen artifacts like cowsay.

## What Changes

- Background executor writes `NodeDeployStatus::Failed(reason)` to KV on error, closing the feedback gap between the spawned task and the coordinator
- Coordinator's health polling checks KV-stored node deploy status alongside GetHealth RPC, detecting pre-restart failures
- Binary validation path in `upgrade_nix()` becomes configurable via `NodeUpgradeConfig.expected_binary` (default: `bin/aspen-node`)
- `NodeUpgrade` RPC carries an optional `expected_binary` field so the deploy request can specify what binary to validate
- Single-node deploy test updated to verify happy path (configurable binary check passes)
- Multi-node deploy test updated to verify sad path (default `bin/aspen-node` check fails for cowsay, coordinator detects failure, cluster stays healthy)

## Capabilities

### New Capabilities

- `deploy-executor-feedback`: Executor failure reporting from background task to KV, coordinator KV-status polling during health checks
- `deploy-configurable-binary`: Configurable binary validation path in NodeUpgradeConfig and the deploy protocol

### Modified Capabilities


## Impact

- `crates/aspen-cluster/src/upgrade/executor.rs` — configurable `expected_binary` in `NodeUpgradeConfig`, `upgrade_nix()` uses it
- `crates/aspen-cluster/src/upgrade/types.rs` — new field on `NodeUpgradeConfig`
- `crates/aspen-cluster-handler/src/handler/deploy.rs` — write Failed status on executor error, pass `expected_binary` from RPC
- `crates/aspen-deploy/src/coordinator/health.rs` — check KV node status during health polling
- `crates/aspen-deploy/src/coordinator/mod.rs` — integrate KV status check into `poll_node_health` flow
- `crates/aspen-client-api/` — `NodeUpgrade` request variant gains optional `expected_binary` field
- `nix/tests/ci-dogfood-deploy.nix` — happy path: passes `bin/cowsay` as expected binary
- `nix/tests/ci-dogfood-deploy-multinode.nix` — sad path: default `bin/aspen-node` check, asserts graceful failure
