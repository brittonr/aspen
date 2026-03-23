## Tasks

### 1. Add `cluster update-peer` CLI subcommand

- [ ] 1.1 Add `UpdatePeer(UpdatePeerArgs)` variant to `ClusterCommand` enum in `crates/aspen-cli/src/bin/aspen-cli/commands/cluster.rs`
- [ ] 1.2 Add `UpdatePeerArgs` struct with `--node-id: u64` and `--addr: String` fields
- [ ] 1.3 Implement `update_peer()` async fn: sends `ClientRpcRequest::AddPeer { node_id, endpoint_addr }`, handles `ClientRpcResponse::AddPeerResult`
- [ ] 1.4 Add `UpdatePeerOutput` to `output.rs` with `is_success` and `error` fields, implementing `Outputable`
- [ ] 1.5 Wire into `ClusterCommand::run()` match arm

### 2. Update dogfood deploy script

- [ ] 2.1 Replace the post-deploy `add-learner` address sweep with `update-peer` calls — for each node, tell every OTHER node about its current address using `cli_node "$other" cluster update-peer --node-id "$i" --addr ...`
- [ ] 2.2 Remove the per-node `add-learner` re-announce during the deploy loop (it's redundant with the post-deploy sweep and fails when followers can't reach the leader)
- [ ] 2.3 Test: run `nix run .#dogfood-local -- full-loop` with `ASPEN_NODE_COUNT=3` end-to-end

### 3. Verification

- [ ] 3.1 `cargo nextest run -E 'test(/add_peer/)' -E 'test(/update_peer/)'` — existing handler tests pass
- [ ] 3.2 `cargo clippy -p aspen-cli` — no warnings
- [ ] 3.3 3-node dogfood full-loop passes: start → push → build → deploy → verify
