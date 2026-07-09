# Evaluation Cache Delta: Policy-Aware Cache Identity

## ADDED Requirements

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