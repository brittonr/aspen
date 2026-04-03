## 1. Rename detection in diff engine

- [x] 1.1 Add `DiffKind::Renamed` variant to the `DiffKind` enum in `diff.rs`
- [x] 1.2 Add `old_path: Option<String>` field to `DiffEntry` (populated only for `Renamed` entries)
- [x] 1.3 Implement `detect_renames(result: &mut DiffResult)` — build a `HashMap<[u8;32], usize>` of Removed entries keyed by `old_hash`, scan Added entries for matches, collapse matching pairs into `Renamed` entries
- [x] 1.4 Call `detect_renames` at the end of `diff_trees` (after sort, before return)
- [x] 1.5 Add unit tests: single rename, multiple renames, rename+modify (no match), duplicate hashes (first match wins), no renames (passthrough)

## 2. Unified diff renderer

- [x] 2.1 Add `similar` crate as dependency to `aspen-forge` (`similar = "2"`)
- [x] 2.2 Create `crates/aspen-forge/src/git/unified.rs` with `render_unified_diff(entries: &[DiffEntry], context_lines: u32) -> String`
- [x] 2.3 Implement per-entry rendering: `--- a/{path}` / `+++ b/{path}` headers, `@@` hunk markers via `similar::TextDiff`, context line support
- [x] 2.4 Handle edge cases: Added files (`--- /dev/null`), Removed files (`+++ /dev/null`), Renamed files (`rename from`/`rename to` headers), binary content (`Binary files differ`), missing content (hash-only summary)
- [x] 2.5 Implement `render_diffstat(entries: &[DiffEntry]) -> String` — per-file `+`/`-` counts and bar chart like `git diff --stat`
- [x] 2.6 Re-export from `git/mod.rs`: `pub use unified::{render_unified_diff, render_diffstat}`
- [x] 2.7 Add unit tests: modified file with context, added file, removed file, renamed file, binary file, empty diff, diffstat output format

## 3. RPC request/response types

- [x] 3.1 Add `ForgeDiffCommits` and `ForgeDiffRefs` variants to `ForgeRequest` enum in `forge.rs` (at the END, after existing variants)
- [x] 3.2 Define `DiffEntryResponse` struct in `forge.rs` with `path`, `kind`, `old_path`, `old_mode`, `new_mode`, `old_hash`, `new_hash` (all hex strings for hashes)
- [x] 3.3 Define `ForgeDiffResultResponse` struct with `entries: Vec<DiffEntryResponse>`, `truncated: bool`, `unified_diff: Option<String>`
- [x] 3.4 Add `ForgeDiffResult(ForgeDiffResultResponse)` variant to `ClientRpcResponse` enum (at the END)
- [x] 3.5 Add both variants to `ForgeRequest::to_operation()` as `Operation::Read`
- [x] 3.6 Add conversion `From<DiffEntry> for DiffEntryResponse` (hex-encode hashes)

## 4. Handler wiring

- [x] 4.1 Add `handle_diff_commits` method to `ForgeServiceExecutor` — parse hex hashes, call `diff_commits`, run `detect_renames`, render unified diff if `include_content`, convert to `ForgeDiffResultResponse`
- [x] 4.2 Add `handle_diff_refs` method — resolve refs to commit hashes via existing `handle_get_ref` path, delegate to `handle_diff_commits`
- [x] 4.3 Wire both variants in the `execute` match arm
- [x] 4.4 Add operation names to `HANDLES` array, update handle count test
- [ ] 4.5 Add integration test: create repo, store blobs, create trees, commit, diff via RPC (deferred — requires running cluster)

## 5. CLI command

- [x] 5.1 Add `Diff(DiffArgs)` variant to `GitCommand` enum in `commands/git.rs`
- [x] 5.2 Define `DiffArgs` struct: `repo` (positional), `old_ref` (positional), `new_ref` (optional positional), `--stat`, `--name-only`, `--context <n>`
- [x] 5.3 Implement `handle_diff` — send `ForgeDiffRefs` RPC, display result based on flags (unified / stat / name-only)
- [x] 5.4 Add colored output for diff text: red for `-` lines, green for `+` lines, cyan for `@@` markers

## 6. Verification

- [x] 6.1 `cargo nextest run -p aspen-forge` — all existing + new diff/unified tests pass (378 pass)
- [x] 6.2 `cargo nextest run -p aspen-forge-handler` — handler tests pass (25 pass)
- [x] 6.3 `cargo clippy --all-targets -- --deny warnings` on modified crates (clean)
