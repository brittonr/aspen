## Context

Aspen's `KeyValueStore` trait provides `read`, `write`, `delete`, and `scan` operations that go through Raft consensus. Every write is immediately committed. There is no built-in mechanism for grouping writes into an atomic unit that can be committed or discarded as a whole.

`WriteCommand::Batch` provides atomic multi-key writes, and `WriteCommand::OptimisticTransaction` provides CAS-based atomicity with a read set. These exist but are used at the call site — there is no abstraction that accumulates writes over time and flushes them in one batch.

The saga executor (`aspen-jobs/src/saga/`) handles multi-step workflows with compensation rollback. Each step persists state to KV. If a step fails, compensation functions must manually undo the step's writes. This is error-prone: the compensation author must know every key the step touched.

CI jobs (`aspen-ci-executor-vm`) use `AspenFs::with_prefix` for per-VM namespace isolation. If a job fails, its workspace keys remain in KV until explicit cleanup.

BranchFS (github.com/multikernel/branchfs) demonstrates file-level copy-on-write branching with commit/abort semantics. The same model applied at the KV layer would give all `KeyValueStore` consumers atomic branching with zero-cost abort.

## Goals / Non-Goals

**Goals:**

- A `BranchOverlay` type that implements `KeyValueStore`, buffering writes in-memory and committing them as a single Raft batch
- Zero-cost abort: dropping a branch discards all buffered writes with no Raft interaction
- Optimistic conflict detection on commit using `mod_revision` from `KeyValueWithRevision`
- Composable nesting: `BranchOverlay<BranchOverlay<S>>` works because the trait is recursive
- Deterministic scan-merge logic suitable for Verus formal verification
- FUSE `@branch` virtual path routing for multi-agent workspaces
- Integration points for saga executor, CI jobs, and deploy executor
- Tiger Style resource bounds on branch size, depth, and commit timeout

**Non-Goals:**

- Durable branches that survive node restarts. In-memory branches compose with the existing event store/saga journal for crash recovery. Durable branches would require cleanup-on-crash logic that defeats the purpose.
- Multi-node branch visibility. A branch is local to the node that created it. Other nodes see only committed state.
- Merge conflict resolution strategies beyond optimistic CAS. The caller handles commit failures. Three-way merge, CRDT merge, or last-writer-wins are future work.
- Replacing the existing saga compensation model. Branch-backed sagas are an additional mode, not a replacement.

## Decisions

### D1: In-memory branch storage

**Decision**: Branch writes are stored in a `DashMap<String, BranchEntry>` where `BranchEntry` is either `Write(value)` or `Tombstone`. No KV-backed storage.

**Rationale**: The primary value of branching is zero-cost abort. If branch state lives in KV, abort requires deleting keys through Raft — the same cost as committing. In-memory storage makes abort O(1) (drop the map). Commit is a single `WriteCommand::Batch` or `OptimisticTransaction`.

**Alternatives considered**:

- *KV-backed branches*: Durable but defeats zero-cost abort. Requires crash cleanup. Adds latency to every branch write (Raft round-trip per write, or complex write-ahead buffering).
- *Hybrid (in-memory + spill to KV)*: Adds complexity for a use case (multi-hour branches) that is better solved by periodic checkpoints in the workflow journal.

### D2: Optimistic conflict detection via read set

**Decision**: The branch records `(key, mod_revision)` for every key read from the parent. On commit, these become the `read_set` of an `OptimisticTransaction`. If any key's `mod_revision` changed, the commit is rejected.

**Rationale**: Aspen already has `OptimisticTransaction` with `read_set: Vec<(String, i64)>` and `write_set: Vec<WriteOp>`. This is the exact shape needed. Per-key conflict detection is strictly more precise than BranchFS's per-branch version counter.

**Alternatives considered**:

- *First-wins version counter (BranchFS model)*: Simpler but coarse — any concurrent write to the parent rejects the commit, even if it touched unrelated keys.
- *No conflict detection*: Last-writer-wins. Useful for some cases (deploy override) but unsafe as a default.

