# Tasks: Pin the fabric boundary compatibility fixtures

## Phase 1: Implementation

- [x] [serial] Write the shared explicit inputs, and generate the fixtures with a throwaway generator at `3de348149^`. Record the generator, commit, and hashes. a[pin-fabric-boundary-compatibility-fixtures.premigration]
- [x] [serial] Add the fixture-equality test and the impl and verify markers. a[pin-fabric-boundary-compatibility-fixtures.equality] a[pin-fabric-boundary-compatibility-fixtures.traceability]

## Phase 2: Validation

- [x] [serial] Positive: the focused test shows byte and ref equality for all eight fixtures. a[pin-fabric-boundary-compatibility-fixtures.equality]
- [x] [serial] Negative: one-field mutations change every ref, and tampered or truncated fixtures are rejected. a[pin-fabric-boundary-compatibility-fixtures.sensitivity]
- [x] [serial] Run fmt, clippy with `-D warnings`, `cargo test --workspace`, and `inherited-tracey-debt`, and record the counts. a[pin-fabric-boundary-compatibility-fixtures.traceability]
- [x] [serial] Run pinned Octet root and lib against the base, and repair every new finding without allows until the per-lint delta is zero. a[pin-fabric-boundary-compatibility-fixtures.octet]
