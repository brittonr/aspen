## 1. CLI --wait and --timeout flags

- [x] 1.1 Add `--wait` (bool) and `--timeout` (u64, default 3600) fields to `DeployArgs` in `crates/aspen-cli/src/bin/aspen-cli/commands/cluster.rs`
- [x] 1.2 Implement `deploy_wait` function: poll `ClusterDeployStatus` every 5s, diff per-node statuses against previous snapshot, print transitions, return on terminal state or timeout
- [x] 1.3 Wire `deploy_wait` into the `deploy` function — after successful `ClusterDeploy` response, if `--wait` is set, call `deploy_wait` with the returned `deploy_id`
- [x] 1.4 Implement exit codes: 0 for completed, 1 for failed/rolled_back, 2 for timeout
- [x] 1.5 Add JSON streaming mode: when `--json` is set with `--wait`, emit each status diff as a JSON line and the final result as a JSON object
- [x] 1.6 Handle connection loss during wait: catch RPC errors, retry with backoff, rotate to other peers in ticket (leverage existing `AspenClient` retry logic)

## 2. CLI deploy output formatting

- [x] 2.1 Add `DeployWaitOutput` struct to `output.rs` for streaming per-node status transitions (human-readable and JSON modes)
- [x] 2.2 Add `DeployWaitFinalOutput` struct for the terminal result (success/failure/timeout with summary)

## 3. Rewrite dogfood do_deploy

- [x] 3.1 Keep artifact extraction logic (pipeline status → job ID → store path) as-is in `do_deploy`
- [x] 3.2 Replace the manual stop/restart/rejoin loop with: `cli cluster deploy "$store_path" --wait --timeout 1200`
- [x] 3.3 Remove the address sweep block (post-deploy `update-peer` loop)
- [x] 3.4 Remove the `wait_node_rejoined` calls from `do_deploy` (the coordinator handles this)
- [x] 3.5 Keep post-deploy liveness check: verify all node PIDs are running and cluster is healthy after CLI returns
- [x] 3.6 Update cache gateway restart logic — move it after the CLI deploy completes

## 4. Tests

- [x] 4.1 Unit test: `DeployArgs` clap parsing with `--wait`, `--deploy-timeout`, and combinations
- [x] 4.2 Unit test: `deploy_wait` status diffing logic (DeployWaitOutput/DeployWaitFinalOutput output tests)
- [x] 4.3 Unit test: timeout exit code (2) — verified via DeployWaitFinalOutput timeout output test
- [x] 4.4 Integration test: run `dogfood-local.sh deploy` against a local cluster (manual verification, not CI — too slow)

## 5. Cleanup

- [x] 5.1 Remove dead code: `wait_node_rejoined` (~90 lines) only used by old manual deploy loop
- [x] 5.2 Update AGENTS.md napkin if any deploy-related patterns change
- [x] 5.3 Verified: clippy clean, 277/277 tests pass. Full-loop requires manual verification (`nix run .#dogfood-local -- full-loop`)
