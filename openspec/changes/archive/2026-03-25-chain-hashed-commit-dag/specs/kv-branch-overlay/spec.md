## MODIFIED Requirements

### Requirement: Atomic commit via Raft batch

A branch commit SHALL flush all buffered writes and deletes as a single atomic Raft operation. If the branch has a non-empty read set, the commit SHALL use `OptimisticTransaction`. If the read set is empty, the commit SHALL use `WriteCommand::Batch`.

When the `commit-dag` feature is enabled, `commit()` SHALL additionally compute a `CommitId`, store a `Commit` snapshot in KV, update the branch tip pointer, and return the `CommitId` alongside the `WriteResult`. The commit metadata write SHALL be included in the same Raft batch as the data mutations.

When the `commit-dag` feature is NOT enabled, `commit()` SHALL behave exactly as before, returning only a `WriteResult`.

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

#### Scenario: Commit with commit-dag feature produces CommitId

- **WHEN** the `commit-dag` feature is enabled
- **AND** a branch commits dirty writes for keys A, B
- **THEN** `branch.commit()` SHALL return a `CommitResult` containing both the `WriteResult` and a `CommitId`
- **AND** the Raft batch SHALL include `Set(_sys:commit:{commit_id_hex}, <serialized_commit>)` and `Set(_sys:commit-tip:{branch_id}, <commit_id_hex>)`

#### Scenario: Commit without commit-dag feature is unchanged

- **WHEN** the `commit-dag` feature is NOT enabled
- **AND** a branch commits dirty writes
- **THEN** `branch.commit()` SHALL return only a `WriteResult`
- **AND** no `_sys:commit:` entries SHALL be written

#### Scenario: Commit metadata is atomic with data mutations

- **WHEN** the `commit-dag` feature is enabled
- **AND** a branch commits
- **THEN** the data mutations and commit metadata SHALL be in the same Raft batch
- **AND** either all entries (data + metadata) are applied or none are

### Requirement: Branch tracks parent commit

When the `commit-dag` feature is enabled, `BranchOverlay` SHALL track the CommitId of the most recent commit on this branch. After `commit()`, the parent commit field SHALL be updated to the CommitId just produced. On branch creation, the parent commit SHALL be `None` (unless created via `fork_from`).

#### Scenario: First commit has no parent

- **WHEN** a newly created branch commits for the first time
- **THEN** the resulting Commit's `parent` field SHALL be `None`

#### Scenario: Second commit chains from first

- **WHEN** a branch commits producing `C1`
- **AND** the branch accumulates new writes and commits again producing `C2`
- **THEN** `C2.parent` SHALL be `Some(C1)`

#### Scenario: Fork branch tracks source commit as parent

- **WHEN** `fork_from(C5, "my-fork", store)` creates a new branch
- **AND** the new branch commits producing `C6`
- **THEN** `C6.parent` SHALL be `Some(C5)`
