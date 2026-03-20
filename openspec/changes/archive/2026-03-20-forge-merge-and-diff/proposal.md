## Why

The Forge can store code, track issues, manage patches, and gate merges on CI status — but it can't actually merge anything. There's no operation that takes two trees and produces a merge commit, and no way to see what changed between commits. Without diff and merge, patch review is metadata-only (you see "patch exists" but not "what the patch changed"), and the merge-gating machinery has nothing to execute against. These are the two missing primitives blocking Aspen from developing on its own Forge.

## What Changes

- Tree-to-tree diff engine that compares two `TreeObject`s and produces a structured changeset (added/removed/modified/renamed files with content hunks)
- Commit-to-commit diff that walks parent chains to produce the same output
- Three-way merge that takes a base tree and two branch trees, detects conflicts, and produces a merged tree (or a conflict report)
- Merge commit creation that writes the merged tree + merge commit object to iroh-blobs and advances the target ref through Raft consensus
- `ForgeNode::merge_patch()` high-level operation that ties merge-gating checks (approvals, CI status, branch protection) to the actual merge commit creation
- Inline diff context on patch COBs so reviewers can see hunks without a separate diff call

## Capabilities

### New Capabilities

- `tree-diff`: Structured diff between two trees or commits, producing add/remove/modify/rename entries with optional content-level hunks
- `tree-merge`: Three-way merge of tree objects with conflict detection, producing either a merged tree or a conflict report
- `patch-merge`: End-to-end patch landing that enforces branch protection, creates the merge commit, updates refs via Raft, and closes the patch COB

### Modified Capabilities

- `forge`: ForgeNode gains `merge_patch()` and `diff_commits()` top-level methods; `CobStore` gains merge execution that writes merge commits
- `merge-gating`: MergeChecker is currently check-only; it needs to be wired into the actual merge path so rejections prevent commit creation

## Impact

- **crates/aspen-forge/src/git/**: New `diff.rs` and `merge.rs` modules alongside existing `object.rs` and `store.rs`
- **crates/aspen-forge/src/cob/patch.rs**: Patch state machine gains a `Merged` terminal state with merge commit hash
- **crates/aspen-forge/src/node.rs**: `ForgeNode` gets `merge_patch()` and `diff_commits()` methods
- **crates/aspen-forge/src/protection/merge_checker.rs**: Currently returns pass/fail; needs to be called from the merge path, not just checked externally
- **crates/aspen-forge/src/verified/**: New verified functions for diff entry ordering, conflict detection predicates, three-way merge logic
- **crates/aspen-forge/verus/**: Verus specs for merge correctness invariants (conflict detection completeness, tree integrity after merge)
- No new external dependencies — diff and merge are tree-walking algorithms over existing `TreeObject`/`BlobObject` types
