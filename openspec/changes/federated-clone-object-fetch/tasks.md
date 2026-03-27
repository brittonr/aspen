## 1. Extend ImportResult

- [x] 1.1 Add `mappings: Vec<(Sha1Hash, blake3::Hash)>` field to `ImportResult` in `crates/aspen-forge/src/git/bridge/importer.rs`
- [x] 1.2 Populate `mappings` in `import_objects()` — append each wave's `(sha1, blake3)` pairs after blob storage, including skipped objects that already had mappings
- [x] 1.3 Update all callers of `import_objects()` to handle the new field (git_bridge push handler, chunked push handler)

## 2. Rewrite federation_import_objects

- [x] 2.1 Convert SyncObjects to `(Sha1Hash, GitObjectType, Vec<u8>)` format — compute SHA-1, parse object type, reconstruct full git bytes with header
- [x] 2.2 Replace the sequential `import_object()` loop with a single `import_objects()` call
- [x] 2.3 Build `content_to_local_blake3` map from `ImportResult.mappings` — track content hashes (blake3 of raw content) during conversion, correlate with mappings by SHA-1 after import

## 3. Update sync_from_origin

- [x] 3.1 Update `sync_from_origin()` in `federation_git.rs` to use the new `federation_import_objects` return type for building the ref translation map
- [x] 3.2 Verify `translate_ref_hashes` still works correctly with the new `FederationImportStats`

## 4. Tests

- [x] 4.1 Unit test: `import_objects()` with reverse-dependency-order input verifies all objects imported and `mappings` vec is complete
- [x] 4.2 Unit test: `import_objects()` with partially-known objects verifies `mappings` includes both new and existing entries
- [x] 4.3 Integration test: end-to-end federated clone via `fed:` URL produces a repo with correct refs and fetchable objects (or a focused test of `federation_import_objects` with SyncObjects in random order)
- [x] 4.4 Verify existing git bridge push tests pass without modification
