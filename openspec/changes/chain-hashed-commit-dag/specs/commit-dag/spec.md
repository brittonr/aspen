## ADDED Requirements

### Requirement: CommitId is a chain hash

A `CommitId` SHALL be a 32-byte BLAKE3 hash computed as `blake3(parent_commit_hash || branch_id_bytes || mutations_hash || raft_revision_le_bytes || timestamp_ms_le_bytes)`. The `CommitId` type SHALL alias `ChainHash` from `aspen-raft/src/verified/integrity.rs`.

#### Scenario: First commit on a branch chains from genesis

- **WHEN** a branch with id "experiment" has no prior commits
- **AND** the branch commits with dirty map `{a: Set("1"), b: Delete}`
- **THEN** the CommitId SHALL be computed with `parent_commit_hash = GENESIS_HASH` (all zeros)
- **AND** the CommitId SHALL be a deterministic function of the branch id, mutations, raft revision, and timestamp

#### Scenario: Subsequent commit chains from parent

- **WHEN** a branch has a prior commit with CommitId `C1`
- **AND** the branch commits new mutations
- **THEN** the CommitId SHALL be computed with `parent_commit_hash = C1`
- **AND** the resulting CommitId SHALL differ from `C1`

#### Scenario: Identical mutations produce different CommitIds with different parents

- **WHEN** branch A commits mutations `{x: Set("v")}` with parent `P1`
- **AND** branch B commits mutations `{x: Set("v")}` with parent `P2` where `P1 != P2`
- **THEN** the two CommitIds SHALL differ

#### Scenario: CommitId is deterministic

- **WHEN** the same parent hash, branch id, mutations, raft revision, and timestamp are provided twice
- **THEN** the computed CommitId SHALL be identical both times

### Requirement: Commit struct captures mutation snapshot

A `Commit` SHALL be an immutable struct containing: `id` (CommitId), `parent` (Option<CommitId>), `branch_id` (String), `mutations` (sorted Vec of key-mutation pairs), `mutations_hash` (BLAKE3 of sorted mutations), `raft_revision` (u64), `chain_hash_at_commit` (ChainHash from Raft log at commit time), and `timestamp_ms` (u64).

#### Scenario: Commit stores sorted mutations

- **WHEN** a branch has dirty entries `{c: Set("3"), a: Set("1"), b: Delete}` (insertion order)
- **AND** the branch commits
- **THEN** the Commit's mutations SHALL be stored sorted by key: `[(a, Set("1")), (b, Delete), (c, Set("3"))]`

#### Scenario: Commit binds to Raft log position

- **WHEN** a branch commits and the resulting Raft write lands at log index 42 in term 3
- **THEN** the Commit's `raft_revision` SHALL be 42
- **AND** the Commit's `chain_hash_at_commit` SHALL be the Raft chain hash at log index 42

#### Scenario: Commit is immutable after creation

- **WHEN** a Commit with CommitId `C1` is stored in KV
- **THEN** no operation SHALL modify the Commit's fields
- **AND** re-reading the Commit by CommitId SHALL return identical data

### Requirement: Commits stored in KV with system prefix

Commits SHALL be serialized and stored as KV entries at key `_sys:commit:{commit_id_hex}`. Branch head pointers SHALL be stored at `_sys:commit-tip:{branch_id}` containing the hex CommitId of the branch's most recent commit.

#### Scenario: Commit stored at system key

- **WHEN** a branch commits producing CommitId with hex `abcd1234...`
- **THEN** the Commit SHALL be stored at KV key `_sys:commit:abcd1234...`
- **AND** the branch tip SHALL be updated at `_sys:commit-tip:{branch_id}` to `abcd1234...`

#### Scenario: Commit retrievable by CommitId

- **WHEN** a Commit was stored with CommitId `C1`
- **AND** a caller requests the Commit by CommitId `C1`
- **THEN** the system SHALL deserialize and return the Commit from `_sys:commit:{C1_hex}`

#### Scenario: Branch tip tracks latest commit

