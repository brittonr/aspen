# Marble Object Store Specification

## Purpose

Spike a marble-plus-art physical backend behind the Molten storage seam so the storage-path decision rests on measured comparison, not preference.

## Requirements

### Requirement: The backend stays seam-bound and reversible [r[aspen.marble_store.spike]]

The backend MUST be optional, configuration-selected, and confined behind the storage seam. Blocking operations MUST stay inside a bounded executor, and maintenance scheduling MUST stay inside the backend. A measured keep-or-replace decision MUST be recorded.

#### Scenario: Default path unchanged

- GIVEN the spike backend is not selected
- WHEN the runtime operates
- THEN behavior MUST be identical to the current path

#### Scenario: Decision is measured

- GIVEN campaign workloads run against both paths
- WHEN the spike closes
- THEN the decision record MUST contain write, read, recovery, and space numbers

### Requirement: BLAKE3 stays the only content identity [r[aspen.marble_store.identity]]

ObjectIds MUST remain private physical handles. Identical digests MUST resolve to one logical object. In-flight batch mappings MUST be served from the backend cache until `write_batch` returns, and only atomically recovered batches MUST replay after a crash.

#### Scenario: Digest round-trip

- GIVEN an object stored through the backend
- WHEN its BLAKE3 digest is resolved and fetched
- THEN the fetched bytes MUST match the stored bytes

#### Scenario: Interrupted batch

- GIVEN a batch interrupted before commit
- WHEN recovery runs
- THEN no mappings from that batch MUST exist

### Requirement: The art index covers fixed-length digests [r[aspen.marble_store.art_index]]

The digest-to-ObjectId index MUST use fixed 32-byte keys and MUST fail lookup for absent digests.

#### Scenario: Absent digest

- GIVEN a digest never stored
- WHEN lookup runs
- THEN the lookup MUST fail

### Requirement: Revisions are pinned [r[aspen.marble_store.pinning]]

Reviewed marble and art revisions MUST be pinned and recorded as transport `crates.io`, plane `implementation`.

#### Scenario: Dependency classification

- GIVEN the workspace dependency graph after adoption
- WHEN classification runs
- THEN both pins MUST appear on implementation edges

### Requirement: Claims stay bounded [r[aspen.marble_store.boundary]]

Documentation MUST state that the spike changes no runtime semantics and claims nothing beyond recorded measurements.

#### Scenario: Over-claim rejected

- GIVEN documentation claims runtime superiority without recorded numbers
- WHEN boundary verification runs
- THEN the claim MUST fail verification

### Requirement: The spike is verified [r[aspen.marble_store.verification]]

Positive and negative fixtures MUST cover round-trips, absent digests, interrupted batches, and exhaustion, with ChaosControl campaign references recorded.

#### Scenario: Complete matrix passes

- GIVEN backend fixtures and recorded campaign references
- WHEN package, workspace, Clippy, Cairn, and Nix checks run
- THEN round-trips MUST hold and failure cases MUST classify as declared
