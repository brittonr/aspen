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

### Requirement: Pluggable CDC chunker evaluation [r[snix-store.pluggable-cdc-chunker-evaluation]]
Snix-backed blob chunking MUST expose an internal deterministic chunker boundary that can run the existing FastCDC implementation and optional experimental CDC candidates without changing raw-content BLAKE3 blob identity.

#### Scenario: FastCDC remains the default [r[snix-store.pluggable-cdc-chunker-evaluation.fastcdc-default]]
- GIVEN the experimental chunker feature/config is not enabled
- WHEN a blob is chunked through the canonical Aspen Snix chunking path
- THEN the implementation SHALL use the existing FastCDC behavior and bounds
- AND the raw blob digest SHALL remain the BLAKE3 hash of the complete unchunked blob contents

#### Scenario: Candidate chunker preserves chunk invariants [r[snix-store.pluggable-cdc-chunker-evaluation.candidate-invariants]]
- GIVEN a VectorCDC-style or other experimental chunker candidate is enabled for measurement
- WHEN it chunks a non-empty blob
- THEN the emitted chunks SHALL be deterministic, contiguous, ordered, non-overlapping, and cover the full blob
- AND every emitted chunk hash SHALL be the BLAKE3 hash of that chunk's raw bytes
- AND every non-final chunk SHALL satisfy the configured chunk-size bounds

### Requirement: CDC benchmark evidence [r[snix-store.cdc-benchmark-evidence]]
Any proposed VectorCDC-style adoption MUST include reproducible benchmark evidence comparing it against the current FastCDC baseline on representative Nix/Snix/Aspen corpora before it can become a default or recommended configuration.

#### Scenario: Baseline benchmark captures full chunking cost [r[snix-store.cdc-benchmark-evidence.fastcdc-baseline]]
- GIVEN the benchmark corpus is available locally
- WHEN the FastCDC baseline benchmark runs
- THEN the evidence SHALL report CDC throughput, total chunking wall time, chunk count, chunk-size distribution, BLAKE3 hashing time if separable, and manifest/object-count impact

#### Scenario: Candidate benchmark compares deduplication behavior [r[snix-store.cdc-benchmark-evidence.candidate-dedup-comparison]]
- GIVEN a candidate VectorCDC-style implementation is measured on the same corpus
- WHEN benchmark results are compared against FastCDC
- THEN the evidence SHALL report relative throughput, byte-level dedup/reuse ratio, reused chunk count, changed-boundary count where available, and any compression/object-store regressions

#### Scenario: Promotion requires positive end-to-end evidence [r[snix-store.cdc-benchmark-evidence.promotion-gate]]
- GIVEN a reviewer proposes changing the default chunker or recommending a VectorCDC-style candidate
- WHEN promotion evidence is evaluated
- THEN the evidence SHALL show a material end-to-end benefit on representative corpus data without reducing deduplication space savings beyond an explicitly accepted threshold
- AND unsupported CPU/platform behavior SHALL be documented with either scalar fallback evidence or a fail-closed configuration gate

### Requirement: Chunker research remains optional [r[snix-store.chunker-research-optional]]
VectorCDC-style research code MUST remain optional and isolated until benchmark, portability, and compatibility evidence justifies promotion.

#### Scenario: Default builds avoid experimental dependencies [r[snix-store.chunker-research-optional.default-deps]]
- WHEN default Aspen Snix builds resolve dependencies
- THEN experimental VectorCDC or SIMD-specific candidate dependencies SHALL NOT be required

#### Scenario: Candidate failure does not corrupt storage [r[snix-store.chunker-research-optional.fail-safe]]
- GIVEN an experimental chunker candidate is unavailable, unsupported, or returns invalid metadata
- WHEN chunking is requested through the Aspen Snix path
- THEN the system SHALL fail closed or fall back only through an explicitly configured policy
- AND it SHALL NOT persist manifests whose chunks violate the chunk invariants
