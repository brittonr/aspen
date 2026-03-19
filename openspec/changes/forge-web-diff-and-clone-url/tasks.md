## 1. Clone URL display

- [x] 1.1 Thread the cluster ticket string from `AspenClient` through `AppState` so templates can access it
- [x] 1.2 Add a clone URL section to `repo_overview` template showing `git clone aspen://{ticket}/{repo_id} {repo_name}`
- [x] 1.3 Add CSS for the clone URL box (monospace, selectable, subtle background)

## 2. Markdown in issues and patches

- [x] 2.1 Move `render_markdown` from `routes.rs` to `templates.rs` (or a shared helper) so templates can call it
- [x] 2.2 Render issue body as markdown in `issue_detail` template
- [x] 2.3 Render comment bodies as markdown in `issue_detail` template
- [x] 2.4 Render patch description as markdown in `patch_detail` template

## 3. Commit diff — data layer

- [x] 3.1 Add `similar` crate dependency to `aspen-forge-web/Cargo.toml`
- [x] 3.2 Add `AppState::get_commit` method (wraps existing `ForgeGetCommit` RPC)
- [x] 3.3 Add `AppState::diff_trees` method — takes two tree hashes, recursively walks both trees, returns list of changed files with their old/new blob hashes
- [x] 3.4 Add `AppState::compute_file_diff` method — fetches two blobs, computes unified diff using `similar`, returns diff lines with add/remove/context classification
- [x] 3.5 Add resource bounds: max 50 files per diff, max 256KB per blob for inline diff

## 4. Commit diff — route and template

- [x] 4.1 Add `/{repo_id}/commit/{hash}` route in `routes.rs` dispatching to a `commit_detail` handler
- [x] 4.2 Implement `commit_detail` handler — fetch commit, fetch parent commit (if any), diff trees, compute file diffs, render template
- [x] 4.3 Create `commit_detail` template — commit metadata header, file change list, unified diff blocks with line-level coloring
- [x] 4.4 Add diff-specific CSS — green/red line backgrounds, file headers, diff block borders
- [x] 4.5 Link commit hashes on the commit log page and repo overview to `/{repo_id}/commit/{hash}`

## 5. Testing

- [ ] 5.1 End-to-end test: start cluster, push repo with multiple commits, verify commit detail page returns 200 with diff content
- [ ] 5.2 Test root commit (no parent) — all files shown as added
- [ ] 5.3 Verify clone URL appears on repo overview and contains correct ticket/repo_id
