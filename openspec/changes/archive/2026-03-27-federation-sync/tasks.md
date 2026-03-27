## 1. Pure ref-diff function

- [x] 1.1 Add `compute_ref_diff` and `RefDiff` struct in `crates/aspen-federation/src/verified/ref_diff.rs`
- [x] 1.2 Re-export from `crates/aspen-federation/src/verified/mod.rs` and `crates/aspen-federation/src/lib.rs`
- [x] 1.3 Unit tests for `compute_ref_diff`: in-sync, to-pull-only, to-push-only, mixed, divergent conflicts, empty inputs

## 2. RPC and Client API

- [x] 2.1 Add `FederationBidiSync` request variant with `peer_node_id`, `peer_addr`, `repo_id`, `push_wins` fields to `crates/aspen-client-api/src/messages/federation.rs`
- [x] 2.2 Add `FederationBidiSyncResponse` struct with `is_success`, `pulled`, `pushed`, `pull_refs_updated`, `push_refs_updated`, `conflicts`, `errors`, `error`
- [x] 2.3 Add corresponding `ClientRpcRequest::FederationBidiSync` and `ClientRpcResponse::FederationBidiSyncResult` variants in `mod.rs`
- [x] 2.4 Wire `to_operation` and `domain()` routing for the new variant

## 3. Forge handler

- [x] 3.1 Add `handle_federation_bidi_sync` in `crates/aspen-forge-handler/src/handler/handlers/federation.rs`: connect, handshake, get remote state, get local state, compute diff, resolve conflicts, pull, push, return combined stats
- [x] 3.2 Wire in `crates/aspen-forge-handler/src/executor.rs`

## 4. CLI

- [x] 4.1 Add `--repo` and `--push-wins` optional flags to `SyncArgs` in `crates/aspen-cli/src/bin/aspen-cli/commands/federation.rs`
- [x] 4.2 Update `federation_sync` to dispatch: with `--repo` → `FederationBidiSync` RPC, without → existing `FederationSyncPeer`
- [x] 4.3 Format and print the bidi sync response (pulled, pushed, conflicts)

## 5. Tests

- [x] 5.1 Wire-level integration test: two clusters with divergent repos, sync transfers objects in both directions
- [x] 5.2 Wire-level test: push rejected (untrusted) but pull succeeds, partial failure reported correctly
