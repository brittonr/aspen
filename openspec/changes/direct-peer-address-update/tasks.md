## Tasks

### 1. Add `cluster update-peer` CLI subcommand

- [x] 1.1 Add `UpdatePeer(UpdatePeerArgs)` variant to `ClusterCommand` enum in `crates/aspen-cli/src/bin/aspen-cli/commands/cluster.rs`
- [x] 1.2 Add `UpdatePeerArgs` struct with `--node-id: u64` and `--addr: String` fields
- [x] 1.3 Implement `update_peer()` async fn: sends `ClientRpcRequest::AddPeer { node_id, endpoint_addr }`, handles `ClientRpcResponse::AddPeerResult`
- [x] 1.4 Add `UpdatePeerOutput` to `output.rs` with `is_success` and `error` fields, implementing `Outputable`
- [x] 1.5 Wire into `ClusterCommand::run()` match arm

### 2. Update dogfood deploy script

- [x] 2.1 Replace the post-deploy `add-learner` address sweep with `update-peer` calls — for each node, tell every OTHER node about its current address using `cli_node "$other" cluster update-peer --node-id "$i" --addr ...`
- [x] 2.2 Remove the per-node `add-learner` re-announce during the deploy loop (it's redundant with the post-deploy sweep and fails when followers can't reach the leader)
- [x] 2.3 Test: run `nix run .#dogfood-local -- full-loop` with `ASPEN_NODE_COUNT=3` end-to-end

### 3. Verification

- [x] 3.1 `cargo nextest run -E 'test(/add_peer/)' -E 'test(/update_peer/)'` — existing handler tests pass
- [x] 3.2 `cargo clippy -p aspen-cli` — no warnings
- [x] 3.3 3-node dogfood full-loop passes: start → push → build → deploy → verify
