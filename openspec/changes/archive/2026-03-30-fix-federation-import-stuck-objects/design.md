## Context

Federation sync pulls git objects from an origin cluster and imports them into a local mirror via `federation_import_objects`. The import uses a convergent retry loop: blobs (no deps) import on pass 1, trees on pass 2, commits on pass 3. Each pass calls `import_objects` which uses wave-based topological sorting to resolve dependencies.

The problem surfaces at the dependency resolution step. `import_objects` calls `extract_tree_dependencies` to get the SHA-1 hashes that a tree references. These SHA-1s come from the raw tree content — which was produced by the *origin* cluster. When the local import stores a blob, it may re-serialize it and get a different SHA-1 (though for blobs this doesn't happen). For trees, the re-serialization can change entry ordering or mode formatting, producing a different SHA-1 than what the origin had.

The convergent loop stores origin SHA-1 → BLAKE3 mappings via `re_sha1_to_origin` after each pass. But this mapping is only populated when `origin_sha1` differs from the re-serialized SHA-1. If a tree entry references a sub-object by the origin SHA-1 that was never stored as a mapping, the dependency check fails (`has_sha1` returns false), and the tree stays stuck.

Evidence from dogfood logs: 2,902 objects stuck after convergence, all trees/commits whose dependencies reference unmapped origin SHA-1s.

## Goals / Non-Goals

**Goals:**

- All ~34,000 objects from federation sync import successfully (zero stuck)
- Federated `git clone` completes without "Could not read" errors
- No regression for direct (non-federation) git push/fetch

**Non-Goals:**

- Optimizing import speed (convergent loop is already fast enough)
- Changing the federation sync wire protocol
- Pack file support or delta compression

## Decisions

### Decision 1: Pre-populate origin SHA-1 mappings before import

**Choice**: Before calling the convergent import loop, scan all incoming `SyncObject`s that carry `origin_sha1`. Compute the git SHA-1 from their raw content. For each object, store a mapping from `origin_sha1` → the computed SHA-1. This way, when `extract_tree_dependencies` finds a SHA-1 in a tree entry, the mapping is already present.

**Why**: The convergent loop relies on `has_sha1` to check whether a dependency is available. If origin SHA-1 mappings are populated up front, the dependency check succeeds on the first pass instead of failing and retrying.

**Alternative**: Store both origin and local SHA-1 during `import_objects`. More invasive — requires changing the importer interface.

### Decision 2: Raw-bytes import to prevent SHA-1 drift

**Choice**: Pass the original raw git object bytes (header + content) from the `SyncObject` directly into the importer, skipping re-serialization. If `import_single_object` stores the exact bytes received from the origin, the resulting SHA-1 matches the origin's SHA-1. No drift, no mapping mismatch.

**Why**: The root cause is SHA-1 drift from re-serialization. Eliminating re-serialization eliminates the root cause. The `SyncObject.data` field already contains the raw content; adding the git header (`"type size\0"`) produces valid git object bytes.

**Alternative**: Normalize tree entry sort order to match git's sort. Already done (see napkin: TreeObject sort fix). But commit/tag message byte preservation and edge cases make this fragile.

### Decision 3: Final-pass retry for genuinely stuck objects

**Choice**: After the convergent loop stalls (no progress), do one more pass with relaxed dependency checking: import objects even if some dependencies are missing, since the missing deps may be among the stuck objects themselves (circular dependency at the mapping level, not the data level).

**Why**: Some objects are stuck because their dependencies are also stuck — both waiting for each other's mapping. A single pass that ignores missing-dep errors breaks the deadlock.

**Risk**: Could import objects with dangling references. Mitigate by only doing this for objects that passed content validation (SHA-1 matches).

## Risks / Trade-offs

- **[Pre-populate overhead]** → Scanning all objects twice (once for mapping, once for import). ~34K objects × hash computation = ~50ms. Acceptable.
- **[Raw-bytes assumption]** → Assumes `SyncObject.data` is valid git content. Already validated by the origin's exporter.
- **[Relaxed final pass]** → Could store objects with missing sub-objects. The convergent loop already handles partial success, and the DAG walk at export time skips missing objects.

## Open Questions

1. Should the raw-bytes import be the default path for all federation imports, or only a fallback when the convergent loop stalls?
