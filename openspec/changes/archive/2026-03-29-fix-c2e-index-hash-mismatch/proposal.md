## Why

The federation c2e (content→envelope) index is broken due to two overlapping hash mismatches. This prevents incremental federation sync from deduplicating objects — every sync re-transfers the full repo instead of only new objects.

The c2e index maps a content identifier → BLAKE3 envelope hash, enabling the exporter to skip objects the remote already has. Two bugs make this non-functional:

1. **Hash domain confusion**: The remote sends envelope BLAKE3 hashes in `have_hashes`, but the exporter treats them as content hashes for c2e lookup. These are completely different 32-byte values — explains why only 4/33K entries matched (coincidental).

2. **Content hash divergence**: Even after fixing (1), import computes `blake3(original_git_bytes)` while export computes `blake3(reconstructed_git_bytes)`. These differ because `TreeObject::new()` re-sorts entries using Rust's `str::cmp` instead of git's mode-aware sort order. The sort divergence also means exported trees have wrong SHA-1 hashes — a git correctness bug independent of c2e.

## What Changes

- **c2e index key**: Re-key from `blake3(content)` to `sha1_hex`. SHA-1 is the canonical git object identifier, computed identically on both sides by construction.
- **have_set hash domain**: Change `collect_local_blake3_hashes` to send SHA-1 hashes. The exporter converts these to envelope BLAKE3 via the c2e index for DAG walk dedup.
- **TreeObject sort order**: Fix `TreeObject::new()` to use git's mode-aware sort (append `/` for directories when comparing). This restores byte-identical round-trip for tree objects and correct SHA-1 computation on export.
- **Commit message round-trip**: The `str::lines().collect().join("\n")` pattern drops trailing context for edge-case messages. Preserve raw message bytes through import/export.

## Capabilities

### New Capabilities

- `c2e-sha1-index`: SHA-1 keyed content-to-envelope index for federation dedup
- `git-object-roundtrip`: Byte-identical import→export round-trip for all git object types

### Modified Capabilities

## Impact

- `crates/aspen-forge/src/git/object.rs` — TreeObject sort order
- `crates/aspen-forge/src/git/bridge/importer.rs` — c2e index write path
- `crates/aspen-forge/src/git/bridge/converter/import.rs` — commit message parsing
- `crates/aspen-forge/src/git/bridge/converter/export.rs` — tree entry iteration
- `crates/aspen-forge/src/resolver.rs` — c2e scan, have_set conversion, export content hash
- `crates/aspen-forge-handler/src/handler/handlers/federation.rs` — `collect_local_blake3_hashes`, import stats
- `crates/aspen-forge-handler/src/handler/handlers/federation_git.rs` — ref translation
- Wire protocol: `SyncObject.hash` stays as blake3(content) for verification; no breaking wire change
