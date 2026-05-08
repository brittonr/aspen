# Experimental Hashless CDC Candidate Evidence

Status: captured
Date: 2026-05-08

## Scope

This evidence captures the first pluggable chunker implementation slice for `evaluate-vectorcdc-chunking`.

The candidate is intentionally named `experimental-hashless-cdc` rather than `VectorCDC`: it is a deterministic hashless rolling/gear-style CDC prototype that exercises the same local Aspen Snix chunker seam and benchmark harness. It is suitable for boundary, feature-gate, and comparison plumbing, but it is not a production replacement for FastCDC and does not claim VectorCDC paper equivalence.

## Implementation

- Added `ChunkerAlgorithm` and `ChunkingError` in `crates/aspen-snix/src/chunking.rs`.
- Preserved `chunk_blob(data)` as the production default FastCDC path.
- Added `chunk_blob_with_algorithm(data, algorithm)` for explicit evaluation-only selection.
- Added `experimental-vectorcdc` Cargo feature.
- Added `ChunkerAlgorithm::ExperimentalHashlessCdc` behind `experimental-vectorcdc`.
- The candidate uses the same `MIN_CHUNK_SIZE`, `AVG_CHUNK_SIZE`, `MAX_CHUNK_SIZE`, BLAKE3 per-chunk hashes, offset ordering, and coverage invariants as FastCDC.
- The candidate returns `ChunkingError::UnsupportedAlgorithm` when requested without the explicit feature.
- Updated `crates/aspen-snix/benches/chunking_baseline.rs` to select algorithms through `ASPEN_SNIX_CHUNKER` while keeping FastCDC as the default.

## Captured candidate report

Report path: `openspec/changes/evaluate-vectorcdc-chunking/evidence/experimental-hashless-baseline-report.json`

Summary:

- Algorithm: `experimental-hashless-cdc`
- Feature/config required:
  - Cargo feature: `experimental-vectorcdc`
  - Benchmark selection: `ASPEN_SNIX_CHUNKER=experimental-hashless-cdc`
- Corpus:
  - files: `42`
  - bytes: `17090841`
  - classes:
    - `aspen-build-artifact`: `17` files, `2061817` bytes
    - `nix-store`: `16` files, `10295044` bytes
    - `synthetic-small-delta`: `9` files, `4733980` bytes
- Results:
  - wall time: `17314` microseconds
  - throughput: `941.3640941676725` MiB/s
  - total chunks: `230`
  - unique chunks: `224`
  - duplicate chunks: `6`
  - dedup ratio: `0.02608695652173913`
  - chunk size min/p50/p95/max: `14` / `53797` / `200271` / `262144`

## Current comparison note

A refreshed FastCDC run from the same harness reported:

- throughput: `1490.328525564637` MiB/s
- total chunks: `253`
- duplicate chunks: `42`
- dedup ratio: `0.16600790513833993`

The experimental hashless candidate was slower in this local run (`~0.63x` FastCDC throughput) and showed materially lower duplicate chunk reuse. This is enough to validate the pluggable evaluation seam and to reject immediate promotion, but not enough to make a final algorithm-family decision; a real VectorCDC implementation or tuned hashless candidate still needs separate evidence.

## Commands / Oracles

- command: `nix run .#rustfmt`
- command: `cargo test -p aspen-snix chunking --lib`
- command: `cargo test -p aspen-snix chunking --lib --features experimental-vectorcdc`
- command: `cargo bench -p aspen-snix --bench chunking_baseline --no-run`
- command: `cargo bench -p aspen-snix --bench chunking_baseline --features experimental-vectorcdc --no-run`
- command: `ASPEN_SNIX_CHUNKER=experimental-hashless-cdc cargo bench -p aspen-snix --bench chunking_baseline --features experimental-vectorcdc > openspec/changes/evaluate-vectorcdc-chunking/evidence/experimental-hashless-baseline-report.json`
- command: `cargo bench -p aspen-snix --bench chunking_baseline > openspec/changes/evaluate-vectorcdc-chunking/evidence/fastcdc-baseline-report.json`
- oracle: default `chunk_blob` still matches explicit `ChunkerAlgorithm::FastCdc`.
- oracle: requesting `ExperimentalHashlessCdc` without `experimental-vectorcdc` returns `UnsupportedAlgorithm`.
- oracle: with `experimental-vectorcdc`, candidate chunks preserve coverage, contiguity, bounds, hash, and determinism invariants.

## Outcomes

- result: pass — FastCDC remains the default production path.
- result: pass — experimental candidate is isolated behind explicit feature/config selection.
- result: pass — no new external dependency was added for the prototype.
- result: pass — focused default and feature-enabled chunking tests pass.
- result: pass — both default and feature-enabled benchmark binaries compile.
- result: pass — candidate benchmark report emits valid JSON and can be compared with FastCDC baseline fields.
