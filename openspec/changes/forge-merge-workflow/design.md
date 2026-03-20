## Context

`ForgeNode::merge_patch()` already implements the full merge workflow: resolve patch → check branch protection → CAS retry loop with fast-forward detection → three-way merge → create merge commit → advance ref → transition COB. The problem is that the RPC handler bypasses this and calls `cobs.merge_patch()` directly (COB state transition only), the CLI requires a pre-computed merge commit hash, and the web UI has no merge actions.

The infrastructure is 90% complete. This change wires the existing `ForgeNode::merge_patch()` through the RPC layer, adds merge strategy selection, and builds the user-facing surfaces (CLI, web UI).

## Goals / Non-Goals

**Goals:**

- Server-side merge execution via `ForgeNode::merge_patch()` through all interfaces (RPC, CLI, web)
- Merge strategy selection: merge commit (default), fast-forward-only, squash
- Pre-merge checks without side effects (mergeability endpoint)
- Web UI for merge/approve actions with CI status display and diff view

**Non-Goals:**

- File-level content merge (line-by-line diff3) — conflicts remain at tree entry level
- Interactive conflict resolution UI — conflicts block the merge and must be fixed in the source branch
- Rebase merge strategy — can be added later
- Review/code-review threading — the existing approval system is sufficient for now

## Decisions

### 1. `ForgeNode::merge_patch()` gains a `MergeStrategy` parameter

The existing method already handles fast-forward detection. Adding a strategy enum:

```rust
pub enum MergeStrategy {
    /// Create merge commit (default). Falls back to fast-forward when possible.
    MergeCommit,
    /// Fast-forward only. Fails if target has diverged.
    FastForwardOnly,
    /// Squash all patch commits into a single commit on the target branch.
    Squash,
}
```

**Rationale**: Matching GitHub/GitLab's three strategies covers the common cases. Rebase is more complex (requires rewriting commits) and can be added later without API changes.

**Squash implementation**: Create a single-parent commit (parent = target head) with the merged tree. The commit message concatenates all patch commit messages. The patch COB transitions to Merged with the squash commit hash.

### 2. `ForgeMergePatch` RPC drops `merge_commit`, adds `strategy`

Current:

```rust
ForgeMergePatch { repo_id, patch_id, merge_commit: String }
```

New:

```rust
ForgeMergePatch { repo_id, patch_id, strategy: Option<String>, message: Option<String> }
```

`strategy` is `Option<String>` (not the enum directly) for wire compatibility — parsed server-side. Values: `"merge"` (default), `"fast-forward"`, `"squash"`. `message` overrides the auto-generated merge commit message.

**Rationale**: The RPC layer uses strings for enums to avoid coupling wire format to Rust types. The handler validates and converts.

### 3. New `ForgeCheckMerge` RPC for dry-run

```rust
ForgeCheckMerge { repo_id, patch_id }
→ ForgeCheckMergeResult { mergeable, strategy_available: Vec<String>, conflicts: Vec<String>, protection_status: ProtectionCheckResult }
```

Runs the same checks as `merge_patch()` without side effects: resolves patch, checks protection rules, attempts tree merge, reports conflicts. The web UI calls this to show mergeability status before the user clicks merge.

**Rationale**: Separating the check from the action prevents accidental merges and gives the UI the data it needs to render merge/block states.

### 4. Web UI merge flow

The patch detail page gains:

- **Mergeability banner**: Green (mergeable), yellow (checks pending), red (conflicts/protection blocked). Calls `ForgeCheckMerge` on page load.
- **Merge button**: Dropdown with strategy selection. Disabled when not mergeable.
- **Approve button**: Posts `ForgeApprovePatch` RPC.
- **Diff tab**: Shows tree diff between patch head and target ref using `diff_trees`. File-level (added/modified/removed), not line-level.

All actions use form POST → server-side RPC → redirect back to patch detail. No JavaScript required.

### 5. CLI merge drops `--merge-commit`

```
aspen-cli patch merge --repo <id> <patch-id> [--strategy merge|fast-forward|squash] [--message "..."]
```

The server handles merge commit creation. The CLI is a thin RPC caller.

## Risks / Trade-offs

**[Squash loses history]** → Expected behavior. Users choose squash when they want a clean single-commit on the target branch. The patch COB retains the full revision history.

**[CAS retry loop with squash creates different commits on retry]** → Each retry re-reads target head, so the squash commit parent changes. The tree content stays the same (assuming no conflicts). This is correct — the squash commit should be based on the current target, not a stale one.

**[ForgeCheckMerge can go stale]** → The check result is a point-in-time snapshot. Between check and merge, another push could land. The CAS retry loop in `merge_patch()` handles this — the check is advisory, the merge is authoritative.

**[Breaking RPC change]** → `ForgeMergePatch` field change breaks existing callers. No production callers exist — only test code. Acceptable for pre-1.0.
