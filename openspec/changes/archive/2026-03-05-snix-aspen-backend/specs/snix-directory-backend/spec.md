## ADDED Requirements

### Requirement: DirectoryService get operation

The system SHALL retrieve directory nodes from Aspen Raft KV by their BLAKE3 digest. Directories are stored as protobuf-encoded bytes under the key `snix:dir:<blake3-hex>`.

#### Scenario: Get existing directory

- **WHEN** `get()` is called with a B3Digest of a previously stored directory
- **THEN** the service SHALL return `Ok(Some(directory))` with the decoded Directory node

#### Scenario: Get non-existent directory

- **WHEN** `get()` is called with a B3Digest not present in KV
- **THEN** the service SHALL return `Ok(None)`

### Requirement: DirectoryService put operation

The system SHALL store directory nodes in Aspen Raft KV. The key MUST be derived from the BLAKE3 hash of the protobuf-encoded directory content. The value MUST be the protobuf-encoded bytes.

#### Scenario: Store a directory node

- **WHEN** `put()` is called with a Directory containing file and subdirectory entries
- **THEN** the service SHALL encode the directory as protobuf, compute its BLAKE3 digest, store it at `snix:dir:<digest-hex>`, and return the B3Digest

#### Scenario: Store duplicate directory is idempotent

- **WHEN** `put()` is called with a directory whose BLAKE3 digest already exists in KV
- **THEN** the service SHALL return the same B3Digest without error

### Requirement: DirectoryService get_recursive operation

The system SHALL return a stream of all directories reachable from a root digest, yielding children before parents (post-order).

#### Scenario: Recursive get of directory tree

- **WHEN** `get_recursive()` is called with a root directory digest
- **THEN** the service SHALL yield all descendant directories followed by the root
- **AND** every yielded directory's child directory references SHALL have been yielded earlier in the stream

### Requirement: DirectoryPutter batched writes

The system SHALL accept multiple directories via `put()` calls on a DirectoryPutter and commit them atomically on `close()`.

#### Scenario: Batch put multiple directories

- **WHEN** multiple directories are put via DirectoryPutter and `close()` is called
- **THEN** all directories SHALL be stored in KV and the root digest SHALL be returned

### Requirement: KV key format for directories

The system SHALL use the key format `snix:dir:<lowercase-hex-blake3>` for all directory entries. The hex encoding MUST be lowercase and exactly 64 characters.

#### Scenario: Key format verification

- **WHEN** a directory with known BLAKE3 digest `abc123...` is stored
- **THEN** the KV key SHALL be `snix:dir:abc123...` (64 hex chars, lowercase)