- **WHEN** a branch produces commits `C1`, then `C2`, then `C3`
- **THEN** `_sys:commit-tip:{branch_id}` SHALL contain `C3`'s hex CommitId
- **AND** `C3.parent` SHALL be `Some(C2.id)`
- **AND** `C2.parent` SHALL be `Some(C1.id)`

### Requirement: Mutations hash is deterministic BLAKE3 over sorted entries

The `mutations_hash` field SHALL be computed by streaming sorted mutation entries through a BLAKE3 hasher. For each entry, the hasher SHALL consume: key length (u32 LE), key bytes, mutation tag (0x01 for Set, 0x02 for Delete), and for Set mutations the value length (u32 LE) and value bytes. This function SHALL be pure (no I/O, no async, no time dependency) and live in `src/verified/`.

#### Scenario: Empty mutations produce a known hash

- **WHEN** the dirty map is empty
- **THEN** the mutations_hash SHALL be `blake3("")` (BLAKE3 of empty input)

#### Scenario: Sort order determines hash

- **WHEN** mutations are `{b: Set("2"), a: Set("1")}`
- **THEN** the hasher SHALL process key "a" before key "b"
- **AND** the result SHALL be identical regardless of the insertion order into the dirty map

#### Scenario: Tombstones contribute to hash

- **WHEN** mutations are `{a: Set("1"), b: Delete}`
- **THEN** key "b" SHALL contribute tag byte 0x02 (Delete) to the hash
- **AND** the result SHALL differ from mutations `{a: Set("1")}` alone

### Requirement: Fork from commit

`fork_from(commit_id, branch_id, parent_store)` SHALL create a new `BranchOverlay` pre-populated with the mutations from the specified commit. The new branch's genesis commit SHALL reference the source CommitId as its parent.

#### Scenario: Fork creates branch with commit's mutations

- **WHEN** commit `C1` has mutations `{a: Set("1"), b: Set("2")}`
- **AND** `fork_from(C1, "fork-1", store)` is called
- **THEN** the new branch SHALL have dirty entries `{a: Set("1"), b: Set("2")}`
- **AND** reading key "a" from the new branch SHALL return "1" without querying the parent store

#### Scenario: Fork from nonexistent commit fails

- **WHEN** `fork_from(nonexistent_commit_id, "fork-1", store)` is called
- **THEN** the operation SHALL return `CommitNotFound` error

#### Scenario: Fork from GC'd commit fails gracefully

- **WHEN** commit `C1` has been garbage collected
- **AND** `fork_from(C1, "fork-1", store)` is called
- **THEN** the operation SHALL return `CommitNotFound` error with a message indicating the commit may have been garbage collected

#### Scenario: Fork verifies commit integrity

- **WHEN** `fork_from(C1, "fork-1", store)` is called
- **THEN** the system SHALL recompute the mutations_hash from the stored mutations
- **AND** verify it matches the Commit's `mutations_hash` field
- **AND** reject the fork with `CommitCorrupted` if they differ

### Requirement: Diff between commits

`diff(commit_a, commit_b)` SHALL compare the mutation snapshots of two commits and return a list of `DiffEntry` values indicating keys that were added, removed, or changed between the two commits.

#### Scenario: Diff of identical commits is empty

- **WHEN** `diff(C1, C1)` is called
- **THEN** the result SHALL be an empty list

#### Scenario: Diff detects added keys

- **WHEN** commit `C1` has mutations `{a: Set("1")}`
- **AND** commit `C2` has mutations `{a: Set("1"), b: Set("2")}`
- **THEN** `diff(C1, C2)` SHALL include `DiffEntry::Added { key: "b", value: "2" }`

#### Scenario: Diff detects removed keys

- **WHEN** commit `C1` has mutations `{a: Set("1"), b: Set("2")}`
- **AND** commit `C2` has mutations `{a: Set("1")}`
- **THEN** `diff(C1, C2)` SHALL include `DiffEntry::Removed { key: "b" }`

#### Scenario: Diff detects changed values

