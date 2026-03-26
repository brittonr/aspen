## 1. Server-side git object serving

- [x] 1.1 Add `export_commit_dag_blake3` method to `GitBridgeExporter` that accepts `have_hashes: &HashSet<[u8; 32]>` (BLAKE3) instead of SHA1, translating internally via hash mappings
- [x] 1.2 Extend `ForgeResourceResolver::sync_objects` to handle `want_types` containing `"commit"`, `"tree"`, `"blob"` — walk DAG from ref heads using `export_commit_dag_blake3`, return git objects as `SyncObject` entries
- [x] 1.3 Add depth-limited DAG walk that stops at `MAX_DAG_DEPTH` and sets `has_more: true`
- [x] 1.4 Unit tests for `ForgeResourceResolver::sync_objects` with git object types using `DeterministicKeyValueStore` + `GitBridgeExporter`

## 2. Client-side object import

- [x] 2.1 Add `federation_import_objects` function in `aspen-forge-handler` that takes received `SyncObject` entries and imports them via `GitBridgeImporter::import_objects`
- [x] 2.2 Wire BLAKE3 verification into the import path (verify hash before import, drop mismatched objects)
- [x] 2.3 Implement `have_hashes` collection from existing mirror repo — scan local blob store for BLAKE3 hashes of objects already imported

## 3. Mirror repo lifecycle

- [x] 3.1 Add `create_mirror_repo` function to `ForgeNode` — creates a forge repo tagged with `_fed:mirror:{fed_id}` metadata (origin cluster key, fed_id, created_at)
- [x] 3.2 Add `get_or_create_mirror` helper that checks for existing mirror by fed_id, creates if missing
- [x] 3.3 Add `update_mirror_refs` function that sets ref heads on the mirror repo after object import
- [x] 3.4 Store last sync timestamp in mirror metadata KV key
- [x] 3.5 Add read-only guard: forge push handler checks mirror tag and rejects writes to mirror repos

## 4. Extended federation fetch handler

- [x] 4.1 Refactor `handle_federation_fetch_refs` into two-phase flow: fetch refs, then fetch git objects
- [x] 4.2 Wire mirror repo creation/update into the fetch handler — after object import, call `get_or_create_mirror` + `update_mirror_refs`
- [x] 4.3 Add transfer statistics to `FederationFetchRefsResponse` — per-type object counts, total bytes, skipped count
- [x] 4.4 Support pagination: loop `SyncObjects` while `has_more`, accumulating `have_hashes` between rounds

## 5. Federation pull CLI command

- [x] 5.1 Add `FederationPull` RPC request/response to `aspen-client-api` messages
- [x] 5.2 Add `pull` subcommand to `federation` CLI — takes `--repo <mirror-repo-id>`, reads mirror metadata to find origin peer and fed_id
- [x] 5.3 Wire `FederationPull` handler in `aspen-forge-handler` that reuses the two-phase fetch logic from task 4.1
- [x] 5.4 Add `federation pull` to CLI help and man page

## 6. Integration tests

- [ ] 6.1 Unit test: round-trip — create forge repo with objects on cluster A, federate it, fetch from cluster B, verify mirror has same refs and objects
- [ ] 6.2 Unit test: incremental sync — add a commit on cluster A, pull from cluster B, verify only new objects transferred
- [ ] 6.3 Unit test: mirror is read-only — attempt push to mirror repo, verify rejection
- [ ] 6.4 NixOS VM test: two-node federation with full fetch and git clone from mirror
