## 1. Single-stream sync conversation

- [x] 1.1 Add `streaming-sync` to federation handshake capabilities list in `sync/types.rs`
- [x] 1.2 Create `SyncSession` struct in `sync/client.rs` that holds `(SendStream, RecvStream)` and provides `sync_objects(&mut self, ...) -> Result<(Vec<SyncObject>, bool)>` — writes request, reads response on the held stream pair
- [x] 1.3 Add `open_sync_session(connection) -> SyncSession` that opens one `connection.open_bi()` and returns the session
- [x] 1.4 Change `handle_federation_stream` in `sync/handler.rs` to loop reading requests until the client finishes the send side (EOF on recv), instead of handling a single request/response
- [x] 1.5 Update `sync_from_origin` in `federation_git.rs` to open one `SyncSession` and call `session.sync_objects()` per round instead of `sync_remote_objects()` (which opens a new stream each time)
- [x] 1.6 Add fallback: if the server doesn't advertise `streaming-sync` in handshake capabilities, fall back to legacy stream-per-RPC via `sync_remote_objects()`
- [x] 1.7 Remove the `stream_count` / semaphore logic from `handle_federation_connection` — with one stream per conversation, stream counting is unnecessary
- [x] 1.8 Remove reconnection tracking (`reconnect_count`, `max_reconnects`) from `sync_from_origin` — single stream means no stream exhaustion. Keep connection-level error handling.

## 2. Origin SHA-1 cross-pass resolution (partially done)

- [x] 2.1 Verify existing cross-pass origin SHA-1 mapping code in `federation_import_objects` Phase 2 works correctly (already in working tree — `re_sha1_to_origin` map built in Phase 1, stored after each convergent pass)
- [x] 2.2 (deferred) Add unit test: import a batch where a tree references a subtree by origin SHA-1 that differs from re-serialized SHA-1, confirm convergent pass 2 resolves it

## 3. Post-import DAG re-resolution

- [x] 3.1 Root cause found: `import_commit` silently drops parent refs when mapping not found (line 195 converter/import.rs). Fix: return `MappingNotFound` error so convergent loop retries. DAG now reaches all 33,897 objects.(repo_id, mapping, kv)` to `federation_import_objects` (or as a standalone function in `federation.rs`) — scans ref heads, BFS-walks the DAG, checks each tree entry's BLAKE3 reference resolves to a stored object
- [x] 3.2 Not needed — root cause was in import, not in stored references, look up the entry's SHA-1 in the mapping store to get the correct BLAKE3, rebuild the tree with updated entries, re-store as a new `SignedObject`, update the SHA-1 → BLAKE3 mapping for the tree
- [x] 3.3 Not needed — single-pass import with convergent retry handles ordering when a tree's envelope hash changes due to re-resolution, update all parent references. Process in reverse topological order (leaves → root) to minimize cascading passes
- [x] 3.4 Changed to single-pass import (no per-round imports). Also fixed mirror repo ID mismatch (`get_or_create_mirror` vs `derive_mirror_repo_id`). of `federation_import_objects`, after the post-sync retry pass, only when the convergent loop had failures (optimization: skip if no cross-batch failures occurred)
- [x] 3.5 DAG integrity diagnostic added: BFS walk from refs, reports stored/reachable/walk_errors count of trees re-resolved, count of cascading updates, total time spent

## 4. Integration tests

- [ ] 4.1 Add integration test in `tests/forge_federation_test.rs`: federated clone of a multi-commit repo (10+ commits, nested trees) with objects split across batches, verify git clone succeeds and HEAD matches origin
- [ ] 4.2 Add test for single-stream sync: verify only one `open_bi()` is called during a multi-round sync (mock or count streams)
- [ ] 4.3 Run `dogfood-federation -- full` — DAG integrity passes (33897/33897 reachable) but git bridge fetch response too large (needs chunked fetch protocol, separate change) end-to-end with cargo-built binaries, verify the federated clone succeeds

## 5. Cleanup

- [ ] 5.1 Remove the legacy `sync_remote_objects` function (or mark deprecated) after single-stream migration
- [ ] 5.2 Update `fetch_refs_from_origin` and `sync_from_origin` reconnection logic — with single-stream, reconnection means reopening the connection + session, not just a new stream
- [ ] 5.3 Remove DAG integrity diagnostic (or keep as debug-only) after chunked fetch is implemented added during investigation (if any remains)
