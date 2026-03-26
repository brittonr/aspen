## Why

Federation git clone wiring works end-to-end (handshake, mirror creation, ref sync) but the clone produces an empty repo because the origin cluster fails to export git objects during `sync_objects`. The error: `blob storage error: storage error: encode error` from `IrohBlobStore::get_bytes`. The `ForgeResourceResolver::sync_objects` → `GitObjectExporter::export_dag_blake3` → `GitExporter::export_object` → `blobs.get_bytes(iroh_hash)` path fails because the iroh-blobs store can't read back the blob. Without this fix, federated git clone delivers refs but no content.

## What Changes

- **Diagnose the blob read failure**: Determine whether the issue is a hash mismatch (BLAKE3 of envelope vs iroh-blobs hash), missing blob tags (GC'd before read), or iroh-blobs bao encoding corruption. Add diagnostic logging at the `export_object` failure point.
- **Fix the root cause**: Depending on diagnosis — either align the hash used for lookup, ensure blobs are tagged/protected before federation export, or handle the iroh-blobs error gracefully with a fallback read path.
- **Verify with NixOS VM test**: The existing `federation-git-clone-test` should produce a working tree with file content once fixed.

## Capabilities

### New Capabilities

- `federation-blob-export-fix`: Fix blob store read failures during federation git object export so that `sync_objects` returns commit/tree/blob content alongside refs.

### Modified Capabilities

## Impact

- `crates/aspen-forge/src/git/bridge/exporter.rs` — `export_object` blob lookup
- `crates/aspen-blob/src/store/iroh_store.rs` — `get_bytes` error handling
- `crates/aspen-forge/src/resolver.rs` — `export_git_objects` error path
- `nix/tests/federation-git-clone.nix` — tighten assertion to verify file content
