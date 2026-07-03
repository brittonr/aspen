//! Tracey markers for chunk-store and evaluation-cache GC proof coverage.
//!
//! Implementation and tests live in `src/chunk/**` and `src/eval/parts/cache/**`;
//! Aspen's current Cairn Tracey scanner records references from `tools/**`, so
//! this file anchors the accepted proof IDs to focused chunk/cache tests.
//!
//! Verification evidence:
//! - `src/chunk/parts/store/tests/m000/p000/body.rs`
//! - `src/chunk/parts/store/tests/m000/p001/body.rs`
//! - `src/eval/parts/cache/tests/m000/p000/body.rs`

// r[verify molten.chunk_cache_state_proof.chunk_availability]
// r[verify molten.chunk_cache_state_proof.retention_gc]
// r[verify molten.chunk_cache_state_proof.cache_stale_reuse]
// r[verify molten.chunk_cache_state_proof.cache_invalidation_gc]
