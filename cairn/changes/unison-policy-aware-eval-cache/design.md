## Context

The evaluation cache is only sound when every semantically relevant input is part of the key or is revalidated before reuse. Molten's safety model has many such inputs: policy, capability, revocation, resource, handler profile, provenance, source gates, schemas, dependencies, and effect manifests.

## Design

### Cache key model

A normative cache key binds:

- computation kind;
- root artifact/input refs;
- dependency closure ref;
- schema refs and compatibility refs;
- effect manifest and handler profile refs;
- policy refs and exported policy digest;
- capability context and revocation epoch refs;
- resource/budget profile refs;
- provenance/source-gate/evidence refs;
- deterministic seed/logical time when relevant.

Diagnostic caches may omit some fields only when marked non-normative and unable to satisfy gates.

### Admission freshness

Before a cache hit satisfies pass evidence, Molten rechecks freshness for policy, capability, revocation, resource, handler profile, provenance/source-gate, and retention context. If any bound input is stale or incompatible, the hit denies or recomputes.

### Compatibility substitutions

Some substitutions may be safe, such as a policy export ref replaced by an explicitly equivalent policy receipt or a handler profile replaced by a compatibility receipt. Such substitutions must be recorded and cannot be implicit.

### Functional core and shell

Pure cores build cache keys, compare freshness summaries, evaluate compatibility receipts, and decide hit/miss/deny. Shells read caches, run computations, persist receipts, and render diagnostics.

### Non-goals

- Do not reuse cache entries across policy/capability/revocation/resource/profile changes without explicit compatibility evidence.
- Do not let cache hits bypass runtime gates.
- Do not treat diagnostic cache entries as pass evidence.
- Do not adopt Unison's cache format, codebase model, or typechecker.