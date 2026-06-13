# Evaluation Cache Specification

## Purpose

Defines the `evaluation-cache` capability.

## Requirements

### Requirement: Cache keys are domain separated
r[molten.eval_cache.key_model] Molten MUST define domain-separated cache keys that bind operation kind, canonical input ref, dependency closure hash, dependency refs, optional handler profile ref, policy refs, capability refs, revocation refs, tool ref, and tool version.

#### Scenario: Mutable names are excluded from identity
- GIVEN two requests with the same canonical inputs but different display names or local paths
- WHEN Molten computes their evaluation cache keys
- THEN mutable names, mtimes, and local paths do not substitute for content refs or dependency closure hashes.

### Requirement: Cache values bind deterministic outputs
r[molten.eval_cache.value_model] Molten MUST define cache values with status, output hash or content ref, deterministic tier, dependency refs, policy refs, evidence refs, and diagnostics.

#### Scenario: Output integrity is verified
- GIVEN a cached value with an output ref
- WHEN Molten reads the cache entry
- THEN the output bytes or content ref are verified against the cached value before the result is used.

### Requirement: Determinism tiers are explicit
r[molten.eval_cache.determinism_tiers] Molten MUST classify cacheable results as pure, simulated, policy-current, or production-effectful trace-only.

#### Scenario: Trace-only entries are not semantic hits
- GIVEN a production-effectful trace-only cache entry
- WHEN a semantic cache lookup requests a reusable result
- THEN Molten denies the hit and emits diagnostics instead of replaying it as deterministic output.

### Requirement: Names are not sufficient keys
r[molten.eval_cache.no_name_keys] Molten MUST document and enforce that mutable names, file mtimes, and local paths are not sufficient cache keys.

#### Scenario: Name-only change cannot alias cache result
- GIVEN a cache key derived from display-name metadata alone
- WHEN compared with a content-ref-derived key
- THEN the refs differ and the name-only key cannot authorize reuse.

### Requirement: Cache index is persisted in Redb
r[molten.eval_cache.redb_store] Molten MUST add a Redb-backed local cache index for cache keys, values, dependencies, policy refs, capability refs, revocation refs, evidence refs, receipts, and tombstones.

#### Scenario: Indexed dependency invalidation finds entries
- GIVEN cache entries indexed by dependency ref
- WHEN that dependency is invalidated
- THEN matching key refs are found and tombstoned only through retention-gated mutation.

### Requirement: Cache decisions emit receipts
r[molten.eval_cache.receipts] Molten MUST emit trace/evidence records for cache puts, hits, misses, stale entries, tombstones, and cache-influenced decisions.

#### Scenario: Cache miss has receipt evidence
- GIVEN a lookup for an unknown key
- WHEN Molten denies the cache hit
- THEN it emits a receipt with miss diagnostics.

### Requirement: Policy-current entries are revalidated
r[molten.eval_cache.policy_current] Molten MUST revalidate policy-current cache entries against current policy, capability, and revocation inputs before use.

#### Scenario: Policy-current stale entry denies
- GIVEN a policy-current cache entry bound to old policy refs
- WHEN the current policy refs differ
- THEN Molten denies reuse as stale.

### Requirement: Negative results bind denial inputs
r[molten.eval_cache.negative_results] Molten MUST cache deterministic negative results only when denial inputs are represented in the key or policy inputs.

#### Scenario: Unbound denial evidence is rejected
- GIVEN a denial result with evidence refs absent from the key assumptions or policy refs
- WHEN Molten stores it in the cache
- THEN the cache write is rejected.

### Requirement: Schema validation is cacheable
r[molten.eval_cache.schema_validation] Molten MUST cache schema validation results for immutable value and schema hashes.

#### Scenario: Schema fingerprint key is stable
- GIVEN the same normalized schema shape and tool version
- WHEN Molten computes schema validation cache keys
- THEN the key ref is stable.

### Requirement: Choreography projection is cacheable
r[molten.eval_cache.choreography_projection] Molten MUST cache Trellis projectability and endpoint projection results by protocol artifact ref, role ref, dependency closure hash, projector ref, and policy refs.

#### Scenario: Projection key binds role and protocol
- GIVEN a protocol artifact and endpoint role
- WHEN Molten computes a projection cache key
- THEN the key binds both refs and the dependency closure.

### Requirement: Wasm inspection is cacheable
r[molten.eval_cache.wasm_inspection] Molten MUST cache Wasm/component inspection summaries by module artifact ref and inspector version.

#### Scenario: Inspector version changes key
- GIVEN a module artifact and two inspector versions
- WHEN Molten computes inspection cache keys
- THEN the keys differ across inspector versions.

### Requirement: Transcript results are cacheable
r[molten.eval_cache.transcript_results] Molten MUST cache deterministic executable transcript results by transcript artifact, dependency closure, handler profile, and seed or config refs.

#### Scenario: Handler profile participates in transcript key
- GIVEN a transcript run with a handler profile
- WHEN Molten computes the transcript cache key
- THEN the handler profile ref is part of the key identity.

### Requirement: Hit and miss tests cover stable inputs
r[molten.eval_cache.hit_miss_tests] Molten MUST test cache hit and miss behavior under unchanged and changed canonical inputs.

#### Scenario: Changed input misses
- GIVEN a cached result for one input ref
- WHEN lookup uses a different input ref
- THEN the result is a miss.

### Requirement: Policy revalidation tests cover stale inputs
r[molten.eval_cache.policy_revalidation_tests] Molten MUST test that policy-current cache entries are rejected after relevant policy, capability, or revocation changes.

#### Scenario: Revocation changes deny reuse
- GIVEN a policy-current cache entry bound to a revocation set
- WHEN the current revocation refs differ
- THEN lookup fails stale.

### Requirement: Simulated handler tests cover handler keys
r[molten.eval_cache.simulated_handler_tests] Molten MUST test deterministic local, mock, or chaos handler cache keys.

#### Scenario: Simulated handler profile is bound
- GIVEN a simulated handler profile ref
- WHEN Molten caches a simulated result
- THEN the handler profile ref is part of the key.

### Requirement: Property tests cover cache invariants
r[molten.eval_cache.property_tests] Molten MUST add Hegel property tests for key determinism, dependency invalidation, and no-name-key invariants.

#### Scenario: Generated dependency invalidation tombstones matching keys
- GIVEN generated dependency refs and cache entries
- WHEN invalidation runs for a generated dependency
- THEN matching keys are tombstoned and unrelated keys are not treated as semantic hits.
