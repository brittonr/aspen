## ADDED Requirements

### Requirement: DocsExporter includes commit metadata

When the `commit-dag-federation` feature is enabled, the `DocsExporter` SHALL export commit metadata as KV entries alongside normal data entries. For each `BranchOverlay.commit()` that produces a `CommitId`, the exporter SHALL emit the serialized `Commit` at key `_sys:commit:{commit_id_hex}` and the branch tip update at `_sys:commit-tip:{branch_id}`.

#### Scenario: Commit metadata exported with data entries

- **WHEN** a branch commits mutations `{a: Set("1"), b: Set("2")}` producing CommitId `C1`
- **AND** the DocsExporter processes the resulting Raft log entries
- **THEN** the exporter SHALL emit entries for keys "a", "b", `_sys:commit:{C1_hex}`, and `_sys:commit-tip:{branch_id}`

#### Scenario: Commit metadata flows through existing export pipeline

- **WHEN** commit metadata entries are exported
- **THEN** they SHALL use the same `DocsWriter` trait and batch pipeline as regular KV entries
- **AND** no changes to the `DocsWriter` trait SHALL be required

#### Scenario: Backward compatibility without feature

- **WHEN** the `commit-dag-federation` feature is NOT enabled
- **THEN** the DocsExporter SHALL export only regular KV entries
- **AND** `_sys:commit:` entries written to KV SHALL still be exported as regular KV entries (pass-through)

### Requirement: DocsImporter verifies commit chain integrity

When the `commit-dag-federation` feature is enabled, the `DocsImporter` SHALL verify commit chain integrity before applying data entries that are associated with a commit. Verification SHALL recompute the `mutations_hash` from the commit's stored mutations and verify it matches the `mutations_hash` field. The importer SHALL also verify the parent chain link is valid (parent commit exists or is the genesis hash).

#### Scenario: Valid commit passes verification

- **WHEN** the importer receives a commit entry at `_sys:commit:{C1_hex}`
- **AND** the commit's `mutations_hash` matches the recomputed hash from its stored mutations
- **AND** the commit's parent is either `None` (genesis) or a previously verified commit
- **THEN** the importer SHALL accept the commit and apply its associated data entries

#### Scenario: Tampered mutations detected

- **WHEN** the importer receives a commit entry at `_sys:commit:{C1_hex}`
- **AND** the commit's `mutations_hash` does NOT match the recomputed hash from its stored mutations
- **THEN** the importer SHALL reject the commit
- **AND** the importer SHALL NOT apply any data entries associated with this commit
- **AND** the importer SHALL log a warning with the source cluster ID and commit ID

#### Scenario: Missing parent commit

- **WHEN** the importer receives a commit with `parent = Some(P1)`
- **AND** commit `P1` has not been received or verified
- **THEN** the importer SHALL buffer the commit until the parent arrives (up to `COMMIT_IMPORT_BUFFER_TIMEOUT`)
- **AND** if the parent does not arrive within the timeout, the importer SHALL accept the commit with a warning (eventual consistency — parent may arrive later or may have been GC'd on source)

#### Scenario: Backward compatibility with non-commit data

- **WHEN** the importer receives regular KV entries without associated commit metadata
- **THEN** the importer SHALL apply them using the existing priority-based conflict resolution
- **AND** no commit verification SHALL be performed

### Requirement: Batch atomicity via commit grouping

When the `commit-dag-federation` feature is enabled, the `DocsImporter` SHALL group data entries by their associated CommitId and apply each group atomically via `WriteCommand::Batch` or `SetMulti`. The association is determined by matching data entry keys against the commit's `mutations` field.

#### Scenario: Commit entries applied atomically

- **WHEN** commit `C1` has mutations `{a: Set("1"), b: Set("2"), c: Delete}`
- **AND** the importer has received entries for keys "a", "b", and the commit metadata for `C1`
- **THEN** the importer SHALL apply `Set(a, "1")`, `Set(b, "2")`, `Delete(c)` as a single `SetMulti` operation

#### Scenario: Partial batch buffered until complete

- **WHEN** commit `C1` has mutations for keys "a", "b", "c"
- **AND** the importer has received entries for "a" and "b" but not "c"
- **THEN** the importer SHALL buffer "a" and "b" until "c" arrives (up to `COMMIT_IMPORT_BUFFER_TIMEOUT`)
- **AND** if "c" does not arrive within the timeout, the importer SHALL apply the partial batch with a warning

#### Scenario: Entries without commit applied individually

- **WHEN** data entries arrive without any associated commit metadata
- **THEN** the importer SHALL apply them individually using the existing entry-by-entry import path

### Requirement: Federation commit provenance tracking

When a commit is imported from a federated peer, the system SHALL record provenance metadata linking the imported commit to its source cluster. The provenance SHALL be stored alongside the commit at `_sys:commit-origin:{commit_id_hex}` containing the source cluster ID, import timestamp, and verification result.

#### Scenario: Provenance recorded on successful import

- **WHEN** commit `C1` is imported from cluster "cluster-west"
- **AND** chain verification succeeds
- **THEN** `_sys:commit-origin:{C1_hex}` SHALL contain `{source: "cluster-west", verified: true, imported_at_ms: <timestamp>}`

#### Scenario: Provenance records verification failure

- **WHEN** commit `C1` is imported from cluster "cluster-east"
- **AND** mutations_hash verification fails
- **THEN** `_sys:commit-origin:{C1_hex}` SHALL contain `{source: "cluster-east", verified: false, imported_at_ms: <timestamp>, reason: "mutations_hash_mismatch"}`

#### Scenario: Provenance queryable for audit

- **WHEN** a caller queries provenance for CommitId `C1`
- **THEN** the system SHALL return the source cluster, verification status, and import timestamp
- **AND** this enables auditing which cluster produced which state and whether it was verified

### Requirement: Resource bounds for federation commit import

The importer SHALL enforce bounds on commit buffering and verification to prevent resource exhaustion from a misbehaving federated peer.

#### Scenario: Buffer size bounded

- **WHEN** the import buffer for pending commit entries exceeds `MAX_COMMIT_IMPORT_BUFFER` (1,000 entries)
- **THEN** the oldest buffered entries SHALL be dropped
- **AND** a warning SHALL be logged

#### Scenario: Verification timeout bounded

- **WHEN** commit chain verification takes longer than `COMMIT_VERIFY_TIMEOUT_MS` (5,000ms)
- **THEN** verification SHALL be aborted
- **AND** the commit SHALL be accepted with `verified: false` in provenance

#### Scenario: Per-peer commit rate bounded

- **WHEN** a federated peer sends more than `MAX_COMMITS_PER_PEER_PER_MINUTE` (100) commits in one minute
- **THEN** excess commits SHALL be dropped
- **AND** the importer SHALL log a rate-limit warning for that peer
