## Phase 1: Core decisions

- [x] [serial] Implement the canonical key projection over dataspace identity, normalized access arguments, and optional capability context, backed by BLAKE3. r[aspen.dataspace_access_cache.projection]
- [x] [serial] Implement the pure retention decision core from supplied values. r[aspen.dataspace_access_cache.decision] r[aspen.dataspace_access_cache.deferral]
- [x] [serial] Require an explicit capacity with no implicit default bound. r[aspen.dataspace_access_cache.bound]

## Phase 2: Bounded store

- [x] [depends:aspen.dataspace_access_cache.bound] Implement the bounded store using core decisions. r[aspen.dataspace_access_cache.bound]
- [x] [depends:aspen.dataspace_access_cache.deferral] Execute deferred releases strictly after the guard releases. r[aspen.dataspace_access_cache.deferral]
- [x] [serial] Route cache misses to the existing dataspace adapter unchanged. r[aspen.dataspace_access_cache.boundary]

## Phase 3: Fixtures and evidence

- [x] [parallel] Add positive fixtures for projection stability, bounded retention, eviction order, promotion at and below threshold, and deferred-release ordering. r[aspen.dataspace_access_cache.verification]
- [x] [parallel] Add negative and boundary fixtures for missing bounds, reversed watermarks, zero capacity, single slot, threshold 0 and 100. r[aspen.dataspace_access_cache.verification]
- [x] [serial] Document the claim boundary and the recorded cross-repo retention contract. r[aspen.dataspace_access_cache.boundary]
- [x] [serial] Run workspace, Clippy, formatting, Cairn, and Nix checks, then record evidence. r[aspen.dataspace_access_cache.verification]
