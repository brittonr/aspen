## Context

Federation git sync transfers objects in batches of 2,000 over QUIC. A full repo (33,807 objects for Aspen itself) takes ~17 rounds. Each batch is imported immediately via `federation_import_objects` → `GitImporter::import_objects`, which does a single topological sort into waves and processes them sequentially.

The problem: `import_objects` treats any dependency not in the current input set as "external" (assumed already imported). When `import_object_store_blob` → `converter.import_tree` runs, it resolves each child SHA-1 to a BLAKE3 mapping via `HashMappingStore`. If the mapping doesn't exist (the blob was in a different batch and its import failed, or the dependency chain crosses batch boundaries in an unlucky way), the tree import fails. The `?` operator in wave processing propagates this error and aborts the entire `import_objects` call.

A post-sync retry pass already exists — it calls `federation_import_objects` with all accumulated objects. But `federation_import_objects` internally does Pass 1 (blobs) → Pass 2 (trees+commits) → Pass 3 (retry on Pass 2 failure). Each pass calls `import_objects` once. If a tree fails because a subtree it depends on also failed (cascading failure from a missing blob mapping), the single retry isn't enough. 2,902 objects remain unmapped, including the HEAD commit.

## Goals / Non-Goals

**Goals:**

- 100% object import for repos where all objects are present across the collected batches
- No regression in import speed for the common case (most objects import on first pass)
- Structured progress reporting so operators can tell if convergence is stalling

**Non-Goals:**

- Handling genuinely missing objects (objects not exported by the origin) — that's a different bug
- Changing the batch-based sync protocol itself (the convergent retry is a post-collection fix)
- Parallelizing the convergent loop across passes (sequential is fine; each pass is internally parallel via waves)

## Decisions

### 1. Fixed-point convergent loop in `federation_import_objects`

**Choice:** Replace the current blob→non-blob→retry-on-failure three-pass structure with a convergent loop. Each iteration calls `import_objects` with all objects that still lack SHA-1 mappings. Loop terminates when an iteration imports zero new objects (no progress) or all objects are mapped.

**Rationale:** The root cause is cascading dependency failures across batch boundaries. A tree fails → its parent tree fails → the commit fails. Each convergent pass resolves one more layer of the dependency chain. For a repo with depth-N dependency chains split across batches, the loop converges in at most N passes. Typical git repos have depth 3-5 (blob → tree → commit), so 2-4 passes suffice.

**Alternative:** Fix `import_objects` to do internal multi-wave retry. Rejected because `import_objects` is also used by the git push path, where all dependencies are always present in a single call. Adding retry logic there would add complexity and latency to the common push case.

### 2. Partial-success semantics in `import_objects`

**Choice:** Change wave processing so a single object failure collects the error and continues with remaining objects in the wave (and subsequent waves). Return both the successfully imported mappings AND the list of failures in `ImportResult`.

**Rationale:** Currently `result?` in the wave loop aborts on first error. This means one bad tree kills the entire import of thousands of objects — including independent objects that could succeed. Partial success lets each convergent pass make maximum progress.

**Implementation:** Add `failures: Vec<(Sha1Hash, String)>` to `ImportResult`. In the wave loop, collect errors per-object instead of propagating. After all waves, if failures is non-empty, log a warning but return Ok with the partial result.

**Alternative:** Keep `import_objects` strict and do the retry entirely in `federation_import_objects` by re-invoking with progressively smaller object sets. Rejected because it multiplies the overhead of re-checking `has_sha1` for all already-imported objects each pass, and makes the convergent loop harder to reason about.

### 3. Convergence bound

**Choice:** Cap the convergent loop at 10 iterations. If objects remain after 10 passes, log an error with the count and SHA-1 list of stuck objects. This prevents infinite loops from genuinely broken object graphs (cycles shouldn't exist in git, but defensive coding).

**Rationale:** Git DAG depth rarely exceeds 5 for tree structures. 10 gives comfortable headroom. The cap is a safety bound, not an expected limit.

### 4. Filter-then-retry, not retry-all

**Choice:** Each convergent pass filters to only objects without existing SHA-1 mappings (check `has_sha1` per object). Objects successfully imported in prior passes are skipped cheaply.

**Rationale:** `import_objects` already checks `has_sha1` and skips mapped objects. But rebuilding `PendingObject` structs and running topological sort on 33k objects when only 2,900 need retrying is wasteful. Pre-filtering reduces the set each iteration.

## Risks / Trade-offs

**[Convergence stall]** → If an object's dependency is genuinely missing (not in any batch), the loop runs all 10 iterations then stops. Mitigation: log the stuck objects with their missing dependencies so the operator can diagnose.

**[Performance on healthy imports]** → The convergent loop adds one extra `has_sha1` scan after the first pass to confirm everything imported. For the common case (all objects import in pass 1), this is a single cheap scan with zero retries. No measurable overhead.

**[Memory]** → Keeping all 33k SyncObjects in memory for the convergent loop is already done today (`all_git_objects` in `handle_federation_git_fetch`). No change.
