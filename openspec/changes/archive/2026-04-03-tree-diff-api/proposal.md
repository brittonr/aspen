## Why

The tree diff engine (`aspen-forge/src/git/diff.rs`) exists and works — structural comparison of two trees, recursive subdirectory walk, content loading, mode change detection, truncation limits. But it's completely internal. No RPC variant exposes it, no CLI command invokes it, and there's no way for a client to ask "what changed between these two commits?" without reimplementing the walk client-side.

This matters because diff is the foundation of every forge workflow that follows: patch review (what did this PR change?), CI change detection (which paths were touched?), merge preview, commit browsing, and federation conflict resolution. The merge engine already exists. The diff engine already exists. The gap is the wiring.

A secondary gap: the diff engine reports structural changes (which files were added/removed/modified) but doesn't produce unified text diffs (the `+`/`-` line output users expect from `git diff`). For CLI and TUI display, we need a text diff renderer that operates on the `DiffEntry` content bytes.

## What Changes

- Add `ForgeDiffCommits` and `ForgeDiffRefs` variants to `ForgeRequest` / `ClientRpcRequest`
- Add a `ForgeDiffResult` response variant carrying serialized `DiffEntry` records
- Wire the handler in `aspen-forge-handler` to call existing `diff_commits` / `diff_trees`
- Add `aspen-cli forge diff <repo> <ref1> [ref2]` command with unified diff text output
- Add a `unified_diff` renderer module to `aspen-forge` that takes `DiffEntry` content bytes and produces standard unified diff text (with configurable context lines)
- Add rename detection as a post-pass over structural diff output — entries where a `Removed` and `Added` entry share the same content hash are collapsed into a `Renamed` kind

## Capabilities

### New Capabilities

- `diff-rpc`: RPC variants for commit-to-commit and ref-to-ref diff, exposed through the standard client protocol
- `unified-diff-renderer`: Text diff output from `DiffEntry` content bytes, producing standard unified diff format with configurable context lines
- `rename-detection`: Post-pass over structural diff that collapses matching Removed+Added pairs into Renamed entries based on content hash equality

### Modified Capabilities

- `tree-diff` (existing spec): Extended with `Renamed` diff kind and rename detection

## Impact

- `crates/aspen-client-api/src/messages/forge.rs`: Add `ForgeDiffCommits`, `ForgeDiffRefs` request variants
- `crates/aspen-client-api/src/messages/mod.rs`: Add `ForgeDiffResult` response variant
- `crates/aspen-forge-handler/src/executor.rs`: Wire diff handlers
- `crates/aspen-forge/src/git/diff.rs`: Add `DiffKind::Renamed`, `detect_renames()` post-pass
- `crates/aspen-forge/src/git/unified.rs`: New unified diff text renderer
- `crates/aspen-forge/src/git/mod.rs`: Re-export unified diff types
- `crates/aspen-cli/src/bin/aspen-cli/commands/git.rs`: Add `Diff` subcommand
