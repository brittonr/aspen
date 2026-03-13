## ADDED Requirements

### Requirement: BranchOverlay implements KeyValueStore

The `BranchOverlay<S: KeyValueStore>` type SHALL implement `KeyValueStore`, enabling transparent use anywhere a `KeyValueStore` is accepted. Reads SHALL fall through to the parent store for keys not present in the branch's dirty map. Writes and deletes SHALL buffer in-memory without touching the parent.

#### Scenario: Read falls through to parent

- **WHEN** a branch has no dirty entry for key "config/db/host"
- **AND** the parent store contains "config/db/host" with value "localhost"
- **THEN** `branch.read("config/db/host")` SHALL return "localhost"
- **AND** the branch SHALL record the key's `mod_revision` in its read set

#### Scenario: Read returns branch delta

- **WHEN** a branch has a dirty write for key "config/db/host" with value "remotehost"
- **THEN** `branch.read("config/db/host")` SHALL return "remotehost"
- **AND** the parent store SHALL NOT be queried

#### Scenario: Read respects tombstone

- **WHEN** a branch has a tombstone for key "config/db/host"
- **AND** the parent store contains "config/db/host"
- **THEN** `branch.read("config/db/host")` SHALL return `NotFound`

#### Scenario: Write buffers in memory

- **WHEN** `branch.write("key", "value")` is called
- **THEN** the write SHALL be stored in the branch's dirty map
- **AND** the parent store SHALL NOT receive any write

#### Scenario: Delete creates tombstone

- **WHEN** `branch.delete("key")` is called
- **THEN** a tombstone SHALL be recorded in the branch's dirty map
- **AND** the parent store SHALL NOT receive any delete

### Requirement: Atomic commit via Raft batch

A branch commit SHALL flush all buffered writes and deletes as a single atomic Raft operation. If the branch has a non-empty read set, the commit SHALL use `OptimisticTransaction`. If the read set is empty, the commit SHALL use `WriteCommand::Batch`.

#### Scenario: Commit with empty read set

- **WHEN** a branch has dirty writes for keys A, B, C and tombstone for key D
- **AND** the branch has not read any keys from the parent
- **THEN** `branch.commit()` SHALL issue a single `WriteCommand::Batch` containing Set(A), Set(B), Set(C), Delete(D)
- **AND** all four operations SHALL be applied atomically

#### Scenario: Commit with read set detects no conflict

- **WHEN** a branch has read key X (mod_revision=5) from the parent
- **AND** key X still has mod_revision=5 in the parent
- **AND** the branch has dirty write for key Y
- **THEN** `branch.commit()` SHALL issue an `OptimisticTransaction` with read_set=[(X, 5)] and write_set=[Set(Y)]
- **AND** the transaction SHALL succeed

#### Scenario: Commit with read set detects conflict

- **WHEN** a branch has read key X (mod_revision=5) from the parent
- **AND** key X now has mod_revision=7 in the parent (modified concurrently)
- **THEN** `branch.commit()` SHALL issue an `OptimisticTransaction` with read_set=[(X, 5)]
- **AND** the transaction SHALL be rejected
- **AND** the branch SHALL return a conflict error without modifying the parent

#### Scenario: Commit exceeding MAX_BATCH_SIZE

- **WHEN** a branch has more than `MAX_BATCH_SIZE` (1,000) dirty entries
- **THEN** `branch.commit()` SHALL return an error indicating the batch is too large
- **AND** the parent store SHALL NOT be modified

### Requirement: Zero-cost abort via Drop

Dropping a `BranchOverlay` SHALL discard all buffered writes and tombstones with no Raft interaction and no side effects on the parent store.

#### Scenario: Abort discards writes

- **WHEN** a branch has dirty writes for keys A, B, C
- **AND** the branch is dropped without calling `commit()`
- **THEN** the parent store SHALL NOT contain any writes from the branch
- **AND** no Raft operations SHALL be issued

### Requirement: Nested branch composition

`BranchOverlay<BranchOverlay<S>>` SHALL compose correctly. An inner branch commit SHALL merge its dirty map into the outer branch, not into Raft. Read resolution SHALL walk the chain from innermost to outermost to parent.

#### Scenario: Nested read resolution

- **WHEN** an inner branch has no dirty entry for key K
- **AND** the outer branch has a dirty write for key K with value "outer"
- **THEN** `inner.read(K)` SHALL return "outer"

#### Scenario: Inner commit merges into outer

- **WHEN** an inner branch has dirty writes for keys A, B
- **AND** `inner.commit()` is called
- **THEN** the outer branch's dirty map SHALL contain A and B
- **AND** no Raft operations SHALL be issued

#### Scenario: Inner tombstone shadows outer write

- **WHEN** the outer branch has a dirty write for key K
- **AND** the inner branch has a tombstone for key K
- **THEN** `inner.read(K)` SHALL return `NotFound`
- **AND** after `inner.commit()`, the outer branch SHALL have a tombstone for K

#### Scenario: Nesting depth limit

- **WHEN** branches are nested beyond `MAX_BRANCH_DEPTH` (8) levels
- **THEN** creating a new nested branch SHALL return an error

### Requirement: Tiger Style resource bounds

All branch operations SHALL enforce explicit resource limits to prevent memory exhaustion.

#### Scenario: Dirty key count limit

- **WHEN** a branch already contains `MAX_BRANCH_DIRTY_KEYS` (10,000) dirty entries
- **AND** a write is attempted for a new key
- **THEN** the write SHALL return an error indicating the branch is full

#### Scenario: Total bytes limit

- **WHEN** the sum of dirty value sizes in a branch reaches `MAX_BRANCH_TOTAL_BYTES` (64 MB)
- **AND** a write is attempted that would exceed the limit
- **THEN** the write SHALL return an error indicating the byte limit is exceeded

#### Scenario: Commit timeout

- **WHEN** a branch commit's Raft write does not complete within `BRANCH_COMMIT_TIMEOUT` (10s)
- **THEN** the commit SHALL return a timeout error
- **AND** the branch's dirty state SHALL remain intact for retry
