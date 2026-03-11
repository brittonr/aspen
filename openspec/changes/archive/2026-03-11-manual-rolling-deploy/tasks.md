## 1. Types, Constants, and RPC Messages

- [x] 1.1 Add deploy-related constants to `aspen-constants`: `DRAIN_TIMEOUT_SECS` (30), `DEPLOY_HEALTH_TIMEOUT_SECS` (120), `MAX_DEPLOY_HISTORY` (50), `DEPLOY_STATUS_POLL_INTERVAL_SECS` (5), `DEPLOY_LOG_GAP_THRESHOLD` (100)
- [x] 1.2 Add `ClusterDeploy`, `ClusterDeployStatus`, `ClusterRollback`, `NodeUpgrade`, `NodeRollback` request variants to `ClientRpcRequest` in `aspen-client-api`
- [x] 1.3 Add corresponding response variants to `ClientRpcResponse`: `ClusterDeployResult`, `ClusterDeployStatusResult`, `ClusterRollbackResult`, `NodeUpgradeResult`, `NodeRollbackResult`
- [x] 1.4 Implement `to_operation()`, `domain()`, `variant_name()` for new RPC variants
- [x] 1.5 Add postcard discriminant golden tests for the new variants

## 2. Deployment Types Crate (`aspen-deploy`)

- [x] 2.1 Create `crates/aspen-deploy/Cargo.toml`, add to workspace members, add `deploy` feature to root Cargo.toml
- [x] 2.2 Define `DeploymentStatus` enum: `Pending`, `Deploying`, `Completed`, `Failed`, `RollingBack`, `RolledBack`
- [x] 2.3 Define `NodeDeployStatus` enum: `Pending`, `Draining`, `Upgrading`, `Restarting`, `Healthy`, `Failed(String)`
- [x] 2.4 Define `DeploymentRecord` struct: `deploy_id`, `artifact` (store path or blob hash), `strategy`, `status`, `nodes: Vec<NodeDeployState>`, `created_at_ms`, `updated_at_ms`
- [x] 2.5 Define `DeployArtifact` enum: `NixStorePath(String)`, `BlobHash(String)` — what gets passed around
- [x] 2.6 Define `DeployStrategy` enum: `Rolling { max_concurrent: u32 }`

## 3. Quorum Safety (verified functions)

- [x] 3.1 Create `aspen-deploy/src/verified/mod.rs` and `quorum.rs` with pure functions
- [x] 3.2 Implement `quorum_size(voter_count: u32) -> u32` — returns `ceil((voter_count + 1) / 2)`
- [x] 3.3 Implement `can_upgrade_node(healthy_voters: u32, currently_upgrading: u32, voter_count: u32) -> bool` — checks quorum invariant
- [x] 3.4 Implement `max_concurrent_upgrades(voter_count: u32) -> u32` — returns `(voter_count - 1) / 2`, minimum 1
- [x] 3.5 Write unit tests for edge cases: 1-node, 3-node, 5-node, 7-node clusters
- [x] 3.6 Add Verus specs in `aspen-deploy/verus/quorum_spec.rs` proving quorum safety invariant holds

## 4. Node-Level Upgrade (`aspen-cluster`)

- [x] 4.1 Create `aspen-cluster/src/upgrade/mod.rs` with `NodeUpgradeExecutor` struct
- [x] 4.2 Implement graceful drain: set drain flag → reject new client RPCs → return `NOT_LEADER` for writes → wait for in-flight ops up to `DRAIN_TIMEOUT_SECS` → force-proceed if timeout
- [x] 4.3 Implement Nix upgrade path: `nix-env --profile <path> --set <store-path>`, verify binary at `<store-path>/bin/aspen-node`
- [x] 4.4 Implement blob upgrade path: download blob to staging dir, SHA-256 validate, `--version` smoke test, atomic rename, preserve `.bak`
- [x] 4.5 Implement restart detection: check for systemd (`$NOTIFY_SOCKET` / `$INVOCATION_ID`) → `systemctl restart`, else `execve` with original args
- [x] 4.6 Implement status reporting: write `_sys:deploy:node:{node_id}` with status transitions through the upgrade lifecycle
- [x] 4.7 Implement `NodeRollback`: Nix `--rollback` or restore `.bak`, then restart
- [x] 4.8 Write unit tests for drain logic (mock in-flight counter), binary validation, status state machine

