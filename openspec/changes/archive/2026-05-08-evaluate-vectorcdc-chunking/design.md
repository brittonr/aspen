## Context

`crates/aspen-snix/src/chunking.rs` currently chunks blobs with `fastcdc::v2020::FastCDC` using fixed constants: 16 KiB min, 64 KiB average, and 256 KiB max. The canonical `snix-store` spec already defines chunk bounds, but it does not require benchmark evidence, chunker configurability, or a safe path for comparing alternative CDC algorithms.

Upstream Snix castore keeps chunking below the raw BLAKE3 blob identity: `Node::File` records the raw file digest and physical chunking is a storage/transport concern. That same boundary is the safety rail for this change: candidates may change physical chunk layout but must not change raw blob identity, manifests' hash meaning, or trust semantics.

## Goals / Non-Goals

**Goals:**
- Make FastCDC the measured baseline rather than an assumed optimum.
- Add a narrow internal chunker abstraction/config seam suitable for benchmarks and optional candidates.
- Evaluate VectorCDC-style hashless CDC candidates on representative Nix/Snix/Aspen corpora.
- Gate default adoption on corpus evidence, not paper claims alone.

**Non-Goals:**
- Do not change BLAKE3 blob identity or directory/pathinfo semantics.
- Do not require VectorCDC in default builds before evidence exists.
- Do not claim compatibility with IPFS/IPLD chunk roots; Aspen/Snix chunks remain below the blob identifier.
- Do not optimize object-store composition or split BlobStore/ChunkStore in this change unless needed for measurement harnesses.

## Decisions

### 1. Preserve FastCDC as the default

**Choice:** Existing FastCDC constants and output behavior remain the default until a later evidence-backed promotion changes them.

**Rationale:** Current behavior is tested and reflected by existing chunk-size requirements. A research spike should not silently change storage layout or operational behavior.

**Alternative:** Replace FastCDC immediately with VectorCDC. Rejected because VectorCDC accelerates specific hashless CDC families rather than FastCDC directly, and Nix-store corpus evidence is not yet available.

### 2. Introduce a narrow chunker boundary

**Choice:** Implementation should define a small deterministic chunker interface that returns ordered chunk offset/size/hash metadata and can be backed by FastCDC or optional candidates.

**Rationale:** The benchmark harness and adoption gates need apples-to-apples comparisons without entangling blob identity, manifest persistence, compression, or object-store I/O.

**Implementation:** Keep the pure chunking logic independent from upload/storage side effects. Candidate chunkers must satisfy contiguity, bounds, determinism, final-chunk, and BLAKE3 chunk-hash invariants.

### 3. Benchmark total ingest and dedup effects, not just CDC throughput

**Choice:** Evidence must include raw CDC throughput plus end-to-end ingest context: chunk-count distribution, dedup/reuse ratio, total hashing/compression time, and object-count impact.

**Rationale:** VectorCDC can be compelling if CDC is the bottleneck. If zstd, BLAKE3, object-store roundtrips, or manifest overhead dominate, changing CDC may have low ROI or hurt storage behavior.

### 4. Keep candidate adoption feature-gated and reversible

**Choice:** VectorCDC-style implementations remain behind explicit feature/config flags until promotion evidence passes.

**Rationale:** Different CPU SIMD support and algorithm families create portability and reproducibility risks. Operators need a fail-safe default and a clear evidence bundle before default changes.

## Risks / Trade-offs

**Different chunk boundaries reduce reuse** → Compare dedup ratio and cross-version chunk reuse against FastCDC on representative corpora before promotion.

**SIMD portability drift** → Require scalar fallback or explicit unsupported-platform behavior in candidate evidence.

**Benchmark overfits synthetic data** → Include real Nix store paths, Aspen build artifacts, and synthetic mutation cases that model common rebuild deltas.

**Extra abstraction overhead** → Keep the trait pure and thin; benchmark abstraction overhead against direct FastCDC.
