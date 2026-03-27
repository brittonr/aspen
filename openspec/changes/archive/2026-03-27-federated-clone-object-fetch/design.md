## Context

Federated clone (`fed:` URL) is broken because `federation_import_objects` imports SyncObjects in arrival order. Git objects have strict dependency chains (commits→trees→blobs), and `import_object()` fails when a dependency's SHA-1→BLAKE3 mapping doesn't exist yet. The regular push path already solves this with `import_objects()` (plural) which topologically sorts into waves.

The fix reuses `import_objects()` for federation imports, and extends `ImportResult` to carry per-object hash mappings so ref translation doesn't need extra KV round-trips.

## Goals / Non-Goals

**Goals:**

- Federated clone via `fed:` URL produces a working git repo
- Reuse existing tested topological sort / wave-based import
- `ImportResult` carries per-object `(Sha1Hash, blake3::Hash)` for callers that need hash correlation
- All existing push tests continue to pass

**Non-Goals:**

- Federated push (already works via direct push to remote forge)
- Performance optimization of the federation sync protocol itself
- Fixing the ghost mirror repo (forge identity vs derived ID) — tracked separately

## Decisions

### 1. Extend `ImportResult` with per-object mappings

Add `mappings: Vec<(Sha1Hash, blake3::Hash)>` to `ImportResult`. Each entry records the SHA-1 and BLAKE3 of a successfully imported object.

**Why not a HashMap?** Vec is simpler, preserves import order, and callers that need lookup can build their own map. The federation code needs to iterate all commits anyway to match SHA-1s against ref entries.

**Where populated:** In `import_objects()`, after each wave completes, the wave's `(sha1, blake3)` pairs from `import_object_store_blob()` are appended. Skipped objects (already had mappings) still get their existing mapping added so the vec is complete.

### 2. Convert SyncObjects to `import_objects()` input format

`federation_import_objects` converts each `SyncObject` to `(Sha1Hash, GitObjectType, Vec<u8>)`:

- Compute SHA-1 from `format!("{type} {len}\0{content}")`
- Parse `object_type` string to `GitObjectType` enum
- Reconstruct full git bytes (header + content)

Then pass the whole batch to `import_objects()`. This gets topological sorting, wave parallelism, and batched hash mapping writes for free.

### 3. Build `content_to_local_blake3` from `ImportResult.mappings`

After `import_objects()` returns, iterate `result.mappings` to build the content→BLAKE3 map needed by `translate_ref_hashes()`. For each mapping `(sha1, blake3)`, the content hash is `blake3::hash(&obj.data)` (raw content without header). Keep a parallel vec of content hashes during the SyncObject→import conversion phase, indexed by SHA-1, so the post-import lookup is O(n).

### 4. Keep `translate_ref_hashes` unchanged

The ref translation logic (SHA-1 matching between fetched refs and imported commits) is correct. It just needs a properly populated `content_to_local_blake3` map, which the new flow provides.

## Risks / Trade-offs

**[Risk] SyncObjects with unknown types silently dropped** → Already the case today. The conversion filters to commit/tree/blob only. Tags could be added later.

**[Risk] Large federation syncs hitting `MAX_IMPORT_BATCH_SIZE`** → `import_objects()` rejects batches over the limit. For repos exceeding this, the sync already paginates (max 10 rounds × 1000 objects). Each round would be a separate `import_objects()` call. Cross-round dependencies (tree in round 1 references blob from round 2) would still fail — but this is the same constraint as the current code and only matters for very large repos.

**[Trade-off] `ImportResult` size increase** → Adding a vec of `(Sha1Hash, blake3::Hash)` pairs (52 bytes each) is negligible compared to the object data being imported. For 1000 objects, ~50KB.
