## ADDED Requirements

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
