## Phase 1: Core decisions

- [ ] [serial] Implement the canonical key projection over dataspace identity, normalized access arguments, and optional capability context, backed by BLAKE3. r[aspen.dataspace_access_cache.projection]
- [ ] [serial] Implement the pure retention decision core from supplied values. r[aspen.dataspace_access_cache.decision] r[aspen.dataspace_access_cache.deferral]
- [ ] [serial] Require an explicit capacity with no implicit default bound. r[aspen.dataspace_access_cache.bound]

## Phase 2: Bounded store

- [ ] [depends:aspen.dataspace_access_cache.bound] Implement the bounded store using core decisions. r[aspen.dataspace_access_cache.bound]
- [ ] [depends:aspen.dataspace_access_cache.deferral] Execute deferred releases strictly after the guard releases. r[aspen.dataspace_access_cache.deferral]
- [ ] [serial] Route cache misses to the existing dataspace adapter unchanged. r[aspen.dataspace_access_cache.boundary]

## Phase 3: Fixtures and evidence

- [ ] [parallel] Add positive fixtures for projection stability, bounded retention, eviction order, promotion at and below threshold, and deferred-release ordering. r[aspen.dataspace_access_cache.verification]
- [ ] [parallel] Add negative and boundary fixtures for missing bounds, reversed watermarks, zero capacity, single slot, threshold 0 and 100. r[aspen.dataspace_access_cache.verification]
- [ ] [serial] Document the claim boundary and the recorded cross-repo retention contract. r[aspen.dataspace_access_cache.boundary]
- [ ] [serial] Run workspace, Clippy, formatting, Cairn, and Nix checks, then record evidence. r[aspen.dataspace_access_cache.verification]
