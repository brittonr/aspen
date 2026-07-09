## Why

Unison's test cache shows the value of reusing deterministic work by dependency hash. Molten already has an evaluation cache; the next important adaptation is to ensure cache identity includes every policy and admission input that can change whether a result is safe to reuse.

A cached validation, projection, transcript run, or job stage result must not bypass current policy, capability, revocation, resource, handler profile, provenance, source-gate, or retention context.

## What Changes

- Extend cache keys for normative results to include artifact refs, dependency closures, input refs, schema refs, policy refs, capability context refs, revocation epoch refs, resource refs, handler profile refs, and evidence refs.
- Require admission freshness checks before cache hits can satisfy pass evidence.
- Add compatibility receipts for safe profile/policy substitutions.
- Add negative cache-hit denial fixtures for stale policy, revoked capability, changed handler profile, changed dependency closure, and diagnostic-only results.

## Impact

- **Files**: evaluation cache, transcripts, job DAG, schema/protocol projection, policy gates, effect handlers.
- **Testing**: positive fixtures for deterministic cache hits; negative fixtures for stale or incompatible admission context.
- **Security**: cache hits save work but never grant authority or bypass current gates.