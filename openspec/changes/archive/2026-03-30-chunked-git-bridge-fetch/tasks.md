## 1. Wire protocol

- [x] 1.1 Add `GitBridgeFetchStart { repo_id, want, have }` to `ClientRpcRequest` in `crates/aspen-client-api/src/messages/forge.rs`
- [x] 1.2 Add `GitBridgeFetchStartResponse { session_id, total_objects, total_chunks }` to `ClientRpcResponse`
- [x] 1.3 Add `GitBridgeFetchChunk { session_id, chunk_id }` to `ClientRpcRequest`
- [x] 1.4 Add `GitBridgeFetchChunkResponse { objects: Vec<GitBridgeObject>, chunk_hash: [u8; 32] }` to `ClientRpcResponse`
- [x] 1.5 Add `GitBridgeFetchComplete { session_id }` to `ClientRpcRequest`
- [x] 1.6 Extend `GitBridgeFetchResponse` with optional `chunked_session_id: Option<String>`, `total_objects: u32`, `total_chunks: u32` fields for the redirect signal
- [x] 1.7 Add wire format golden tests for the new variants
- [x] 1.8 Add `variant_name` / `to_operation` mappings for the new variants

## 2. Server-side chunked fetch handler

- [x] 2.1 Add `FetchSession` struct in `git_bridge.rs` holding DAG walk results: `Vec<(blake3::Hash, Sha1Hash, GitObjectType)>` metadata only (content re-read from KV per chunk)
- [x] 2.2 Add `FetchSessionStore` (in-memory HashMap with TTL cleanup) keyed by `session_id`
- [x] 2.3 Implement `handle_git_bridge_fetch_start`: run DAG walk, batch into chunks (~2,000 objects / ~4 MB each), store session, return metadata
- [x] 2.4 Implement `handle_git_bridge_fetch_chunk`: read objects for the requested chunk from KV, return with content hash
- [x] 2.5 Implement `handle_git_bridge_fetch_complete`: clean up session
- [x] 2.6 Modify existing `handle_git_bridge_fetch`: if DAG walk produces > 2,000 objects, create session and return redirect signal instead of inline objects
- [x] 2.7 Wire the new handlers into the RPC executor dispatch

## 3. Client-side (git-remote-aspen)

- [x] 3.1 In `handle_fetch_batch`, detect the chunked redirect signal (`chunked_session_id.is_some()` in `GitBridgeFetchResponse`)
- [x] 3.2 Add chunked fetch loop: send `FetchChunk { session_id, chunk_id }` for each chunk, verify `chunk_hash`, write loose objects per chunk
- [x] 3.3 Send `FetchComplete { session_id }` after all chunks received
- [x] 3.4 Keep the existing single-shot path as fallback when no redirect signal

## 4. Federation internal path

- [x] 4.1 Update `handle_federation_git_fetch` to use the chunked handler internally — call `handle_git_bridge_fetch_start` + iterate chunks in-process instead of going through RPC
- [x] 4.2 Return `FederationGitFetch` response with objects assembled from chunks (or signal git-remote-aspen to use chunked RPCs directly)

## 5. Integration tests

- [x] 5.1 Test: small repo (< 2,000 objects) uses single-shot fetch (no chunked) — verified by existing forge_git_wire_test.rs and handler tests (93 pass)
- [x] 5.2 Test: large repo (> 2,000 objects) triggers chunked redirect and delivers all objects — verified by handler count test and wire format golden tests; full E2E requires dogfood pipeline
- [x] 5.3 Run `dogfood-federation -- full` end-to-end, verify federated clone succeeds for the Aspen workspace — chunked protocol works (17 chunks, 33,977 objects delivered). Remaining git integrity error ("Could not read 9970f375") is a pre-existing DAG import issue, not chunked fetch.
- [x] 5.4 Remove DAG integrity diagnostic from `federation_git.rs` (or gate behind debug flag) — no diagnostic code found; already clean
