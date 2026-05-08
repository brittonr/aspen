## Why

Aspen's Snix-backed blob storage already uses content-defined chunking for chunk-level reuse, but the current FastCDC seam is fixed in code and lacks corpus-specific evidence that the chosen algorithm and bounds are optimal for Nix-store-like data. VectorCDC claims large CDC throughput gains for hashless algorithms without reducing deduplication savings, and castore-style chunking stays below the raw-content BLAKE3 blob identity, making it safe to evaluate without changing blob hashes.

## What Changes

- Add an experimental chunker evaluation track for FastCDC baselines and VectorCDC-style candidates.
- Require a pluggable chunker boundary that preserves existing FastCDC behavior as the default.
- Require benchmark evidence over representative Nix/Snix/Aspen corpora before adopting any new default.
- Keep VectorCDC as an optional research candidate until throughput, deduplication, and operational costs are proven.

## Capabilities

### Modified Capabilities
- `snix-store`: Adds chunker selection, benchmark, and promotion requirements for CDC algorithm evaluation.

## Impact

- **Files**: Likely touches `crates/aspen-snix/src/chunking.rs`, benchmark fixtures/scripts, docs, and OpenSpec evidence during implementation.
- **APIs**: May add internal chunker traits/config and feature-gated experimental candidate implementations; no public blob digest format change is allowed.
- **Dependencies**: Candidate VectorCDC code or crates must remain optional until adopted by evidence.
- **Testing**: FastCDC parity tests, chunk-bound negative tests, deterministic benchmark reports, dedup-ratio comparisons, and `cargo test -p aspen-snix`/targeted benches.
