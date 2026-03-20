## Context

Forge stores Git objects (blobs, trees, commits) as BLAKE3-addressed `SignedObject<GitObject>` in iroh-blobs, with refs going through Raft consensus. `TreeObject` contains sorted `Vec<TreeEntry>` where each entry has `mode`, `name`, and `hash`. `CommitObject` has a tree hash, parent hashes, author, and message. `GitBlobStore` provides `get_tree()`, `get_blob()`, `get_commit()`, `create_tree()`, `commit()`, and `store_blob()`.

Branch protection (`MergeChecker`) already checks CI status and approval counts but returns pass/fail — nothing calls it as part of an actual merge operation. The `Patch` COB has states (`Open`, `Closed`) but no `Merged` terminal state and no mechanism to record which merge commit landed the patch.

All tree walking and object reading is async because objects live in iroh-blobs. Trees are sorted by name, which makes two-pointer comparison straightforward.

## Goals / Non-Goals

**Goals:**

- Tree-to-tree diff producing structured `DiffEntry` records (add/remove/modify with content hunks for blobs)
- Three-way tree merge (base + ours + theirs) producing either a merged `TreeObject` or a conflict report
- `ForgeNode::merge_patch()` that enforces branch protection, creates the merge commit, advances the ref, and transitions the patch to `Merged`
- Verified pure functions for diff/merge predicates in `src/verified/`, with Verus specs
- Recursive tree handling (subdirectories diffed/merged by walking into child trees)

**Non-Goals:**

- Line-level diff (content hunks are whole-blob for now; line-level is a display concern for TUI/CLI later)
- Rename detection (requires content similarity heuristics — add later)
- Rebase (different operation model; merge-only for now)
- Automatic conflict resolution strategies (always report conflicts; user resolves)
- Squash merge (single-parent commit that flattens history — separate feature)

## Decisions

### 1. Diff as sorted-entry two-pointer walk

Both trees are sorted by name. Walk both entry lists with two pointers:

- Left-only → `Removed`
- Right-only → `Added`
- Both present, same hash → skip (unchanged)
- Both present, different hash, both directories → recurse and prefix path
- Both present, different hash, both files → `Modified` (fetch both blobs for content)
- Both present, different hash, mode changed → `Modified` with mode change flag

**Why not hash the whole tree first?** We already have the tree hashes for early-out (if equal, skip entirely), but the entry-level walk is needed to produce per-file results. The two-pointer approach is O(n) where n is the number of entries at each tree level.

**Alternative considered:** Flatten both trees to full paths first, then diff the flat lists. Rejected because it forces fetching all subtrees upfront even when most are unchanged. The recursive approach skips entire subtrees when their root hashes match.

### 2. Three-way merge via base-ours-theirs entry comparison

For each entry name present in any of the three trees:

- Unchanged in both sides (same hash as base) → keep as-is
- Changed only in ours → take ours
- Changed only in theirs → take theirs
- Changed in both to the same hash → take either (convergent)
- Changed in both to different hashes → conflict
- Added only in ours → include
- Added only in theirs → include
- Added in both with same hash → include
- Added in both with different hashes → conflict
- Removed in one, unchanged in other → remove
- Removed in one, modified in other → conflict

For directory entries: recurse into the subtrees using the same three-way logic.

**Why not use a CRDT merge?** Trees are point-in-time snapshots, not append-only logs. Three-way merge with explicit conflict detection is the standard Git model and what users expect.

### 3. Merge commit creation as atomic Raft operation

`merge_patch()` does:

