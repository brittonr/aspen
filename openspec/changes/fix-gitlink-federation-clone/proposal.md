## Why

Federation clone fails with "bad tree object" on any repo containing git submodules. Gitlink entries (mode 160000) were silently dropped during tree import, causing SHA-1 mismatch when trees are re-exported. This blocks the federated dogfood pipeline on the Aspen workspace, which has submodules for `cloud-hypervisor` and `iroh-proxy-utils`.

## What Changes

- Preserve gitlink entries during tree import instead of skipping them, storing the raw SHA-1 zero-padded in the 32-byte BLAKE3 hash field
- Export gitlink entries using stored SHA-1 directly, bypassing the BLAKE3→SHA-1 mapping lookup
- Skip gitlink entries during DAG traversal (they reference commits in external repositories)
- Add `TreeEntry` helpers: `gitlink()`, `is_gitlink()`, `gitlink_sha1_bytes()`

## Capabilities

### New Capabilities

- `gitlink-roundtrip`: Correct import, storage, and export of gitlink (submodule) tree entries through the Forge git bridge

### Modified Capabilities

- `federated-git-clone`: Federation clone now works for repos containing submodules

## Impact

- `crates/aspen-forge/src/git/object.rs`: New methods on `TreeEntry`
- `crates/aspen-forge/src/git/bridge/converter/import.rs`: Tree parser preserves gitlinks
- `crates/aspen-forge/src/git/bridge/converter/export.rs`: Tree exporter handles gitlinks
- `crates/aspen-forge/src/git/bridge/exporter.rs`: DAG walk skips gitlink deps
