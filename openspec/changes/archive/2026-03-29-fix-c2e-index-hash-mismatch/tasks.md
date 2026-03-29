## 1. Fix TreeObject sort order (git compatibility)

- [x] 1.1 Add `git_tree_sort_key` helper function to `crates/aspen-forge/src/git/object.rs` that returns a sort key respecting git's mode-aware comparison (append `/` for directory mode `040000`)
- [x] 1.2 Change `TreeObject::new()` to sort using `git_tree_sort_key` instead of `name.cmp()`
- [x] 1.3 Update `test_tree_sorting` in `object.rs` to cover the directory-file prefix overlap case (`foo` dir vs `foo.c` file)
- [x] 1.4 Verify existing tests still pass (sort change is no-op for common entries)

## 2. Fix commit message round-trip

- [x] 2.1 In `converter/import.rs` `parse_git_commit()`, replace `lines.collect::<Vec<_>>().join("\n")` with a byte-offset approach that preserves the raw message after the blank line separator
- [x] 2.2 In `converter/export.rs` `export_commit()`, remove the conditional trailing-newline addition — just emit `commit.message` as-is since it now preserves the original bytes
- [x] 2.3 Apply the same fix to `parse_git_tag` / `export_tag` for tag messages

## 3. Re-key c2e index by SHA-1

- [x] 3.1 In `importer.rs` `import_objects()`, change c2e write key from `hex::encode(content_hash)` to `sha1.to_hex()` — remove content_hash computation entirely from the wave loop
- [x] 3.2 In `resolver.rs` `export_git_objects()`, change c2e scan prefix and lookup to use sha1_hex. Use the `_sha1` return value from `converter.export_object()` instead of `blake3::hash(&data)` for the c2e key
- [x] 3.3 In `resolver.rs` pre-populate block, parse c2e keys as sha1_hex instead of content_hash_hex, and match against a sha1-based have_set

## 4. Fix have_set hash domain

- [x] 4.1 Add `collect_local_sha1_hashes()` in `federation.rs` that scans `forge:hashmap:sha1:{repo}:` and returns `Vec<[u8; 20]>` of SHA-1 hashes
- [x] 4.2 Add helper `sha1_to_have_hash([u8; 20]) -> [u8; 32]` that zero-pads SHA-1 to 32 bytes for the wire format
- [x] 4.3 Change `federation_git.rs` fetch loop to call `collect_local_sha1_hashes()` and convert to `[u8; 32]` via zero-padding for `have_hashes`
- [x] 4.4 In `resolver.rs` `export_git_objects()`, when processing have_set for c2e lookup, truncate 32-byte entries to 20-byte SHA-1 hex for key lookup

## 5. Round-trip integration tests

- [x] 5.1 Add `test_tree_roundtrip_mode_aware_sort` in `crates/aspen-forge/src/git/bridge/tests.rs` — import a tree with directory+file name prefix overlap, export, verify byte-identical output and SHA-1 match
- [x] 5.2 Add `test_commit_roundtrip_message_preservation` — import a commit with multi-paragraph message, export, verify byte-identical content
- [x] 5.3 Add `test_c2e_index_sha1_roundtrip` — import objects, verify c2e entries are keyed by sha1_hex, then simulate export c2e lookup and verify 100% hit rate
- [x] 5.4 Add `test_incremental_federation_sync_dedup` — import objects on a "remote" KV, populate have_set with sha1 hashes, verify exporter skips all known objects

## 6. Cleanup

- [x] 6.1 Remove dead content_hash computation from `import_objects()` wave loop (the `content_start`/`content_hash` lines)
- [x] 6.2 Update the `content_to_local_blake3` field in `FederationImportStats` to `sha1_to_local_blake3` (HashMap<[u8; 20], blake3::Hash>) and update all consumers in `federation_git.rs` `translate_ref_hashes()`
- [x] 6.3 Update napkin with resolution