1. Resolve current patch state → get head commit, target ref
2. `MergeChecker::check_merge_allowed()` → fail fast on protection violations
3. Read base tree (merge-base = target ref's current commit), ours tree (target), theirs tree (patch head)
4. Three-way merge → fail on conflicts
5. `create_tree()` → write merged tree to iroh-blobs
6. `commit()` → write merge commit with two parents (target commit + patch head commit)
7. `refs.compare_and_swap()` → atomically advance target ref (CAS prevents races)
8. Transition patch COB to `Merged` state with merge commit hash

Steps 7-8 use CAS on the ref. If another push lands between step 2 and step 7, the CAS fails and the merge must be retried. This prevents lost updates without holding locks across async operations.

**Alternative considered:** Locking the ref for the duration of the merge. Rejected because it violates the async safety rule (never hold locks across `.await`) and creates deadlock risk under concurrent merges.

### 4. DiffEntry and MergeConflict as value types

```rust
pub struct DiffEntry {
    pub path: String,           // full path from repo root
    pub kind: DiffKind,         // Added, Removed, Modified
    pub old_mode: Option<u32>,
    pub new_mode: Option<u32>,
    pub old_hash: Option<[u8; 32]>,
    pub new_hash: Option<[u8; 32]>,
    pub old_content: Option<Vec<u8>>,  // populated on request
    pub new_content: Option<Vec<u8>>,  // populated on request
}

pub struct TreeMergeResult {
    pub tree: Option<TreeObject>,       // Some if clean merge
    pub conflicts: Vec<MergeConflict>,  // non-empty if conflicts
}

pub struct MergeConflict {
    pub path: String,
    pub kind: ConflictKind,  // BothModified, ModifyDelete, BothAdded
    pub base: Option<[u8; 32]>,
    pub ours: Option<[u8; 32]>,
    pub theirs: Option<[u8; 32]>,
}
```

Content (`old_content`/`new_content`) is optional and populated lazily — callers that only need structural diff (which files changed?) skip the blob fetches.

### 5. Verified functions for merge predicates

Pure functions in `src/verified/merge.rs`:

- `classify_three_way(base: Option<Hash>, ours: Option<Hash>, theirs: Option<Hash>) -> ThreeWayClass` — determines the merge action for a single entry
- `is_convergent_change(ours: Hash, theirs: Hash) -> bool` — both sides changed to the same thing
- `is_conflict(base: Option<Hash>, ours: Option<Hash>, theirs: Option<Hash>) -> bool`

These are deterministic, no-I/O, testable independently. Verus specs prove:

- Conflict detection is symmetric: `is_conflict(b, o, t) == is_conflict(b, t, o)`
- Convergent changes never conflict
- Unchanged entries never conflict
- Classification is exhaustive (every combination maps to exactly one action)

### 6. Patch state machine gains `Merged` state

`PatchState` adds `Merged { merge_commit: [u8; 32], merged_by: PublicKey, merged_at_ms: u64 }`. This is a terminal state — no further transitions from `Merged`. The merge commit hash links the COB to the actual Git history, closing the loop between collaboration metadata and repository content.

## Risks / Trade-offs

**[Large tree performance]** → Recursive async tree walking could be slow for trees with many nested directories. Mitigation: early-out when subtree hashes match (skip entire unchanged directories). For the self-hosting use case, Aspen's own repo is moderate-sized; this is acceptable. Parallelizing subtree fetches is a future optimization.

**[CAS retry under contention]** → If many merges target the same branch simultaneously, CAS retries could pile up. Mitigation: merge-to-main is typically serialized by human review cadence. If contention becomes real, add a bounded retry (3 attempts) with backoff.

**[No merge-base computation]** → True merge-base requires walking commit history to find the common ancestor. For v1, the merge-base is the target branch's current commit at merge time (fast-forward-like merge). Full merge-base computation (LCA of commit DAG) is a follow-up.

**[Blob content in memory]** → `DiffEntry` can hold both old and new blob content. For large files this could be expensive. Mitigation: content is optional and loaded lazily. Callers that just need the structural diff don't pay this cost. Add a `MAX_DIFF_BLOB_SIZE` constant (1 MB) to skip content loading for oversized blobs.

## Open Questions

- Should `merge_patch()` support fast-forward merges (when the target ref is an ancestor of the patch head)? Leaning yes — detect it and skip the three-way merge, just advance the ref.
- Should the merge commit message be auto-generated ("Merge patch 'title'") or caller-provided? Leaning auto-generated with optional override.
