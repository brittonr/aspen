## Why

Molten now has the ingredients for sound local caching: canonical Preserves boundaries, an artifact registry with dependency closures, schema identity fingerprints, typed-storage migration receipts, and upgrade transcript gates. Recomputing deterministic validation, schema compatibility, dependency closure, Wasm inspection, and transcript checks on every run will be slow. Caching by paths, mtimes, or mutable names would be unsound.

A local evaluation cache gives Molten a content-addressed, evidence-bearing cache for deterministic work. Cache keys must include canonical inputs, dependency closure hashes, handler profiles, policy refs, and tool versions so stale or name-based results cannot influence trust-boundary decisions.

## What Changes

- Add local canonical cache key/value DTOs for deterministic Molten evaluations.
- Add deterministic tiers: `pure`, `simulated`, `policy-current`, and `production-effectful-trace-only`.
- Add a Redb-backed local cache index keyed by canonical cache-key refs with reverse indexes by dependency, policy, evidence, operation kind, and result status.
- Emit receipts for cache hit, miss, insert, stale/policy-current denial, invalidation, and trace-only production observations.
- Add CLI inspection and mutation commands for local test workflows: put/get/status/invalidate/list/show.
- Integrate first cached operations around schema identity/fingerprint/compatibility and artifact dependency closure, with hooks for Wasm inspection and deterministic transcript gates.
- Preserve fail-closed behavior: cache hits never bypass current policy/capability/revocation inputs unless those inputs are represented in the key and revalidated.

## Impact

This makes repeated deterministic validation and upgrade gates faster without weakening evidence semantics. It also establishes a shared cache substrate for future schema validation, Wasm inspection, Trellis projection, executable transcript checks, and incident replay workflows.