## 5. Deployment Coordinator (`aspen-deploy`)

- [x] 5.1 Implement `DeploymentCoordinator` struct taking `Arc<dyn KeyValueStore>` for state storage
- [x] 5.2 Implement `start_deployment()`: CAS write `_sys:deploy:current` with `Pending` status, fail if existing deployment is `Deploying`
- [x] 5.3 Implement `run_deployment()`: loop through nodes (followers first, leader last), call quorum check, send `NodeUpgrade` RPC, poll health, update per-node status via CAS
- [x] 5.4 Implement health polling: check `GetHealth`, Raft membership, log gap < `DEPLOY_LOG_GAP_THRESHOLD`, bounded by `DEPLOY_HEALTH_TIMEOUT_SECS`
- [x] 5.5 Implement failure handling: on health timeout → mark node `Failed`, mark deployment `Failed`, stop upgrading
- [x] 5.6 Implement leader-last logic: detect own node ID, defer self-upgrade to final step, write state before self-restart so new leader can finalize
- [x] 5.7 Implement leader failover recovery: on startup/leader election, check `_sys:deploy:current` for in-progress deployment, resume if found
- [x] 5.8 Implement `rollback_deployment()`: read current deployment, send `NodeRollback` to upgraded nodes in rolling fashion, update status
- [x] 5.9 Implement `get_status()`: read `_sys:deploy:current`, fall back to latest `_sys:deploy:history:*`
- [x] 5.10 Implement history management: on completion, copy record to `_sys:deploy:history:{timestamp}`, delete `_sys:deploy:current`, prune old entries
- [x] 5.11 Write integration tests: mock KV store, verify state transitions, concurrent deploy rejection, failover recovery

## 6. RPC Handlers

- [x] 6.1 Create `deploy` handler module in `aspen-cluster-handler` behind `deploy` feature flag
- [x] 6.2 Implement `ClusterDeploy` handler: validate artifact reference, delegate to `DeploymentCoordinator::start_deployment()`, spawn background task for `run_deployment()`
- [x] 6.3 Implement `ClusterDeployStatus` handler: delegate to `DeploymentCoordinator::get_status()`
- [x] 6.4 Implement `ClusterRollback` handler: delegate to `DeploymentCoordinator::rollback_deployment()`
- [x] 6.5 Implement `NodeUpgrade` handler: delegate to `NodeUpgradeExecutor`
- [x] 6.6 Implement `NodeRollback` handler: delegate to rollback logic
- [x] 6.7 Wire handlers into `aspen-rpc-handlers` dispatch with `deploy` feature flag propagation through feature chain

## 7. CLI Commands

- [x] 7.1 Add `cluster deploy <artifact>` command accepting either a Nix store path or blob hash, with `--strategy rolling` (default), `--max-concurrent` (default 1), `--health-timeout` (default 120) flags
- [x] 7.2 Add `cluster deploy-status` command: display deployment state as a table with per-node status, elapsed time, artifact reference
- [x] 7.3 Add `cluster rollback` command: trigger rollback on current/last deployment
- [x] 7.4 Write CLI parse tests for all three new subcommands
- [x] 7.5 Implement progress polling in `cluster deploy`: after initiating, poll status every 5s and print per-node progress until completion or failure

## 8. Testing

- [x] 8.1 Unit tests for quorum safety verified functions (edge cases: 1, 2, 3, 5, 7 nodes; all voters upgrading simultaneously)
- [x] 8.2 Unit tests for deployment state machine (full lifecycle, concurrent rejection, failover recovery)
- [x] 8.3 Unit tests for node drain (clean drain, timeout drain, cancelled operations count)
- [x] 8.4 Integration test with in-memory KV: start deployment, simulate node health responses, verify state transitions
- [x] 8.5 Add `deploy` to nextest profiles (quick profile may want to skip long-running deploy simulation tests)
