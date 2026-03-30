## Context

Federation sync transfers git objects between clusters in batches. The exporter walks the commit DAG (BFS from ref heads), collects objects up to a limit (5000 per batch), and sends them. The importer reconstructs git objects and stores SHA-1→BLAKE3 hash mappings.

The problem: `collect_dag_blake3` in `exporter.rs` stops at the limit mid-traversal. A tree at position 4999 may reference blobs at positions 5001+. The importer needs those blobs to reconstruct the tree's SHA-1 (since SHA-1 depends on children's SHA-1s). Without them, import fails with "hash mapping not found".

Current two-pass mitigation (blobs first, then trees+commits in `federation_import_objects`) only helps when blobs and trees are in the *same* batch. Cross-batch references remain broken.

## Goals / Non-Goals

**Goals:**

- Every federation sync batch is self-contained: all tree→blob and commit→tree references resolve within the batch plus the receiver's `have_set`
- Large repos (33K+ objects) transfer correctly across multiple batches
- No regression in sync performance for repos that fit in a single batch

**Non-Goals:**

- Streaming/incremental git clone protocol (full packfile generation)
- Shallow clone or partial clone support
- Changing the federation wire protocol (SyncObject format stays the same)

## Decisions

### 1. Dependency closure in the exporter, not forward-reference tolerance in the importer

**Decision**: Enforce closure at the exporter so batches are always importable.

**Alternatives considered**:

- **Forward-reference tolerant importer**: Queue unresolved objects and retry after all batches arrive. Adds complexity (persistent retry queues, potential deadlocks if the exporter never sends the missing object) and doesn't compose well with incremental sync where the remote may never get subsequent batches.
- **Two-phase commit**: Collect all objects first, then send. Requires unbounded memory on the exporter for large repos.

**Rationale**: The exporter has full DAG knowledge. Enforcing closure there is a local property check — no distributed coordination needed. The importer stays simple.

### 2. Closure enforcement via post-collection filtering

**Decision**: After `collect_dag_blake3` returns its set of objects, compute the transitive dependency closure. If any object's dependencies are missing (not in the batch and not in `known_blake3`), remove that object and its dependents. This trims the batch to a closed subset.

**Implementation**: Track parent→child edges during collection. After collection, walk edges to verify all targets are present or known. Remove orphans. This naturally respects the limit — the batch may shrink below the limit, and `has_more` stays true so the caller fetches more batches.

**Alternative considered**: Stop the BFS as soon as adding the next object would break closure. Harder to implement because BFS explores breadth-first (commits before their trees), so you don't know an object's dependencies until you read it.

### 3. Retry pass in the importer as belt-and-suspenders

**Decision**: After the primary import pass, retry failed objects once. This catches edge cases where import order within a batch matters (e.g., two trees referencing each other through subtrees in the same batch).

**Rationale**: Cheap insurance. The exporter closure guarantees should make this a no-op in practice, but federation sync involves serialization, network, and concurrent batch assembly — defense in depth.

### 4. Integration test with multi-batch forced scenario

**Decision**: Add a test in `crates/aspen-forge/src/git/bridge/` (or `resolver.rs` tests) that creates a repo with >5000 objects (blobs + trees + commits), exports via the resolver with a limit that forces 2+ batches, and verifies every object round-trips correctly.

**Alternative**: Test in `tests/` as a full cluster integration test. Too heavyweight — the bug is in the DAG walk, testable with in-memory KV.

## Risks / Trade-offs

- **Smaller effective batches**: Closure trimming may reduce batch size below the limit, increasing round-trips. Acceptable: correctness > throughput, and the extra round-trips only occur for large repos.
- **Exporter CPU**: Computing transitive closure adds O(n) work per batch where n = objects in batch. Negligible compared to KV reads and network transfer.
- **Edge case: circular tree references**: Git doesn't allow them, but if corrupt data exists, closure computation could loop. Mitigation: bounded iteration (existing `MAX_DAG_TRAVERSAL_DEPTH`).
