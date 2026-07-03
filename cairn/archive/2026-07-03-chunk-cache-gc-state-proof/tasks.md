# Tasks: chunk-cache-gc-state-proof

## Phase 1: Chunk store proof

- [x] [serial] r[molten.chunk_cache_state_proof.chunk_availability] Define pure chunk availability checks for manifests, chunks, indexes, partial fetches, and missing scans.
- [x] [parallel] r[molten.chunk_cache_state_proof.retention_gc] Bind chunk/manifest GC decisions to exact retention apply and execution gate refs.

## Phase 2: Evaluation cache proof

- [x] [serial] r[molten.chunk_cache_state_proof.cache_stale_reuse] Define pure cache-hit validity checks over keys, dependencies, outputs, policy, capability, revocation, and determinism tier.
- [x] [parallel] r[molten.chunk_cache_state_proof.cache_invalidation_gc] Bind cache invalidation and cleanup to retention admission and audit-preserving tombstone evidence.

## Phase 3: Tests and validation

- [x] [parallel] r[molten.chunk_cache_state_proof.chunk_availability] Add positive put/index/read and partial-fetch-repair tests plus corrupt/missing denial tests.
- [x] [parallel] r[molten.chunk_cache_state_proof.retention_gc] r[molten.chunk_cache_state_proof.cache_invalidation_gc] Add missing retention apply, incomplete reachability, and cleanup denial tests.
- [x] [parallel] r[molten.chunk_cache_state_proof.cache_stale_reuse] Add stale policy-current, revoked capability, changed dependency, trace-only, and name-only alias denial tests.
- [x] [serial] r[molten.chunk_cache_state_proof.chunk_availability] r[molten.chunk_cache_state_proof.retention_gc] r[molten.chunk_cache_state_proof.cache_stale_reuse] r[molten.chunk_cache_state_proof.cache_invalidation_gc] Bind proof refs and run `cargo test chunk cache retention`.
