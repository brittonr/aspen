## Why

Chunk manifests, chunk availability, partial fetches, GC, and evaluation-cache invalidation form storage state machines. They need proof that stale or missing content is not served, retention-gated deletion is required before removal, and cache hits cannot survive dependency, policy, capability, or revocation drift.

## What Changes

- Add requirements for chunk-store availability/GC proof and evaluation-cache invalidation proof.
- Require proof traces for put, fetch, partial fetch, pin, missing scan, GC, cache hit/miss, stale invalidation, and tombstone flows.
- Require negative evidence for corrupt chunks, missing manifests, incomplete reachability, missing retention apply refs, stale cache policy, and name-only cache aliases.

## Impact

- **Files**: chunk store, chunk index, cache status/invalidation, retention integration, and tests.
- **Testing**: available/missing transitions, partial fetch repair, retention-gated GC pass, corrupt/missing denial, stale cache denial, and no name-only cache hit.
