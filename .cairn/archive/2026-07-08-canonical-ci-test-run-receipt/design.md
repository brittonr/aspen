## Context

`flake.nix` runs nextest in a hermetic check and copies `cargo-metadata.json`, `binaries-metadata.json`, and `junit.xml` to the Nix output. Dogfood release evidence already depends on nextest output paths. A canonical receipt can bind those pieces for release readback.

## Design

Define `ci-test-run-receipt-v1` or equivalent canonical Preserves artifact with fields for:

- source/tree ref or Nix source marker;
- profile id and command surface;
- nextest config ref;
- Cargo metadata ref;
- binaries metadata ref;
- test binary/package refs when available;
- rendered JUnit ref;
- counts and final decision;
- diagnostics and caveats.

A pure core builds the receipt from typed metadata refs and validates consistency. The Nix shell or CLI shell computes file refs, invokes nextest, checks output presence, and writes the receipt.

## Validation

Positive tests validate a complete receipt. Negative tests reject missing Cargo metadata, missing binaries metadata, missing JUnit, stale profile id, mismatched counts, and rendered JUnit presented without canonical receipt metadata.
