## 1. Types and Constants

- [x] 1.1 Add `DiffEntry`, `DiffKind`, `DiffResult` types to new `crates/aspen-forge/src/git/diff.rs`
- [x] 1.2 Add `TreeMergeResult`, `MergeConflict`, `ConflictKind` types to new `crates/aspen-forge/src/git/merge.rs`
- [x] 1.3 Add `Merged { merge_commit: [u8; 32], merged_by: PublicKey, merged_at_ms: u64 }` variant to `PatchState` in `cob/patch.rs`
- [x] 1.4 Add `MAX_DIFF_ENTRIES`, `MAX_DIFF_BLOB_SIZE`, `MAX_MERGE_DEPTH`, `MAX_MERGE_CONFLICTS` to `constants.rs`
- [x] 1.5 Add `MergeConflicts` and `MergeDepthExceeded` variants to `ForgeError` in `error.rs`

## 2. Verified Merge Functions

- [x] 2.1 Create `crates/aspen-forge/src/verified/merge.rs` with `classify_three_way(base, ours, theirs) -> ThreeWayClass` pure function
- [x] 2.2 Add `is_convergent_change(ours, theirs) -> bool` and `is_conflict(base, ours, theirs) -> bool` to verified module
- [x] 2.3 Re-export from `src/verified/mod.rs`
- [x] 2.4 Write unit tests for all classification cases (both-modified, modify-delete, both-added, convergent, etc.)

## 3. Verus Specifications

- [x] 3.1 Create `crates/aspen-forge/verus/merge_spec.rs` with Verus specs for `classify_three_way`, `is_conflict`, `is_convergent_change`
- [x] 3.2 Prove symmetry: `is_conflict(b, o, t) == is_conflict(b, t, o)` for all inputs
- [x] 3.3 Prove convergent changes never conflict
- [x] 3.4 Prove unchanged entries never conflict
- [x] 3.5 Prove classification is exhaustive (every input combination maps to exactly one action)
- [x] 3.6 Add merge invariants to `verus/lib.rs` documentation

## 4. Tree Diff

- [x] 4.1 Implement `diff_trees(store, tree_a, tree_b, opts) -> DiffResult` with sorted two-pointer walk in `git/diff.rs`
- [x] 4.2 Handle recursive subdirectory diffing with early-out on matching hashes
- [x] 4.3 Handle mode-only changes (same content hash, different mode)
- [x] 4.4 Implement optional content loading with `MAX_DIFF_BLOB_SIZE` guard
- [x] 4.5 Enforce `MAX_DIFF_ENTRIES` truncation with truncation flag in result
- [x] 4.6 Implement `diff_commits(store, commit_a, commit_b, opts) -> DiffResult` wrapper
- [x] 4.7 Ensure result entries are sorted lexicographically by path
- [x] 4.8 Re-export from `git/mod.rs`

## 5. Tree Merge

- [x] 5.1 Implement `merge_trees(store, base, ours, theirs, depth) -> TreeMergeResult` using `classify_three_way` for each entry
- [x] 5.2 Handle recursive subdirectory merging with `MAX_MERGE_DEPTH` guard
- [x] 5.3 Handle unchanged subtree skip (all three have same hash)
- [x] 5.4 Accumulate conflicts with `MAX_MERGE_CONFLICTS` cap
- [x] 5.5 Write merged tree to blob store via `GitBlobStore::create_tree()` on success
- [x] 5.6 Re-export from `git/mod.rs`

## 6. Patch Merge Operation

- [x] 6.1 Add `MergeChecker` field to `ForgeNode` (constructed from KV store in `ForgeNode::new()`)
- [x] 6.2 Implement `ForgeNode::merge_patch(repo_id, patch_id, custom_message)` that resolves patch, checks protection, merges trees, creates commit, advances ref
- [x] 6.3 Implement fast-forward detection: if target ref commit equals patch base, skip three-way merge and advance ref directly
- [x] 6.4 Implement CAS retry loop (up to 3 attempts) when `refs.compare_and_swap()` fails due to concurrent push
- [x] 6.5 Transition patch COB to `Merged` state after successful ref update
- [x] 6.6 Broadcast `RefUpdate` gossip announcement after successful merge
- [x] 6.7 Auto-generate merge commit message `"Merge patch '<title>'"` with custom override

## 7. Patch State Machine Update

- [x] 7.1 Add `Merged` state handling to patch resolution in `cob/patch.rs` (apply `Merge` operation → terminal state)
- [x] 7.2 Reject further operations (comment, revise, close) on patches in `Merged` state
- [x] 7.3 Add `CobOperation::Merge` variant if not already present, with merge commit hash field

## 8. ForgeNode Diff Method

- [x] 8.1 Add `ForgeNode::diff_commits(commit_a, commit_b, include_content) -> DiffResult` convenience method
- [x] 8.2 Add `ForgeNode::diff_patch(repo_id, patch_id) -> DiffResult` that diffs the patch head against the target branch

## 9. Tests

- [x] 9.1 Unit tests for tree diff: identical trees, add/remove/modify, nested dirs, mode changes, truncation
- [x] 9.2 Unit tests for tree merge: clean merge, both-modified conflict, convergent, add-one-side, modify-delete, recursive
- [x] 9.3 Unit tests for merge resource bounds: depth limit, conflict cap
- [x] 9.4 Integration test for `merge_patch()`: successful merge with protection checks, merge commit in history, patch state transition
- [x] 9.5 Integration test for merge rejection: failing CI blocks merge, insufficient approvals blocks merge
- [x] 9.6 Integration test for CAS retry: concurrent push forces re-merge
- [x] 9.7 Integration test for fast-forward merge: no merge commit created, ref advanced directly
- [x] 9.8 Integration test for `diff_commits()` end-to-end: create two commits, diff them, verify entries
