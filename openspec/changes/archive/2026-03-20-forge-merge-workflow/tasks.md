## 1. Core Types and Strategy

- [x] 1.1 Add `MergeStrategy` enum to `aspen-forge` with `MergeCommit`, `FastForwardOnly`, `Squash` variants and `FromStr`/`Display` impls
- [x] 1.2 Add `FastForwardNotPossible` variant to `ForgeError`
- [x] 1.3 Update `ForgeNode::merge_patch()` to accept `MergeStrategy` parameter
- [x] 1.4 Implement fast-forward-only strategy: fail with `FastForwardNotPossible` when target has diverged
- [x] 1.5 Implement squash strategy: single-parent commit with merged tree, auto-generate message from patch title
- [x] 1.6 Unit tests for each strategy in `node.rs` (existing tests updated + new squash/ff-only tests)

## 2. Merge Check Endpoint

- [x] 2.1 Add `check_merge()` method to `ForgeNode` returning `MergeCheckResult` (mergeable, available_strategies, conflicts, protection_status)
- [x] 2.2 Add `ForgeCheckMerge` request/response types to `aspen-client-api`
- [x] 2.3 Add `handle_check_merge` to forge handler executor
- [x] 2.4 Unit tests for check_merge with mergeable, blocked, and conflicting patches

## 3. RPC Handler Fix

- [x] 3.1 Update `ForgeMergePatch` request in `aspen-client-api`: replace `merge_commit: String` with `strategy: Option<String>` and `message: Option<String>`
- [x] 3.2 Rewrite `handle_merge_patch` in forge handler to call `ForgeNode::merge_patch()` with parsed strategy
- [x] 3.3 Update `ForgeMergePatch` response to include merge commit hash on success
- [x] 3.4 Update `to_operation` mapping for the changed request fields

## 4. CLI Update

- [x] 4.1 Update `PatchMergeArgs`: drop `--merge-commit`, add `--strategy` and `--message` flags
- [x] 4.2 Update `patch_merge()` to send new `ForgeMergePatch` request format
- [x] 4.3 Display merge commit hash from response on success

## 5. Web UI Merge Actions

- [x] 5.1 Add `check_merge` method to `AppState` calling `ForgeCheckMerge` RPC
- [x] 5.2 Update `patch_detail` route to fetch mergeability status alongside patch data
- [x] 5.3 Add mergeability banner template (green/yellow/red based on check result)
- [x] 5.4 Add merge button with strategy dropdown (form POST to new merge route)
- [x] 5.5 Add approve button (form POST to new approve route)
- [x] 5.6 Add `POST /{repo}/patches/{id}/merge` route handler calling `ForgeMergePatch` RPC with redirect
- [x] 5.7 Add `POST /{repo}/patches/{id}/approve` route handler calling `ForgeApprovePatch` RPC with redirect
- [x] 5.8 Add tree diff section to patch detail template showing changed files (added/modified/removed)

## 6. Integration Tests

- [x] 6.1 End-to-end test: create repo → push commits → create patch → approve → merge via RPC → verify ref advanced
- [x] 6.2 Test squash merge produces single-parent commit with correct tree
- [x] 6.3 Test fast-forward-only rejects diverged branches
- [x] 6.4 Test merge check returns correct mergeability for protected branch with CI status
