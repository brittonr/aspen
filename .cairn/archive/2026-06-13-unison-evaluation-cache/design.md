## Context

The artifact registry provides immutable ids and dependency closures. That enables sound caching of deterministic work. Many Molten operations are pure or can be made reproducible under local handlers: schema validation, artifact hashing, Trellis projection, Wasm static inspection, policy normalization, and transcript runs against fresh deterministic state.

Unison's test cache is prior art: if a deterministic test and its dependencies have the same hashes, rerunning it is unnecessary. Molten should generalize this for runtime validation and developer tooling while remaining conservative around effects.

## Goals

- Cache deterministic evaluation results by canonical inputs and dependency closure hashes.
- Make cache keys explicit, inspectable, and domain-separated per operation kind.
- Prevent nondeterministic production effects from poisoning deterministic caches.
- Share cache infrastructure across validation, projection, testing, transcript, and profiling workflows.
- Store cache records as canonical evidence-bearing values suitable for Redb indexing and optional Iroh sharing.

## Non-Goals

- Do not cache side-effectful production adapter results as if they were pure.
- Do not use wall-clock time, file mtime, local paths, or mutable names as sufficient cache identity.
- Do not let cached admission decisions bypass revocation, expiry, or current policy checks when those are inputs to the decision.
- Do not require distributed cache coherence in the first milestone.
- Do not claim that a cache hit is a proof unless the cached result itself carries validated evidence.

## Cache key

A cache key should include:

- operation kind and version,
- canonical input hash,
- direct artifact id if applicable,
- dependency closure hash,
- schema/policy refs used for validation,
- handler profile id for transcript/test/effectful simulation results,
- tool/adapter version or verifier artifact id,
- environmental assumptions for deterministic local handlers,
- domain separator and hash algorithm.

Mutable names may be stored for display but must not define the key.

## Cache value

A cache value should include:

- result status and canonical output hash,
- output bytes or content ref if large,
- trace refs,
- receipts/evidence refs,
- deterministic flag,
- creation time only as metadata,
- dependency refs and policy refs repeated for audit,
- negative-result information for safe repeated diagnostics.

Negative results may be cached if they are deterministic and all denial inputs are represented in the key.

## Determinism tiers

- `pure`: no effects, safe to cache by inputs and dependencies.
- `simulated`: effects interpreted by deterministic local/mock/chaos handler profile, cacheable with handler profile and seed/config in key.
- `policy_current`: cacheable only while referenced policy/capability/revocation artifacts remain unchanged and unexpired.
- `production_effectful`: not cached as a semantic result; may record traces and performance metadata only.

## Integration points

Choreography projection can cache projectability and per-role endpoint artifacts.

Wasm inspection can cache imported hostcalls, component interface summaries, forbidden feature checks, and required effect manifests.

Typed storage can cache schema validation for immutable value hashes, not authorization checks with changing capabilities unless capability state is in the key.

Executable transcripts can cache successful deterministic runs and compare cached trace/receipt output during docs builds.

## Policy and evidence

When a cache hit influences a trust-boundary decision, the runtime should validate that the cache record's evidence is still admissible under current policy and that all policy/capability inputs are represented in the key. Cache-hit receipts should reference the original result receipt rather than minting unexplained authority.

## Open Questions

- Which operation should land first: schema validation, Trellis projection, or Wasm inspection?
- Should cache records be shareable over Iroh blobs, or local-only until provenance rules are stricter?
- How should revocation and expiry be represented in keys for policy-current decisions?
- Should profiling summaries be cached separately from semantic results?
