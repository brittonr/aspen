## ADDED Requirements

### Requirement: Pre-populated origin SHA-1 mappings

Before running the convergent import loop, the system SHALL pre-populate SHA-1 → BLAKE3 mappings for all incoming objects that carry `origin_sha1` metadata.

#### Scenario: Origin SHA-1 available before dependency resolution

- **WHEN** a federation sync delivers 34,000 objects with `origin_sha1` set on each SyncObject
- **THEN** the mapping store contains a SHA-1 entry for each object's `origin_sha1` before the first `import_objects` call
- **AND** `has_sha1` returns true for any `origin_sha1` referenced by a tree entry in the batch

### Requirement: Raw-bytes import preserves SHA-1 identity

The importer SHALL accept raw git object bytes and store them without re-serialization, preserving the original SHA-1 hash.

#### Scenario: Blob imported with original bytes

- **WHEN** a blob SyncObject with content bytes `B` is imported via raw-bytes path
- **THEN** the stored object has SHA-1 equal to `sha1("blob len\0" + B)`
- **AND** the SHA-1 matches the origin cluster's SHA-1 for the same object

#### Scenario: Tree imported with original bytes

- **WHEN** a tree SyncObject with content bytes `T` is imported via raw-bytes path
- **THEN** the stored tree's SHA-1 matches `sha1("tree len\0" + T)`
- **AND** no re-serialization of tree entries occurs

### Requirement: Convergent import reaches zero stuck objects

The convergent import loop SHALL import all valid objects, leaving zero stuck objects when all dependencies are present in the batch.

#### Scenario: Full convergence on 34K objects

- **WHEN** `federation_import_objects` receives 34,000 objects (blobs, trees, commits) with complete dependency closure
- **THEN** the convergent loop imports all 34,000 objects
- **AND** `stats.errors` is empty

#### Scenario: Final retry pass breaks mapping deadlocks

- **WHEN** the convergent loop stalls with N > 0 stuck objects that have SHA-1 validated content
- **THEN** a final retry pass attempts those objects with relaxed dependency checking
- **AND** objects whose dependencies exist (just unmapped) are imported successfully
