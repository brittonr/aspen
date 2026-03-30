## ADDED Requirements

### Requirement: Exporter batches are dependency-closed

The federation git object exporter SHALL produce batches where every object's dependencies are either present in the same batch or present in the receiver's `have_set` (known objects). A tree object SHALL NOT appear in a batch unless all blob/tree hashes it references are resolvable by the receiver.

#### Scenario: Single batch fits entire DAG

- **WHEN** a repo has fewer objects than the batch limit
- **THEN** all objects are returned in one batch with `has_more = false`
- **AND** every tree's referenced blobs are in the batch

#### Scenario: DAG exceeds batch limit

- **WHEN** a repo has more objects than the batch limit (e.g., 5000)
- **THEN** the exporter returns a batch smaller than or equal to the limit
- **AND** every tree in the batch has all its referenced blobs either in the batch or in `known_blake3`
- **AND** `has_more = true` signals the caller to request another batch

#### Scenario: Incremental sync with have_set

- **WHEN** the receiver already has some objects (non-empty `have_set`)
- **THEN** objects in `have_set` are excluded from the batch
- **AND** trees referencing objects in `have_set` are still valid (those dependencies are satisfied externally)

### Requirement: Importer retries objects with unresolved dependencies

The federation git object importer SHALL retry objects that fail due to missing hash mappings after the primary import pass completes. Objects that succeed on retry SHALL be included in the import statistics.

#### Scenario: All objects resolve on first pass

- **WHEN** a dependency-closed batch is imported
- **THEN** all objects are imported on the first pass
- **AND** the retry pass imports zero additional objects

#### Scenario: Import order causes temporary failures

- **WHEN** two trees in a batch reference each other's subtrees
- **THEN** at least one fails on the first pass
- **AND** the retry pass resolves the remaining objects
- **AND** the final import count includes both passes

#### Scenario: Genuinely missing dependency

- **WHEN** an object's dependency is not in the batch and not previously imported
- **THEN** the object fails on both passes
- **AND** the failure is recorded in `stats.errors` as a non-fatal error

### Requirement: Multi-batch federation sync produces complete mirror

The full federation sync loop (multiple batches with incremental `have_set`) SHALL produce a mirror repo containing all objects from the source repo. After sync completes, a git clone from the mirror SHALL succeed.

#### Scenario: Large repo syncs across multiple batches

- **WHEN** a repo with 6000+ objects is synced via federation
- **AND** the batch limit forces at least 2 rounds
- **THEN** the mirror contains all source objects
- **AND** ref heads point to valid commits with fully resolvable DAGs

#### Scenario: Subsequent sync is incremental

- **WHEN** a second sync runs after a successful first sync
- **THEN** only new objects (not in the mirror's `have_set`) are transferred
- **AND** the batch count is zero or minimal
