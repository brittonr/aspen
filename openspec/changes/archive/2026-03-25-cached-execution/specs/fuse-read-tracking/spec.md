## ADDED Requirements

### Requirement: Per-PID read set tracking

When cached execution is enabled on a FUSE mount, AspenFs SHALL maintain a per-PID read set that records every file path and its content hash (BLAKE3) accessed via `open()` and `read()` operations. The read set SHALL be stored in a concurrent map keyed by PID.

#### Scenario: File read is recorded

- **WHEN** cached execution tracking is enabled
- **AND** PID 1234 opens and reads file `/src/main.rs` with BLAKE3 hash H1
- **THEN** the read set for PID 1234 SHALL contain the entry (path="/src/main.rs", hash=H1)

#### Scenario: Multiple files recorded per PID

- **WHEN** PID 1234 reads files `/src/main.rs` (hash H1) and `/src/lib.rs` (hash H2)
- **THEN** the read set for PID 1234 SHALL contain both entries

#### Scenario: Duplicate reads are deduplicated

- **WHEN** PID 1234 reads `/src/main.rs` twice
- **THEN** the read set for PID 1234 SHALL contain exactly one entry for that path

#### Scenario: Tracking disabled means no overhead

- **WHEN** cached execution tracking is NOT enabled on the mount
- **THEN** no read set SHALL be maintained
- **AND** the `read()` path SHALL have zero additional overhead

### Requirement: Session lifecycle management

A tracking session SHALL be created when a tracked process begins and finalized when the process exits. Sessions SHALL have a maximum lifetime to prevent leaks from zombie processes.

#### Scenario: Session created on process start

- **WHEN** a new process with PID 5678 issues its first file operation on a tracked mount
- **AND** the process is a child of a tracked session
- **THEN** a new tracking session SHALL be created for PID 5678

#### Scenario: Session finalized on process exit

- **WHEN** PID 5678's tracking session is active
- **AND** PID 5678 exits
- **THEN** the session's read set SHALL be finalized and made available for cache key computation
- **AND** session resources SHALL be released

#### Scenario: Stale session cleanup

- **WHEN** a tracking session has been active for longer than the session timeout (default 5 minutes)
- **AND** the tracked PID no longer exists in /proc
- **THEN** the session SHALL be cleaned up
- **AND** no cache entry SHALL be created for the stale session

### Requirement: Child process inheritance

When a tracked process forks, the child process SHALL automatically receive its own tracking session linked to the parent session. The child's read set SHALL be independent from the parent's.

#### Scenario: Fork creates child session

- **WHEN** tracked PID 1000 forks child PID 1001
- **AND** PID 1001 issues a file read on the tracked mount
- **THEN** PID 1001 SHALL have its own tracking session
- **AND** PID 1001's read set SHALL be independent from PID 1000's

#### Scenario: Parent records child cache keys

- **WHEN** PID 1000 has child sessions for PIDs 1001 and 1002
- **AND** both children complete and produce cache keys K1 and K2
- **THEN** PID 1000's session metadata SHALL reference K1 and K2 as child dependencies

### Requirement: Content hash source

File content hashes SHALL be sourced from iroh-blobs when available (zero-cost, already computed). For files not backed by iroh-blobs, the content SHALL be hashed with BLAKE3 on read.

#### Scenario: Blob-backed file uses existing hash

- **WHEN** a file is backed by an iroh-blob with known BLAKE3 hash H
- **AND** the file is read by a tracked process
- **THEN** hash H SHALL be recorded in the read set without re-hashing the content

#### Scenario: Non-blob file is hashed on read

- **WHEN** a file is not backed by an iroh-blob (e.g., generated during the build)
- **AND** the file is read by a tracked process
- **THEN** the file content SHALL be hashed with BLAKE3
- **AND** the resulting hash SHALL be recorded in the read set

### Requirement: Read tracking resource bounds

Read tracking SHALL enforce per-session limits: maximum tracked files per session (100,000), maximum concurrent sessions (10,000). Exceeding limits SHALL disable tracking for the affected session without affecting process execution.

#### Scenario: Session exceeds file limit

- **WHEN** a tracked session has recorded 100,000 file reads
- **AND** the process reads another file
- **THEN** tracking SHALL be disabled for that session
- **AND** no cache entry SHALL be created for that session
- **AND** the process SHALL continue executing normally
