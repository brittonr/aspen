## ADDED Requirements

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
