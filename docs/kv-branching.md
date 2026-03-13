# KV Branching

Copy-on-write branching for Aspen's `KeyValueStore` trait.

## Model

A **branch** is an in-memory overlay that intercepts reads and writes. The
parent store is never modified until the branch commits.

```
                ┌─────────────┐
                │   Branch    │  dirty map (DashMap)
                │  (in-mem)   │  read set (mod_revisions)
                └──────┬──────┘
                       │ read miss → fall through
                       ▼
                ┌─────────────┐
                │   Parent    │  Raft-backed KV store
                │   Store     │
                └─────────────┘
```

**Commit** flushes all dirty entries as a single `WriteCommand::Batch` or
`OptimisticTransaction` (if the branch read any parent keys). This gives
atomic commit with conflict detection.

**Abort** is `Drop` — zero Raft interaction, zero cost.

## Operations

| Operation | Behavior |
|-----------|----------|
| `read(key)` | Check dirty map → tombstone returns NotFound → miss falls through to parent, records `mod_revision` |
| `write(key, value)` | Buffers in dirty map. Parent untouched. |
| `delete(key)` | Inserts tombstone in dirty map. Parent untouched. |
| `scan(prefix)` | Merges dirty entries with parent scan using verified two-pointer merge. |
| `commit()` | Flushes as atomic Raft batch. Uses `OptimisticTransaction` if read set is non-empty. |
| `abort()` / `drop` | Discards all state. No Raft interaction. |

## Conflict Detection

Branches track every key read from the parent along with its `mod_revision`.
On commit, these form the `read_set` of an `OptimisticTransaction`. If any
read key was modified concurrently, the commit is rejected with `CommitConflict`.

Branches that never read from the parent (write-only) skip conflict detection
and use a plain `WriteCommand::Batch`.

## Nesting

`BranchOverlay<BranchOverlay<S>>` composes naturally. An inner commit merges
into the outer branch's dirty map — no Raft interaction. Only the outermost
commit touches Raft. Read resolution walks the chain from innermost to
outermost to parent.

Maximum nesting depth: `MAX_BRANCH_DEPTH` (8).

## Resource Bounds

| Constant | Value | Purpose |
|----------|-------|---------|
| `MAX_BRANCH_DIRTY_KEYS` | 10,000 | Max dirty entries per branch |
| `MAX_BRANCH_TOTAL_BYTES` | 64 MB | Max dirty value bytes |
| `MAX_BRANCH_DEPTH` | 8 | Max nesting levels |
| `BRANCH_COMMIT_TIMEOUT_MS` | 10,000 | Raft write timeout |

All overridable via `BranchConfig`.

## Durable Execution Composition

Branches are in-memory by design. For crash recovery, compose with existing
durable primitives:

- **Saga executor**: Branch-backed steps need no compensation. If the step
  fails, the branch is dropped. If it succeeds, the branch commit and saga
  journal write happen in one atomic batch.

- **CI jobs**: Each job gets a branch. Success commits workspace artifacts.
  Failure drops the branch — no orphaned keys, no cleanup.

- **Deploy executor**: Deploy writes happen inside a branch. Health check
  pass → commit. Fail → drop, base state unchanged.

## Verified Scan Merge

The `merge_scan` function in `src/verified/scan_merge.rs` is a pure,
deterministic function with no I/O and no async. Verus specifications in
`verus/scan_merge_spec.rs` prove:

1. **Tombstone exclusion**: No tombstoned key appears in output.
2. **Sort order**: Output is strictly sorted by key.
3. **Branch precedence**: Branch values override parent values for duplicate keys.
