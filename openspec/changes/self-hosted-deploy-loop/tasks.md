## 1. Foundation: Types, Constants, and RPC Messages

- [x] 1.1 Add `Deploy` variant to `JobType` enum in `aspen-ci-core/src/config/types.rs` with deploy-specific fields (`artifact_from`, `strategy`, `health_check_timeout_secs`, `max_concurrent`)
- [x] 1.2 Add `DeployConfig` struct to `aspen-ci-core/src/config/types.rs` with `strategy` enum (`Rolling`), `max_concurrent` (default 1), `health_check_timeout_secs` (default 120)
- [x] 1.3 Add deploy-related constants to `aspen-constants`: `DRAIN_TIMEOUT_SECS` (30), `DEPLOY_HEALTH_TIMEOUT_SECS` (120), `MAX_DEPLOY_HISTORY` (50), `DEPLOY_STATUS_POLL_INTERVAL_SECS` (5)
- [x] 1.4 Add `ClusterDeploy`, `ClusterDeployStatus`, `ClusterRollback`, `NodeUpgrade`, `NodeRollback` request variants to `ClientRpcRequest` in `aspen-client-api`
- [x] 1.5 Add corresponding response variants (`ClusterDeployResult`, `ClusterDeployStatusResult`, `ClusterRollbackResult`, `NodeUpgradeResult`, `NodeRollbackResult`) to `ClientRpcResponse`
- [x] 1.6 Add `to_operation()` and `domain()` implementations for the new RPC variants
- [x] 1.7 Add postcard discriminant golden tests for the new variants to prevent wire format regressions

## 2. Node-Level Upgrade (`aspen-cluster`)

- [x] 2.1 Create `src/upgrade/mod.rs` in `aspen-cluster` with `NodeUpgradeExecutor` struct
- [x] 2.2 Implement graceful drain: stop accepting client RPCs, return `NOT_LEADER` for writes, wait for in-flight ops with bounded timeout
- [x] 2.3 Implement Nix profile switch path: `nix-env --profile <path> --set <store-path>`, verify binary exists, detect systemd and issue `systemctl restart`
- [x] 2.4 Implement blob-based binary swap path: download blob to staging, SHA-256 validate, `--version` smoke test, atomic rename, preserve `.bak`
- [x] 2.5 Implement `execve` fallback for non-systemd environments
- [x] 2.6 Implement upgrade status reporting: write `_sys:deploy:node:{node_id}` with status transitions (`draining` → `restarting` → `healthy` / `failed`)
- [x] 2.7 Implement `NodeRollback` handler: Nix profile rollback or restore `.bak` binary, then restart
- [x] 2.8 Write unit tests for drain logic, binary validation, status reporting

## 3. Cluster Deploy Coordinator (`aspen-deploy` crate)

- [x] 3.1 Create `crates/aspen-deploy/` crate with `Cargo.toml`, add to workspace members
- [x] 3.2 Implement `DeploymentState` enum and `DeploymentRecord` struct with serde serialization for KV storage at `_sys:deploy:current`
- [x] 3.3 Implement `DeploymentCoordinator` with CAS-based state transitions (`pending` → `deploying` → `completed` / `failed`)
- [x] 3.4 Implement rolling strategy: iterate nodes (followers first, leader last), send `NodeUpgrade` RPC per node, poll health between upgrades
- [x] 3.5 Implement quorum safety check: verify `(healthy_voters - upgrading - 1) >= quorum` before each node upgrade
- [x] 3.6 Implement health-gated progression: poll `GetHealth`, check Raft membership, verify log gap < 100 entries, bounded by `DEPLOY_HEALTH_TIMEOUT_SECS`
- [x] 3.7 Implement leader failover recovery: on becoming leader, read `_sys:deploy:current`, resume from last confirmed node
- [x] 3.8 Implement concurrent deployment rejection: CAS on `_sys:deploy:current` prevents two deployments from running simultaneously
- [x] 3.9 Implement rollback orchestration: send `NodeRollback` RPCs in rolling fashion to upgraded nodes
- [x] 3.10 Implement deployment history: write completed records to `_sys:deploy:history:{timestamp}`, prune beyond `MAX_DEPLOY_HISTORY`
- [x] 3.11 Write unit tests for state machine transitions, quorum safety calculations, rollback logic
- [x] 3.12 Add verified functions in `src/verified/` for quorum safety arithmetic and state transition validation

