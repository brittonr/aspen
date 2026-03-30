## 1. Add remap KV constant and write helper

- [ ] 1.1 Add `KV_PREFIX_REMAP` constant (`forge:remap:`) in `crates/aspen-forge/src/git/bridge/constants.rs`
- [ ] 1.2 Add `write_remap(repo_id, origin_blake3, mirror_blake3)` and `get_remap(repo_id, origin_blake3) -> Option<blake3::Hash>` methods to `HashMappingStore` in `crates/aspen-forge/src/git/bridge/mapping.rs`
- [ ] 1.3 Unit test: `write_remap` + `get_remap` round-trip returns the correct mirror hash
- [ ] 1.4 Unit test: `get_remap` for a non-existent key returns `None`

## 2. Write remap entries during federation import

- [ ] 2.1 In `federation_import_objects` (federation.rs), after each successful `import_objects` call, iterate over returned mappings and write a remap entry for each object whose `SyncObject.envelope_hash` is `Some`. Pair `envelope_hash` (origin BLAKE3) with the mirror BLAKE3 from `ImportResult.mappings`.
- [ ] 2.2 In the convergent retry loop, write remap entries for objects recovered in retry passes (same logic as 2.1)
- [ ] 2.3 Add info-level log summarizing remap entries written per batch (count)
- [ ] 2.4 Add debug-level log when `envelope_hash` is `None` (skip remap write)

## 3. Exporter BFS uses remap index for unresolved BLAKE3

- [ ] 3.1 In `export_commit_dag_collect` (exporter.rs), after `read_object_bytes` fails for a BLAKE3 hash, call `mapping.get_remap(repo_id, blake3)` to try translating origin→mirror BLAKE3. If found, re-read with the mirror hash.
- [ ] 3.2 Add a warn-level log when a BLAKE3 hash is unresolvable (not found directly and no remap entry) instead of silently stopping the walk
- [ ] 3.3 When remap resolves a hash, use the **mirror** BLAKE3 as the key in `to_export` and `visited` sets so subsequent references to the same object are deduplicated correctly
- [ ] 3.4 Unit test: exporter with remap entries walks the full DAG (create a mock repo where objects are stored under mirror hashes but tree entries reference origin hashes)

## 4. Integration test

- [ ] 4.1 Add a test in `crates/aspen-forge/src/git/bridge/tests.rs` that simulates the federated import scenario: create objects with origin BLAKE3 signatures, import them into a mirror (new signatures), write remap entries, then export the DAG and verify all objects are returned
- [ ] 4.2 Run `cargo nextest run -E 'test(/remap/)' --workspace` to confirm tests pass

## 5. Dogfood validation

- [ ] 5.1 Run `nix run .#dogfood-federation -- full` and verify federated git clone succeeds for the full 33K-object repo
- [ ] 5.2 Verify `git log --oneline | wc -l` on the cloned repo matches the origin's commit count
