## 1. Wire protocol: PushObjects request + PushResult response

- [x] 1.1 Add `FederationRequest::PushObjects { fed_id, objects: Vec<SyncObject>, ref_updates: Vec<RefEntry> }` to `crates/aspen-federation/src/sync/types.rs`
- [x] 1.2 Add `FederationResponse::PushResult { accepted: bool, imported: u32, skipped: u32, errors: Vec<String> }` to the same file
- [x] 1.3 Unit test: round-trip serialize/deserialize of `PushObjects` and `PushResult` via postcard

## 2. Server handler: accept and import pushed objects

- [x] 2.1 Add `handle_push_objects` in `crates/aspen-federation/src/sync/handler.rs` — check trust (reject untrusted with `"unauthorized"` error), enforce `MAX_OBJECTS_PER_SYNC` limit, then delegate to resolver
- [x] 2.2 Add `import_pushed_objects` method to `FederationResourceResolver` trait in `crates/aspen-federation/src/resolver.rs` with default returning `Err(Unsupported)`
- [x] 2.3 Wire `FederationRequest::PushObjects` dispatch in the handler's match arm
- [x] 2.4 Implement `import_pushed_objects` on `ForgeResolver` in `crates/aspen-forge/src/resolver.rs`: convert `SyncObject` items to `(Sha1Hash, GitObjectType, Vec<u8>)`, call `GitObjectImporter::import_objects`, then update refs via `import_ref` or direct ref set. Create mirror repo if it doesn't exist.

## 3. Client function: push_to_cluster

- [x] 3.1 Add `push_to_cluster(conn, fed_id, objects, ref_updates) -> Result<PushResult>` in `crates/aspen-federation/src/sync/client.rs` — sends `PushObjects` request, reads `PushResult` response
- [x] 3.2 Unit test: verify request construction (fed_id, object count, ref count)

## 4. Client RPC + node handler

- [x] 4.1 Add `ClientRpcRequest::FederationPush { peer_node_id, peer_addr, repo_id }` and `ClientRpcResponse::FederationPushResult(FederationPushResponse)` in `crates/aspen-client-api/src/messages/federation.rs`
- [x] 4.2 Add `FederationPushResponse { is_success, imported, skipped, refs_updated, errors, error }` struct
- [x] 4.3 Add `handle_federation_push` in `crates/aspen-forge-handler/src/handler/handlers/federation.rs` — connect to peer, export local repo objects via `ForgeResolver`, call `push_to_cluster`
- [x] 4.4 Wire the new request in `crates/aspen-forge-handler/src/executor.rs` match arm

## 5. CLI command

- [x] 5.1 Add `Push(PushArgs)` variant to `FederationCommand` in `crates/aspen-cli/src/bin/aspen-cli/commands/federation.rs` with `--peer`, `--repo`, optional `--addr`
- [x] 5.2 Implement `federation_push` handler that sends `FederationPush` RPC and prints results

## 6. Integration test

- [x] 6.1 Two-cluster integration test: Alice pushes a repo to Bob, Bob verifies mirror repo has correct objects and refs (mirrors the existing `test_federation_two_cluster_git_clone` pattern)
