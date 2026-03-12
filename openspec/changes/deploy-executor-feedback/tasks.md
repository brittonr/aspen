## 1. Executor failure reporting

- [x] 1.1 Add `expected_binary: Option<String>` field to `NodeUpgradeConfig` in `crates/aspen-cluster/src/upgrade/types.rs`, default `Some("bin/aspen-node".into())` at all construction sites
- [x] 1.2 Update `upgrade_nix()` in `crates/aspen-cluster/src/upgrade/executor.rs` to use `self.config.expected_binary` instead of hardcoded `"bin/aspen-node"` — `None` skips binary check, `Some(path)` checks that path
- [x] 1.3 Update `handle_node_upgrade` in `crates/aspen-cluster-handler/src/handler/deploy.rs` — on `executor.execute()` Err, write `NodeDeployStatus::Failed(reason)` to KV key `_sys:deploy:node:{node_id}`
- [x] 1.4 Add unit tests for `upgrade_nix` with different `expected_binary` values (None, Some("bin/aspen-node"), Some("bin/cowsay"))

## 2. Coordinator KV status polling

- [x] 2.1 Add `check_node_kv_status()` method to `DeploymentCoordinator` that reads `_sys:deploy:node:{node_id}` via stale read and returns `Option<NodeDeployStatus>`
- [x] 2.2 Integrate KV status check into `poll_node_health()` in `crates/aspen-deploy/src/coordinator/health.rs` — check KV before/alongside GetHealth RPC, return failed immediately if KV says Failed
- [x] 2.3 Add unit tests for coordinator detecting executor failure via KV status

## 3. Wire protocol: expected_binary on NodeUpgrade RPC

- [x] 3.1 Add `expected_binary: Option<String>` to `ClientRpcRequest::NodeUpgrade` in `crates/aspen-client-api/`
- [x] 3.2 Update `handle_node_upgrade` to pass `expected_binary` from the RPC into `NodeUpgradeConfig`
- [x] 3.3 Add `expected_binary: Option<String>` to `ClusterDeploy` RPC (or `DeployRequest`) so the CI executor can propagate it
- [x] 3.4 Thread `expected_binary` through `DeploymentCoordinator.upgrade_single_node()` → `send_upgrade()` → `NodeUpgrade` RPC
- [x] 3.5 Update `IrohNodeRpcClient::send_upgrade` to pass `expected_binary` in the RPC request
- [x] 3.6 Update `resolve_upgrade_config()` in deploy handler to accept and forward `expected_binary`

## 4. CI deploy config support

- [x] 4.1 Add optional `expected_binary` field to deploy job params in `DeployJobParams` and `DeployExecutor::execute()`
- [x] 4.2 Parse `expected_binary` from CI pipeline config (Nickel `ci.ncl`) and pass through to `ClusterDeploy` RPC

## 5. VM tests

- [x] 5.1 Update single-node deploy test (`ci-dogfood-deploy.nix`) — happy path: add `expected_binary = "bin/cowsay"` to deploy job config, assert deploy stage succeeds
- [x] 5.2 Update multi-node deploy test (`ci-dogfood-deploy-multinode.nix`) — sad path: no expected_binary override (default bin/aspen-node), assert deploy stage fails with expected error, cluster healthy, KV intact

## 6. NixOS module fix

- [x] 6.1 Fix NixOS module ExecStart to use profile path `/nix/var/nix/profiles/aspen-node/bin/aspen-node` with ExecStartPre to initialize profile from `cfg.package` on first boot

## 7. Build verification

- [x] 7.1 `cargo build` succeeds with deploy feature
- [x] 7.2 `cargo nextest run` — existing deploy tests pass, new tests pass (485/486 pass, 1 pre-existing failure)
- [x] 7.3 `cargo clippy` clean (pre-existing test-only lint in aspen-cli)
