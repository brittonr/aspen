# snix-store Specification

## Purpose

Defines the Snix Store capability requirements preserved by Aspen's archived OpenSpec records, including blob size bound, directory entry bound, directory depth bound.

## Requirements

### Requirement: Blob size bound

Blob uploads SHALL reject data exceeding `MAX_BLOB_SIZE_BYTES` (1 GB).

#### Scenario: Blob size bound is enforced

- **WHEN** a blob upload exceeds `MAX_BLOB_SIZE_BYTES`
- **THEN** the service SHALL reject the blob before unbounded storage occurs

### Requirement: Directory entry bound

Directory storage SHALL enforce a maximum of `MAX_DIRECTORY_ENTRIES` (100,000) entries per directory.

#### Scenario: Directory entry bound is enforced

- **WHEN** a directory payload has more than `MAX_DIRECTORY_ENTRIES` entries
- **THEN** directory storage SHALL reject the payload

### Requirement: Directory depth bound

Recursive directory traversal SHALL enforce `MAX_DIRECTORY_DEPTH` (256) to prevent unbounded recursion.

#### Scenario: Directory depth bound is enforced

- **WHEN** directory traversal exceeds `MAX_DIRECTORY_DEPTH`
- **THEN** the traversal SHALL fail before unbounded recursion occurs

### Requirement: Path reference bound

PathInfo entries SHALL have at most `MAX_PATH_REFERENCES` (10,000) store path references.

#### Scenario: Path reference bound is enforced

- **WHEN** a PathInfo entry contains too many references
- **THEN** the service SHALL reject the PathInfo entry

### Requirement: Signature bound

PathInfo entries SHALL have at most `MAX_SIGNATURES` (100) signatures.

#### Scenario: Signature bound is enforced

- **WHEN** a PathInfo entry contains too many signatures
- **THEN** the service SHALL reject the PathInfo entry

### Requirement: Chunk size bounds

Content-defined chunks SHALL have sizes between `MIN_CHUNK_SIZE` (16 KiB) and `MAX_CHUNK_SIZE` (256 KiB), except the final chunk which may be smaller.

#### Scenario: Chunk size bounds is enforced

- **WHEN** chunked blob storage emits or receives a non-final chunk outside the configured bounds
- **THEN** the service SHALL reject or avoid that chunk shape

### Requirement: Manifest entry bound

Chunk manifests SHALL have at most `ceil(blob_size / MIN_CHUNK_SIZE)` entries.

#### Scenario: Manifest entry bound is enforced

- **WHEN** a chunk manifest contains more entries than the blob size permits
- **THEN** the service SHALL reject the manifest

### Requirement: Circuit breaker protection

All snix service calls (blob, directory, pathinfo) SHALL be protected by circuit breakers that reject requests after consecutive failures exceed a threshold.

#### Scenario: Circuit breaker protection is enforced

- **WHEN** a snix service exceeds its consecutive failure threshold
- **THEN** the circuit breaker SHALL reject subsequent requests according to policy
