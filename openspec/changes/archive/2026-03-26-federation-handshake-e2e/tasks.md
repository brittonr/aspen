## 1. Add iroh_node_id to HealthResponse

- [x] 1.1 Add `iroh_node_id: Option<String>` field to `HealthResponse` in `crates/aspen-client-api/src/messages/cluster.rs` (with `#[serde(default)]`)
- [x] 1.2 Populate `iroh_node_id` from `ctx.endpoint_manager.peer_id().await` in `handle_get_health` in `crates/aspen-core-essentials-handler/src/core.rs`
- [x] 1.3 Surface `iroh_node_id` in CLI health JSON output in `crates/aspen-cli/src/bin/aspen-cli/output.rs` and `commands/cluster.rs`

## 2. Fix NixOS federation test

- [x] 2.1 Update the cross-cluster sync subtest to extract Alice's `iroh_node_id` from `cluster health --json`
- [x] 2.2 Pass extracted node ID to Bob's `federation sync --peer` command
- [x] 2.3 Assert that the sync result contains Alice's cluster name and trust status (or gracefully handle QUIC connectivity failure between VMs)

## 3. Verify

- [x] 3.1 Run `cargo nextest run -E 'test(/health/)' --features full` to confirm existing health tests pass with new field
- [x] 3.2 Run `cargo clippy --all-targets -- --deny warnings` and `nix run .#rustfmt`