## 4. Deploy Executor (CI integration)

- [x] 4.1 Create `DeployExecutor` in `aspen-ci` (or `aspen-ci-executor-deploy/`) that handles `JobType::Deploy`
- [x] 4.2 Implement artifact resolution: read referenced build job result from KV, extract Nix store path or blob hash
- [x] 4.3 Implement deploy initiation: call `ClusterDeploy` RPC with resolved artifact
- [x] 4.4 Implement progress monitoring: poll `ClusterDeployStatus` every 5s, emit per-node progress as job logs
- [x] 4.5 Implement job completion: map deployment success/failure to CI job success/failure
- [x] 4.6 Register `DeployExecutor` in the worker/executor dispatch (excluded from regular worker pools, runs on leader)
- [x] 4.7 Write tests for artifact resolution from mock job results

## 5. RPC Handlers

- [x] 5.1 Add `ClusterDeploy` handler in `aspen-cluster-handler` that creates a deployment via `DeploymentCoordinator`
- [x] 5.2 Add `ClusterDeployStatus` handler that reads `_sys:deploy:current` and returns status
- [x] 5.3 Add `ClusterRollback` handler that triggers rollback via coordinator
- [x] 5.4 Add `NodeUpgrade` handler in `aspen-cluster-handler` that delegates to `NodeUpgradeExecutor`
- [x] 5.5 Add `NodeRollback` handler that delegates to rollback logic
- [x] 5.6 Wire handlers into `aspen-rpc-handlers` dispatch under `deploy` feature flag
- [x] 5.7 Add deploy feature flag to root `Cargo.toml` and propagate through feature chain

## 6. CLI Commands

- [x] 6.1 Add `cluster deploy <artifact-ref>` command with `--strategy`, `--max-concurrent`, `--health-timeout` flags
- [x] 6.2 Add `cluster deploy-status` command that displays current/last deployment state with per-node status table
- [x] 6.3 Add `cluster rollback` command that triggers deployment rollback
- [x] 6.4 Write CLI parse tests for new subcommands

## 7. Pipeline Config Update

- [x] 7.1 Update `.aspen/ci.ncl` with a stage 4 `deploy` block: `type = 'deploy`, `artifact_from = "build-node"`, `strategy = 'rolling`, `depends_on = ["build", "test"]`
- [x] 7.2 Add `deploy` job validation in `JobConfig::validate()`: require `artifact_from`, validate `strategy` and `max_concurrent` values
- [x] 7.3 Update Nickel CI schema contract to accept deploy fields

## 8. Dogfood Script

- [x] 8.1 Add `do_deploy` function to `scripts/dogfood-local.sh` that calls `cli cluster deploy` with the CI-built artifact
- [x] 8.2 Add deployment status polling loop to `do_deploy` with progress output
- [x] 8.3 Add `do_full_loop` that runs: start → push → build → deploy → verify (binary on running node matches CI output)
- [x] 8.4 Update the `verify` step to check that the running node reports the new version via `cluster status`

## 9. NixOS VM Integration Test

- [x] 9.1 Create `nix/tests/ci-dogfood-deploy.nix` VM test with a 3-node cluster
- [x] 9.2 Test pushes source to Forge, waits for CI pipeline (build + test stages), then triggers deploy stage
- [x] 9.3 Verify each node restarts with the new binary: check version output, health status, and Raft membership
- [x] 9.4 Verify cluster remains operational throughout the rolling upgrade (KV read/write works at every step)
- [x] 9.5 Test rollback: after successful deploy, trigger rollback, verify nodes revert to previous binary

## 10. Documentation

- [x] 10.1 Add `docs/deploy.md` covering deployment architecture, configuration, CLI usage, and safety guarantees
- [x] 10.2 Update `AGENTS.md` with deploy-related module descriptions, feature flags, and test commands
- [x] 10.3 Update `README.md` self-hosting section to describe the complete Forge → CI → Build → Deploy loop
