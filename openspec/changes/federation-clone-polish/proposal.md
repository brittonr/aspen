## Why

Federated git clone works end-to-end but has three loose ends from the initial implementation that hurt correctness and efficiency:

1. **Incremental sync broken**: `have_hashes` sent by the client are content hashes, but the DAG walk compares envelope hashes. No dedup occurs — every sync re-transfers the full DAG.
2. **Multi-branch ref translation wrong**: `translate_ref_hashes` tries all commit content hashes for each ref and takes the first match. With multiple branches pointing to different commits, refs get the wrong local BLAKE3.
3. **Diagnostic logging left in**: info-level logs for every KV read/write and export result clutter production logs.

## What Changes

- Fix incremental federation sync by aligning hash types in the `have_set` → `known_blake3` path
- Fix multi-branch ref translation by matching each ref to its specific commit via SHA1 computed from the raw content
- Remove diagnostic info-level logs added during debugging, keep warn-level for actual failures

## Capabilities

### New Capabilities

- `federation-clone-polish`: Fix incremental sync dedup, multi-branch ref translation, and remove diagnostic logging

### Modified Capabilities

## Impact

- `crates/aspen-forge/src/resolver.rs` — hash alignment in `export_git_objects`
- `crates/aspen-forge/src/git/bridge/exporter.rs` — remove diagnostic logs
- `crates/aspen-forge/src/git/bridge/importer.rs` — remove diagnostic logs
- `crates/aspen-forge-handler/src/handler/handlers/federation_git.rs` — ref translation fix
