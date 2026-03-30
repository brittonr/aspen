## 1. Pre-populate origin SHA-1 mappings

- [ ] 1.1 In `federation_import_objects`, before the convergent loop, iterate all `SyncObject`s that have `origin_sha1.is_some()`. For each, compute the git SHA-1 from `"type len\0" + data`, then call `mapping.store_batch` to write `origin_sha1 → computed_blake3` where `computed_blake3 = blake3::hash(data)`. This ensures `has_sha1(origin_sha1)` returns true during dependency resolution.
- [ ] 1.2 Also store the computed SHA-1 (from raw content) → BLAKE3 mapping for each object, so trees referencing sub-objects by their re-serialized SHA-1 also find mappings.
- [ ] 1.3 Add a counter for pre-populated mappings and log it at info level.

## 2. Raw-bytes import path

- [ ] 2.1 In `federation_import_objects`, build the `import_objects` input tuples using the original `SyncObject.data` bytes with a prepended git header (`"type len\0"`), instead of relying on re-serialization. The SHA-1 passed to `import_objects` should be computed from these exact bytes.
- [ ] 2.2 Skip the `re_sha1_to_origin` tracking for objects whose computed SHA-1 matches their `origin_sha1` (no drift, no remap needed).
- [ ] 2.3 Verify that `import_objects` stores the raw bytes as-is via `store_blob` / blob store write, without re-parsing and re-serializing the tree/commit content.

## 3. Final retry pass for stuck objects

- [ ] 3.1 After the convergent loop stalls (newly_imported == 0, remaining > 0), add a final pass that calls `import_objects` with the stuck objects but sets a flag to skip dependency checking. Objects whose SHA-1 validates against their content are safe to store even if dependency mappings are incomplete.
- [ ] 3.2 If the final pass imports any objects, log the count and continue the convergent loop (the newly imported objects may unblock others).
- [ ] 3.3 If the final pass makes no progress, log the remaining stuck objects at warn level with their SHA-1 and dependency list.

## 4. Tests

- [ ] 4.1 Unit test: pre-populated origin SHA-1 mappings are visible to `has_sha1` before import
- [ ] 4.2 Unit test: raw-bytes import produces SHA-1 matching the input bytes (no drift)
- [ ] 4.3 Run `dogfood-federation -- full` end-to-end: federated clone succeeds, zero "Could not read" errors, DAG integrity diagnostic shows stored == reachable
