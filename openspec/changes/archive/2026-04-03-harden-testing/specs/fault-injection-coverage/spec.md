## ADDED Requirements

### Requirement: Fault injection at redb write boundaries

The system SHALL include buggify fault injection points at redb write operations to simulate storage failures during tests.

#### Scenario: Corrupted write during Raft log append

- **WHEN** buggify is enabled during a madsim test
- **AND** a Raft log append write occurs
- **THEN** the buggify point SHALL be capable of returning a simulated I/O error
- **AND** the Raft node SHALL handle the error without panicking

#### Scenario: Partial fsync during snapshot persist

- **WHEN** buggify triggers during a snapshot fsync
- **THEN** the node SHALL detect the incomplete snapshot on recovery
- **AND** SHALL fall back to log replay or request a fresh snapshot from the leader

### Requirement: Fault injection at Iroh connection boundaries

The system SHALL include buggify fault injection points at Iroh QUIC connection establishment and stream creation.

#### Scenario: Connection timeout during Raft RPC

- **WHEN** buggify triggers a connection timeout on an AppendEntries RPC
- **THEN** the sending node SHALL retry with backoff
- **AND** the cluster SHALL remain available if a majority of nodes are reachable

#### Scenario: Stream reset mid-transfer

- **WHEN** buggify triggers a stream reset during snapshot transfer
- **THEN** the receiving node SHALL discard the partial snapshot
- **AND** SHALL re-request the snapshot from the leader

### Requirement: Fault injection at blob transfer boundaries

The system SHALL include buggify fault injection points at blob download and upload operations.

#### Scenario: Truncated blob download

- **WHEN** buggify triggers a truncated read during blob download
- **THEN** the receiver SHALL detect the hash mismatch
- **AND** SHALL retry the download or report the failure

#### Scenario: Hash mismatch on blob upload

- **WHEN** buggify corrupts blob data during upload
- **THEN** the receiving node SHALL reject the blob
- **AND** SHALL NOT store the corrupted data

### Requirement: Fault injection compiles to no-op in release

All buggify fault injection points SHALL compile to no-ops when the `testing` feature flag is not enabled. There SHALL be zero runtime overhead in production builds.

#### Scenario: Release build has no fault injection overhead

- **WHEN** the crate is compiled without `--features testing`
- **THEN** all buggify callsites SHALL be eliminated by the compiler
- **AND** no atomic loads or branch checks SHALL remain in the generated code
