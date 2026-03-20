## Why

The merge primitives exist (tree diff, three-way merge, `ForgeNode::merge_patch()` with CAS retry, branch protection, CI commit status) but they aren't wired end-to-end. The RPC handler calls `cobs.merge_patch()` (COB transition only) instead of `ForgeNode::merge_patch()` (the full workflow). The CLI requires callers to supply a pre-computed merge commit hash. The web UI shows patches but has no merge/approve actions. There's no way to actually merge a patch through any user-facing interface without manually computing the merge yourself.

## What Changes

- **Fix RPC handler**: `handle_merge_patch` calls `ForgeNode::merge_patch()` instead of `cobs.merge_patch()`, delegating merge commit creation, branch protection enforcement, and ref advancement to the server
- **Merge strategy selection**: Add `MergeStrategy` enum (merge commit, fast-forward-only, squash) to the RPC request and `ForgeNode::merge_patch()`
- **Squash merge**: Implement squash as a single-parent commit with the merged tree, collapsing patch history
- **Update CLI**: `aspen-cli patch merge` no longer requires `--merge-commit`; accepts `--strategy` flag instead
- **Merge check endpoint**: New `ForgeCheckMerge` RPC that dry-runs protection checks and conflict detection without committing, returning mergeability status
- **Web UI merge actions**: Merge button on patch detail page with strategy selector, approval button, mergeability status indicator showing CI and approval state
- **Diff view in web UI**: Show file-level diff between patch head and target branch on the patch detail page

## Capabilities

### New Capabilities

- `merge-strategy`: Merge strategy selection (merge commit, fast-forward, squash) and server-side merge execution
- `merge-check`: Pre-merge dry-run endpoint for checking mergeability without side effects
- `patch-web-actions`: Web UI merge button, approval button, strategy picker, and diff view on patch pages

### Modified Capabilities

- `patch-merge`: The `ForgeMergePatch` RPC changes from COB-only transition to full server-side merge workflow; `merge_commit` field replaced by `strategy`

## Impact

- **crates/aspen-client-api**: `ForgeMergePatch` request fields change (`merge_commit` → `strategy`), new `ForgeCheckMerge` request/response types
- **crates/aspen-forge**: New `MergeStrategy` enum, `merge_patch()` gains strategy parameter, new `check_merge()` method
- **crates/aspen-forge-handler**: `handle_merge_patch` rewritten to call `ForgeNode::merge_patch()`
- **crates/aspen-cli**: `PatchMergeArgs` drops `--merge-commit`, adds `--strategy`
- **crates/aspen-forge-web**: New routes and templates for merge/approve actions, diff display
- **BREAKING**: `ForgeMergePatch` RPC field change breaks existing callers (none in production — only tests)
