# Design: chunk and cache GC state proof

## Scope

This change proves chunk-store availability/GC and evaluation-cache invalidation state machines. It covers manifest creation, chunk put/fetch, partial fetch, availability scan, pinning, retention-gated GC, cache key/value state, policy-current revalidation, dependency invalidation, tombstones, and stale hit denial.

## Proof checklist

- **Proof claim**: chunk reads use only available and hash-verified content; destructive chunk/cache mutation requires retention apply/execution gates; cache hits are reusable only when key, dependency, policy, capability, revocation, and output refs remain valid.
- **Out of scope**: remote peer availability and filesystem durability beyond stored receipt evidence.
- **Trusted assumptions**: BLAKE3 content refs and Redb committed writes are stable for stored values.
- **Positive evidence**: put→index→read, partial fetch repair, pin preservation, retention-gated GC, cache hit, cache miss, and dependency invalidation traces.
- **Negative evidence**: corrupt chunk, missing manifest, incomplete reachability proof, missing apply ref, stale policy-current cache entry, revoked capability, and name-only alias deny.
- **Canonical refs**: manifest refs, chunk refs, index/status refs, pin refs, partial fetch refs, GC receipt refs, cache key refs, cache value refs, tombstone refs, and retention apply/execution refs.
- **Regeneration command**: `cargo test chunk cache retention`.

## Functional core

Keep availability, GC eligibility, and cache-hit decisions pure over manifests, chunk status, pins, retention gates, cache keys, dependencies, and policy/revocation refs. IO shells only serve or delete after pass decisions.

## Non-goals

- No remote availability guarantee.
- No cache reuse for production-effectful trace-only entries.
