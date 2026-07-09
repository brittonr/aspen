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

### Requirement: Evaluation cache state denies stale reuse
r[molten.chunk_cache_state_proof.cache_stale_reuse] Molten MUST prove that evaluation-cache hits are accepted only when key refs, dependency refs, output refs, policy refs, capability refs, revocation refs, and determinism tier remain valid for the requested use.

#### Scenario: Policy-current stale hit denies
- GIVEN a policy-current cache entry bound to old policy refs
- WHEN lookup is performed with different current policy refs
- THEN cache hit decision is `deny`
- AND no cached output is returned as semantic result.

### Requirement: Evaluation cache invalidation is retention bounded
r[molten.chunk_cache_state_proof.cache_invalidation_gc] Molten MUST prove that cache invalidation, tombstoning, and cleanup bind retention admission and preserve audit refs before removing cache content or dependency indexes.

#### Scenario: Invalidation without retention denies cleanup
- GIVEN a cache entry selected for destructive cleanup
- WHEN retention admission or apply evidence is missing
- THEN cleanup decision is `deny`
- AND the cache record remains discoverable for audit.

### Requirement: Normative cache keys bind admission context
r[molten.eval_cache.policy_aware_keys] Molten MUST include artifact refs, dependency closure refs, input refs, schema refs, policy refs or policy export refs, capability context refs, revocation epoch refs, resource refs, effect manifest refs, handler profile refs, provenance refs, source-gate refs, and evidence refs in cache keys for results that may satisfy normative pass evidence.

#### Scenario: Same context produces same cache key
- GIVEN the same computation kind, artifact refs, dependency closure, inputs, schemas, policy export, capability context, revocation epoch, resource refs, effect manifest, handler profile, provenance, source-gate, and evidence refs
- WHEN Molten builds the normative cache key twice
- THEN both keys are identical.

#### Scenario: Changed policy export changes key
- GIVEN a cached result was produced under policy export P1
- WHEN the same computation is requested under policy export P2 without compatibility evidence
- THEN Molten uses a different key or denies reuse of the old hit.

### Requirement: Cache hits require admission freshness
r[molten.eval_cache.admission_freshness] Molten MUST recheck policy, capability, revocation, resource, handler profile, provenance, source-gate, retention, and evidence freshness before a cache hit can satisfy pass evidence.

#### Scenario: Fresh hit satisfies validation
- GIVEN a cache entry matches the current key and all admission freshness checks pass
- WHEN Molten evaluates the hit
- THEN it may satisfy the deterministic computation result
- AND the hit receipt binds freshness checks.

#### Scenario: Revoked capability denies hit
- GIVEN a cache entry was produced before a capability was revoked
- WHEN Molten evaluates the hit after the revocation epoch changed
- THEN the hit denies or recomputes before pass evidence is accepted.

### Requirement: Compatibility substitutions are explicit
r[molten.eval_cache.profile_compatibility] Molten MUST require explicit compatibility receipts for safe policy, schema, handler profile, evidence, or provenance substitutions in cache-hit decisions.

#### Scenario: Compatible handler profile permits reuse
- GIVEN a cache entry was produced under handler profile H1
- WHEN an admitted compatibility receipt proves H2 is equivalent for this computation
- THEN Molten may reuse the entry
- AND the hit receipt binds the compatibility receipt.

#### Scenario: Implicit profile substitution denies
- GIVEN a caller requests reuse under handler profile H2 without compatibility evidence
- WHEN the cache entry was produced under H1
- THEN Molten denies the normative cache hit.

### Requirement: Negative cache hits fail closed
r[molten.eval_cache.negative_hit_denial] Molten MUST deny normative cache hits for stale policy, revoked capability, changed handler profile, changed dependency closure, missing evidence, unsupported substitution, or diagnostic-only cache entries.

#### Scenario: Diagnostic entry cannot satisfy gate
- GIVEN a cached result is marked diagnostic-only
- WHEN a release or policy gate asks for pass evidence
- THEN Molten denies reuse of that entry as normative evidence.

#### Scenario: Changed dependency closure denies hit
- GIVEN the root artifact ref is the same but its admitted dependency closure ref changed
- WHEN Molten evaluates a prior cache entry
- THEN it denies or recomputes instead of treating the hit as equivalent.

### Requirement: Policy-aware cache validation covers positive and negative paths
r[molten.eval_cache.policy_aware_validation] Molten MUST include positive and negative fixtures for deterministic hits, stale policy, revoked capability, profile mismatch, changed closure, compatibility substitution, missing evidence, and diagnostic-only cache denial.

#### Scenario: Deterministic hit fixture passes
- GIVEN a cache entry and current request have identical normative key inputs and fresh admission context
- WHEN validation runs
- THEN Molten emits a passing cache-hit receipt.

#### Scenario: Stale policy fixture denies
- GIVEN a cache entry was produced under an old policy export ref
- WHEN validation runs under a changed policy without compatibility evidence
- THEN the cache hit denies
- AND diagnostics identify the stale policy input.