- **WHEN** commit `C1` has mutations `{a: Set("1")}`
- **AND** commit `C2` has mutations `{a: Set("2")}`
- **THEN** `diff(C1, C2)` SHALL include `DiffEntry::Changed { key: "a", old: "1", new: "2" }`

#### Scenario: Diff detects tombstone transitions

- **WHEN** commit `C1` has mutations `{a: Set("1")}`
- **AND** commit `C2` has mutations `{a: Delete}`
- **THEN** `diff(C1, C2)` SHALL include `DiffEntry::Changed { key: "a", old: Set("1"), new: Delete }`

### Requirement: Commit DAG garbage collection

The system SHALL periodically scan `_sys:commit:` entries and delete commits whose `timestamp_ms` is older than `COMMIT_GC_TTL_SECONDS` (default: 7 days). Commits referenced as `parent` by any non-expired commit SHALL be protected from GC. Commits referenced by a branch tip (`_sys:commit-tip:`) SHALL be protected from GC.

#### Scenario: Expired commit with no references is collected

- **WHEN** commit `C1` has `timestamp_ms` older than 7 days
- **AND** no non-expired commit has `parent = Some(C1.id)`
- **AND** no branch tip points to `C1`
- **THEN** GC SHALL delete `_sys:commit:{C1_hex}`

#### Scenario: Expired commit with live child is protected

- **WHEN** commit `C1` has `timestamp_ms` older than 7 days
- **AND** commit `C2` has `parent = Some(C1.id)` and `C2` is not expired
- **THEN** GC SHALL NOT delete `_sys:commit:{C1_hex}`

#### Scenario: Branch tip commit is never collected

- **WHEN** `_sys:commit-tip:my-branch` points to commit `C3`
- **THEN** GC SHALL NOT delete `C3` regardless of its age

#### Scenario: GC runs only on leader

- **WHEN** the GC task runs on a Raft follower node
- **THEN** GC SHALL skip execution (KV writes require leadership)
- **AND** GC SHALL log a debug message and retry on next interval

### Requirement: Resource bounds

The commit DAG system SHALL enforce fixed limits to prevent resource exhaustion.

#### Scenario: Commit snapshot exceeds key limit

- **WHEN** a branch has more than `MAX_COMMIT_SNAPSHOT_KEYS` (10,000) dirty entries
- **AND** the branch commits
- **THEN** the commit SHALL still succeed (the branch's own `MAX_BRANCH_DIRTY_KEYS` already enforces this limit)
- **AND** `MAX_COMMIT_SNAPSHOT_KEYS` SHALL equal `MAX_BRANCH_DIRTY_KEYS`

#### Scenario: Branch exceeds max commits

- **WHEN** a branch has produced `MAX_COMMITS_PER_BRANCH` (10,000) commits
- **AND** the branch attempts another commit
- **THEN** the commit SHALL succeed
- **AND** the oldest commits beyond the limit SHALL become eligible for GC regardless of TTL

#### Scenario: GC batch size is bounded

- **WHEN** the GC task finds 50,000 expired commits
- **THEN** GC SHALL process at most `COMMIT_GC_BATCH_SIZE` (1,000) commits per run
- **AND** remaining expired commits SHALL be processed on the next GC interval

### Requirement: Verus-verified commit hash computation

The `compute_commit_id` and `compute_mutations_hash` functions SHALL be pure functions in `src/verified/` with corresponding Verus specs in `verus/`. The Verus specs SHALL prove:

1. **Determinism**: Same inputs produce same CommitId
2. **Chain continuity**: Each CommitId depends on its parent (inherits from Raft chain hash proofs)
3. **Tamper detection**: Modified mutations produce different mutations_hash
4. **Sort invariant**: mutations_hash input is always lexicographically sorted by key

#### Scenario: Verus verification passes

- **WHEN** `nix run .#verify-verus commit-dag` is executed
- **THEN** all specs SHALL verify successfully

#### Scenario: Exec functions match spec functions

- **WHEN** the production `compute_commit_id()` is called with specific inputs
- **THEN** the result SHALL match the Verus `compute_commit_id_spec()` for the same inputs
