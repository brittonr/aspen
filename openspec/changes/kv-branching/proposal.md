## Why

Every KV mutation in Aspen is immediately Raft-committed. There is no way to accumulate a set of writes and apply them atomically, or to discard them if something goes wrong. This makes CI jobs leave orphaned workspace keys on failure, saga compensation logic complex and error-prone, and deploy rollback dependent on authors correctly undoing every write. BranchFS demonstrated that copy-on-write branching over a filesystem gives AI agents isolated speculative workspaces with zero-cost abort. The same primitive applied at Aspen's KV layer would give every consumer of `KeyValueStore` — FUSE, CI, sagas, deploys — atomic commit/abort semantics without new traits or protocols.

## What Changes

- New `BranchOverlay<S: KeyValueStore>` type that implements `KeyValueStore` itself. Reads fall through to the parent store. Writes and deletes buffer in-memory. Commit flushes all buffered mutations as a single `WriteCommand::Batch` through Raft. Abort is `Drop`.
- Conflict detection on commit using `mod_revision` from `KeyValueWithRevision`. A branch records the revision of every key it reads from the parent. On commit, an `OptimisticTransaction` rejects the batch if any read key was modified concurrently.
- Nested branches: `BranchOverlay<BranchOverlay<S>>` composes naturally. A nested commit merges into the parent overlay, not into Raft. Only the root-level commit touches Raft.
- FUSE `@branch` virtual paths: the existing `AspenFs` gains branch-aware path routing so that `/@branch-name/path` resolves through a `BranchOverlay` instead of the base store.
- Scan merging: branch scans merge the in-memory dirty map with the parent scan result, filtering tombstones and respecting limits. This logic goes in `src/verified/` as pure functions suitable for Verus verification.
- Integration with saga executor: saga steps can run inside a branch, replacing manual compensation with automatic rollback on failure. Commit + journal write happen in one atomic Raft batch.
- Integration with CI pipeline executor: each CI job gets a branch. Success commits artifacts to the base store. Failure drops the branch with no cleanup.
- Integration with deploy executor: deploys run inside a branch. Health check passes → commit. Fails → drop, base state unchanged.

## Capabilities

### New Capabilities

- `kv-branch-overlay`: Core `BranchOverlay` type implementing `KeyValueStore` with in-memory copy-on-write, tombstones, optimistic conflict detection, and atomic commit via Raft batch writes.
- `kv-branch-scan-merge`: Deterministic sorted-merge logic for combining branch dirty state with parent scan results. Pure functions in `src/verified/` with Verus specs.
- `kv-branch-fuse`: FUSE `@branch` virtual path routing in `AspenFs`, mapping `/@name/path` to a `BranchOverlay` instance.
- `kv-branch-saga`: Saga executor integration where steps run inside branches, replacing compensation with automatic rollback.

### Modified Capabilities

- `ci`: CI job executor uses `BranchOverlay` for workspace isolation instead of raw prefix writes. Jobs that fail leave no orphaned keys.
- `coordination`: Saga executor gains a branch-backed execution mode alongside the existing compensation-based mode.

## Impact

- **New crate**: `aspen-kv-branch` (or module in `aspen-core`)
- **Modified crates**: `aspen-fuse` (virtual path routing), `aspen-jobs` (saga integration), `aspen-ci` (job workspace branches), `aspen-ci-executor-vm` (deploy branches)
- **New feature flag**: `kv-branch` gating the overlay and integrations
- **New constants**: `MAX_BRANCH_DIRTY_KEYS`, `MAX_BRANCH_DEPTH`, `MAX_BRANCH_TOTAL_BYTES`, `BRANCH_COMMIT_TIMEOUT`
- **No protocol changes**: `BranchOverlay` implements the existing `KeyValueStore` trait, so all consumers work without modification
- **No breaking changes**: existing code paths are unchanged; branch usage is opt-in
