## Architecture

Three layers, each independently testable:

```
CLI ("forge diff")  →  RPC (ForgeDiffCommits)  →  Engine (diff.rs + unified.rs)
```

### Layer 1: Diff Engine Extensions (aspen-forge)

**Rename detection** is a post-pass over the `DiffResult`. After the structural diff completes, scan all `Removed` entries and all `Added` entries. When a Removed entry's `old_hash` matches an Added entry's `new_hash`, collapse them into a single `Renamed` entry. This is exact-match only (no fuzzy similarity scoring) — it catches file moves and renames without the complexity budget of similarity heuristics.

The post-pass runs in O(n) using a HashMap keyed on content hash. Bounded by `MAX_DIFF_ENTRIES` (already enforced upstream).

```rust
pub enum DiffKind {
    Added,
    Removed,
    Modified,
    Renamed,  // NEW
}

pub struct DiffEntry {
    // ... existing fields ...
    pub old_path: Option<String>,  // NEW: previous path for Renamed entries
}
```

**Unified diff renderer** (`unified.rs`) takes a `DiffEntry` with `old_content` and `new_content` populated and produces unified diff text. Uses the `similar` crate for line-level diffing. Configurable context lines (default 3). Output format matches `git diff` conventions:

```
--- a/src/main.rs
+++ b/src/main.rs
@@ -10,7 +10,8 @@
 unchanged line
-old line
+new line
 unchanged line
```

For binary files (non-UTF-8 content), emits `Binary files differ` instead of line diff. For entries without content loaded (blobs exceeded `MAX_DIFF_BLOB_SIZE`), emits the hash change only.

### Layer 2: RPC (aspen-client-api + aspen-forge-handler)

Two new request variants:

```rust
ForgeRequest::ForgeDiffCommits {
    repo_id: String,
    old_commit: String,    // hex hash
    new_commit: String,    // hex hash
    include_content: bool, // load blob bytes for unified diff
    context_lines: Option<u32>,
}

ForgeRequest::ForgeDiffRefs {
    repo_id: String,
    old_ref: String,       // e.g. "heads/main"
    new_ref: String,       // e.g. "heads/feature"
    include_content: bool,
    context_lines: Option<u32>,
}
```

Response is a new `ForgeDiffResult` variant carrying:

```rust
ForgeDiffResultResponse {
    entries: Vec<DiffEntryResponse>,  // serializable subset of DiffEntry
    truncated: bool,
    unified_diff: Option<String>,     // pre-rendered text when include_content=true
}
```

`DiffEntryResponse` is the wire-format version of `DiffEntry` — no `Vec<u8>` content fields, just paths/hashes/modes/kind. The unified diff text is rendered server-side and sent as a string.

`ForgeDiffRefs` resolves refs to commit hashes first (via `ForgeGetRef`), then delegates to `diff_commits`.

The handler goes in `executor.rs` alongside the other forge handlers. It calls `diff_commits` with `DiffOptions { include_content }`, runs `detect_renames` on the result, then renders unified diff text if content was loaded.

### Layer 3: CLI (aspen-cli)

```
aspen-cli forge diff <repo> <old_ref> [new_ref]
```

- Two refs: diff between them
- One ref: diff from parent commit to ref HEAD (like `git show`)
- `--stat`: show diffstat (files changed, insertions, deletions) instead of full diff
- `--name-only`: list changed paths only
- `--context <n>`: context lines (default 3)

Output goes to stdout, piped through a pager when connected to a terminal. Colored output using `owo-colors` (already a workspace dep).

## Constraints

- **Wire format**: `DiffEntryResponse` is a new struct, not reusing `DiffEntry` (which has `#[serde(skip)]` on content fields and raw `[u8; 32]` hashes that don't serialize nicely to JSON)
- **Postcard enum ordering**: `ForgeDiffCommits` and `ForgeDiffRefs` go at the END of the `ForgeRequest` enum. `ForgeDiffResult` goes at the end of `ClientRpcResponse`. This preserves existing discriminant values
- **Resource bounds**: Diff already bounded by `MAX_DIFF_ENTRIES`. Unified diff text bounded by content loading via `MAX_DIFF_BLOB_SIZE`. No new unbounded operations
- **`similar` crate**: Add as dependency to `aspen-forge` for line diffing. It's a pure-Rust, no-std-compatible crate with no transitive deps

## Error Cases

- Unknown repo → `RepoNotFound`
- Unknown ref → `RefNotFound`
- Unknown commit hash → `ObjectNotFound`
- Diff truncated → `truncated: true` in response (not an error)
- Blob too large for content → entry still appears with hashes, unified diff omitted for that file
