# Current Aspen Snix Chunking Inventory

Status: captured
Date: 2026-05-08

## Scope

This inventory identifies the current Aspen Snix chunking entrypoints, constants, tests, and call sites so the VectorCDC evaluation can introduce the smallest safe pluggable boundary without changing default behavior.

## Entrypoints and data model

- `crates/aspen-snix/src/chunking.rs`
  - Public function: `chunk_blob(data: &[u8]) -> Vec<Chunk>`.
  - Public output type: `Chunk { hash: blake3::Hash, offset: u64, size: u32 }`.
  - Current algorithm: `fastcdc::v2020::FastCDC::new(data, MIN_CHUNK_SIZE, AVG_CHUNK_SIZE, MAX_CHUNK_SIZE)`.
  - Current hashing: each returned byte range is hashed with `blake3::hash(chunk_data)`.
  - Empty input returns an empty vector.
- `crates/aspen-snix/src/manifest.rs`
  - Manifest entries persist only chunk `hash: [u8; 32]` and `size: u32`.
  - Manifest total size is derived by summing entry sizes.
  - The persisted manifest does not record chunker algorithm, parameters, or offsets; offsets are recoverable from ordered sizes.
- `crates/aspen-snix/src/chunked_blob_service.rs`
  - The blob writer computes the whole raw blob BLAKE3 digest before chunking.
  - Large blobs call `chunking::chunk_blob(&data)` once during `BlobWriter::close`.
  - Each physical chunk is stored by chunk digest when missing; dedup is an `inner.has(chunk_digest)` check.
  - A manifest is stored under `snix:manifest:<whole-blob-digest>`.
  - The whole blob is also stored in the inner service for direct compatibility.
  - `BlobService::chunks()` exposes manifest entries as Snix `ChunkMeta { digest, size }`.
  - Reads reassemble from manifest entries and verify each chunk hash.

## Constants and bounds

- `MIN_CHUNK_SIZE: u32 = 16 * 1024`.
- `AVG_CHUNK_SIZE: u32 = 64 * 1024`.
- `MAX_CHUNK_SIZE: u32 = 256 * 1024`.
- `INLINE_THRESHOLD: u64 = 256 * 1024`; blobs at or below this size bypass chunking.
- `MAX_INLINE_MANIFEST_SIZE: usize = 64 * 1024`; larger manifests are stored as blobs with a KV pointer.

## Current tests

- `chunking.rs`
  - `empty_blob_produces_no_chunks`: empty input has no chunks.
  - `small_blob_single_chunk`: sub-minimum data produces one chunk starting at offset 0 with the BLAKE3 hash of the whole input.
  - `large_blob_multiple_chunks`: chunks cover the full input contiguously and every chunk hash matches its byte range.
  - `chunk_sizes_within_bounds`: non-final chunks respect the configured minimum; all chunks respect the maximum.
  - `deterministic_chunking`: repeated chunking of the same bytes returns identical chunks.
- `chunked_blob_service.rs`
  - Covers small blob pass-through, large blob round-trip, chunk metadata exposure, dedup reuse behavior, hash verification on reassembly, and missing/manifest behavior around the `BlobService` wrapper.
- `manifest.rs`
  - Covers serialization round trips, total-size derivation, inline-size estimation, and many-entry manifests.

## Call-site inventory

Production call sites for `chunking::chunk_blob` are narrow:

1. `crates/aspen-snix/src/chunked_blob_service.rs:297` calls `chunking::chunk_blob(&data)` in `ChunkedBlobWriter::close` after whole-blob digest computation and after the inline-threshold bypass.

Direct test call sites are confined to `crates/aspen-snix/src/chunking.rs` unit tests.

Other crates with `chunking` modules, such as `crates/aspen-fuse`, are separate file-range chunking code paths and are not part of this Snix/castore CDC seam.

## Smallest safe chunker boundary

The first implementation seam should stay inside `crates/aspen-snix/src/chunking.rs`:

1. Introduce a deterministic chunk-boundary abstraction that returns ordered byte ranges or `Chunk` values for `&[u8]`.
2. Keep `chunk_blob(data)` as the default public compatibility wrapper using the existing FastCDC parameters.
3. Add an explicit algorithm/config entrypoint such as `chunk_blob_with(data, &chunker)` or `Chunker::chunk(data)` for benchmark and experimental use.
4. Keep BLAKE3 chunk hashing in one place unless a candidate benchmark proves it must move; this prevents candidate algorithms from changing manifest digest semantics.
5. Do not persist algorithm names or parameters in `Manifest` during the evaluation slice; because blob identity remains the whole raw-content BLAKE3 digest and chunking is storage metadata, parameter metadata is only needed for diagnostics/bench reports unless a future promotion changes re-chunk/read-selection behavior.
6. Keep `ChunkedBlobService::new` and `new_with_arc` behavior unchanged for this slice. A later task can add explicit config injection once benchmark needs are concrete.

## Commands / Oracles

- command: `git status --short --branch`
- command: `search_files` for `chunk_blob|Chunk|MIN_CHUNK_SIZE|AVG_CHUNK_SIZE|MAX_CHUNK_SIZE|INLINE_THRESHOLD` under `crates/aspen-snix`
- command: `search_files` for `aspen_snix::chunking|chunking::|chunk_blob\(` under `crates`
- oracle: source inspection of `crates/aspen-snix/src/chunking.rs`, `chunked_blob_service.rs`, `manifest.rs`, `lib.rs`, and `Cargo.toml`

## Outcomes

- result: pass — Snix/castore CDC use in Aspen has a single production `chunk_blob` caller.
- result: pass — FastCDC parameters and inline/manifest bounds are explicit constants.
- result: pass — existing tests already cover determinism, coverage, bounds, hashing, round-trip behavior, and manifest basics.
- result: pass — the proposed first seam can be local to `chunking.rs` while preserving all current public behavior.