### D3: Commit via existing WriteCommand variants

**Decision**: A branch with no read set commits as `WriteCommand::Batch { operations }`. A branch with a read set commits as `WriteCommand::OptimisticTransaction { read_set, write_set }`. No new Raft commands.

**Rationale**: Both variants already exist, are tested, and go through the write batcher. The branch commit is a thin translation layer, not a new consensus path.

### D4: Nested branch commit targets parent overlay

**Decision**: When `BranchOverlay<BranchOverlay<S>>` commits, the inner branch merges its dirty map into the outer branch's dirty map. Only the outermost branch interacts with Raft.

**Rationale**: This preserves the atomicity guarantee — a nested branch's writes don't become visible until the root commits. It also means inner commit is O(N) map merge with no network I/O. Read resolution walks the chain: inner → outer → ... → base.

### D5: Scan merge as verified pure functions

**Decision**: The merge logic for combining a branch's dirty entries with a parent scan result goes in `src/verified/branch.rs` as pure functions. Verus specs prove tombstone correctness and ordering guarantees.

**Rationale**: Scan merge is the most complex operation and the most likely source of bugs. It is deterministic (no I/O, no async, no time dependency) and well-suited for formal verification. The function signature is roughly `fn merge_scan(dirty: &[(key, entry)], parent: &[(key, value)], tombstones: &[key], limit: u32) -> Vec<(key, value)>`.

### D6: FUSE @branch routing via path prefix detection

**Decision**: `AspenFs` detects `@branch-name` as the first path component after the mount root. Paths under `/@name/` route to a `BranchOverlay` instance managed by the filesystem. `.branchfs_ctl` virtual files allow commit/abort via writes (matching BranchFS's UX).

**Rationale**: The `@` prefix convention is established by BranchFS and avoids conflicts with real directory names. The existing `AspenFs` already routes through `path_to_key()` — the branch detection adds one check at the top of that function.

### D7: Saga integration as opt-in branch mode

**Decision**: `SagaExecutor` gains a `run_step_in_branch` method that wraps step execution in a `BranchOverlay`. On step success, the branch commit and the saga journal write happen in a single `WriteCommand::Batch`. On step failure, the branch is dropped (no compensation needed for KV writes).

**Rationale**: This composes with the existing saga model. Steps with external side effects (API calls, deploys) still use compensation. Steps that only mutate KV state use branches. The saga author chooses per-step.

## Risks / Trade-offs

**[Memory pressure from large branches]** → Tiger Style bounds: `MAX_BRANCH_DIRTY_KEYS = 10,000`, `MAX_BRANCH_TOTAL_BYTES = 64 MB`. Writes beyond the limit return an error, forcing the caller to commit or abort. These limits are configurable at branch creation.

**[Commit rejected due to conflict]** → The caller retries or aborts. For CI jobs this means re-running the job. For sagas this means the step failed and compensation (or branch-abort) kicks in. No silent data loss.

**[Large commit batch exceeds MAX_BATCH_SIZE (1,000)]** → The branch splits the commit into multiple `Batch` writes. This sacrifices atomicity for branches with more than 1,000 dirty keys. For the atomicity guarantee, branches must stay under `MAX_BATCH_SIZE`. Branches that need more should use intermediate commit points (checkpoint pattern).

**[Scan merge performance with large parent result sets]** → Bounded by `MAX_SCAN_RESULTS = 10,000` on the parent side and `MAX_BRANCH_DIRTY_KEYS = 10,000` on the branch side. Worst case is a 20,000-element sorted merge, which is microseconds.

**[Nested branch depth]** → `MAX_BRANCH_DEPTH = 8`. Each nesting level adds one map lookup per read. At depth 8, a read that misses all overlays does 8 map lookups before hitting the parent — negligible compared to the Raft round-trip for the final read.

**[FUSE @branch lifecycle]** → Branches created through FUSE virtual paths are owned by the filesystem. They are dropped on unmount. Long-lived branches should use the programmatic API, not FUSE.
