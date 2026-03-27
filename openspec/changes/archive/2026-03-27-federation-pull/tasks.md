## 1. RPC and Client API

- [x] 1.1 Extend `FederationPull` in `crates/aspen-client-api/src/messages/federation.rs` with optional `peer_node_id`, `peer_addr`, and `repo_id` fields (use `#[serde(default)]` for wire compat)
- [x] 1.2 Update the corresponding `ClientRpcRequest::FederationPull` variant in `crates/aspen-client-api/src/messages/mod.rs` to match
- [x] 1.3 Update `to_operation` mapping if needed for the new fields

## 2. Mirror Metadata

- [x] 2.1 Add `origin_node_id: Option<String>` and `origin_addr_hint: Option<String>` to `MirrorMetadata` in `crates/aspen-forge-handler/src/handler/handlers/federation.rs`
- [x] 2.2 Populate `origin_node_id` and `origin_addr_hint` in `get_or_create_mirror` when creating from a cold pull
- [x] 2.3 Use stored `origin_node_id` (falling back to `origin_cluster_key`) when `handle_federation_pull` reconnects for mirror pulls

## 3. Forge Handler — Cold Pull

- [x] 3.1 Add `handle_federation_pull_remote` function that takes `peer_node_id`, `peer_addr`, and `repo_id`, connects to remote, handshakes to get cluster identity, constructs `FederatedId`, creates mirror, then delegates to `handle_federation_fetch_refs`
- [x] 3.2 Update `handle_federation_pull` to dispatch between cold-pull (peer+repo provided) and mirror-pull (mirror_repo_id provided) modes
- [x] 3.3 Wire the new dispatch in `crates/aspen-forge-handler/src/executor.rs` for the `FederationPull` match arm

## 4. CLI

- [x] 4.1 Extend `PullArgs` in `crates/aspen-cli/src/bin/aspen-cli/commands/federation.rs` with `--peer` and `--addr` optional flags
- [x] 4.2 Update `federation_pull` to pass `peer_node_id`, `peer_addr`, and `repo_id` in the RPC request when `--peer` is provided
- [x] 4.3 Add validation: if `--peer` is set, `--repo` is required; if `--peer` is absent, `--repo` is treated as local mirror ID

## 5. Tests

- [x] 5.1 Unit test: `MirrorMetadata` roundtrip with new fields (serde compat with old format that lacks them)
- [x] 5.2 Integration test: two-node cluster, create repo on node A, cold pull from node B, verify mirror content matches
- [x] 5.3 Integration test: push new commit on A, incremental pull on B, verify only delta transferred (fetched > 0, already_present > 0)
