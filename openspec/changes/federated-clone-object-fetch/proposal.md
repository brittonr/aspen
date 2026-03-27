## Why

`git clone aspen://<ticket>/fed:<origin>:<repo>` silently returns an empty repository. The federation import path (`federation_import_objects`) calls `import_object()` sequentially in arrival order, but tree/commit imports require their dependencies' SHA-1→BLAKE3 mappings to already exist. When objects arrive out of dependency order (commit before tree, tree before blob), the imports fail silently and get skipped. The result: no commits or trees in the mirror, refs can't resolve to SHA-1 hashes, git sees zero refs.

## What Changes

- `federation_import_objects` switches from sequential `import_object()` calls to `import_objects()` (plural), which performs wave-based topological sorting and parallel import — the same path used by every regular `git push`.
- `ImportResult` gains a `mappings` field with per-object `(Sha1Hash, blake3::Hash)` pairs, so callers can build the `content_to_local_blake3` map needed for ref hash translation without extra KV lookups.
- The ghost mirror repo created by `get_or_create_mirror()` (forge identity ID ≠ derived mirror ID) is cleaned up so data isn't stored under two different repo IDs.

## Capabilities

### New Capabilities

- `federation-git-import`: Correct topologically-ordered import of git objects received via federation sync, with per-object hash mapping output for ref translation.

### Modified Capabilities

## Impact

- `crates/aspen-forge/src/git/bridge/importer.rs` — `ImportResult` struct gains `mappings: Vec<(Sha1Hash, blake3::Hash)>` field
- `crates/aspen-forge-handler/src/handler/handlers/federation.rs` — `federation_import_objects()` rewritten to use `import_objects()`
- `crates/aspen-forge-handler/src/handler/handlers/federation_git.rs` — `sync_from_origin()` updated to use new `ImportResult.mappings` for ref translation
- `crates/aspen-forge-handler/src/handler/handlers/git_bridge.rs` — `handle_git_bridge_push` and chunked push paths updated for new `ImportResult` shape
- Existing push tests must still pass (they already use `import_objects()`)
