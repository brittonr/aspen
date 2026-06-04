## Context

Several Molten subsystems now produce immutable, canonical inputs suitable for caching:

- `artifacts` installs immutable artifact envelopes and computes dependency closures.
- `schema_identity` computes structural fingerprints and compatibility decisions.
- `typed_storage` validates schema-tagged values and migration recipes.
- `harness` and upgrade sessions can run deterministic transcript-like checks.
- Wasm and Steel executor preflights already produce canonical inspection/review receipts.

The next local slice is an evaluation cache that stores deterministic results by content identity and evidence refs, not by file names or mutable metadata.

This is an implementation-oriented slice of the broader `unison-evaluation-cache` idea. Unison's test cache is useful prior art, but Molten does not adopt Unison's hash format, typechecker, runtime, CLI, or codebase model.

## Goals

- Define canonical cache keys and values suitable for Redb storage and evidence receipts.
- Include all determinism inputs in cache keys: operation kind, canonical input ref, dependency closure hash, handler profile, policy/capability/revocation refs where relevant, and tool/version refs.
- Classify cache entries by determinism tier.
- Store deterministic positive and negative results only when denial inputs are represented in the key.
- Revalidate `policy-current` entries against current policy/capability/revocation refs before admitting a hit.
- Emit receipts for hit/miss/insert/invalidate/stale-denial events.
- Provide CLI inspection for local development and upgrade-session diagnostics.
- Integrate first with schema identity and artifact dependency closure, then leave hooks for Wasm inspection and deterministic transcript runs.

## Non-Goals

- Do not cache production side effects as semantic results.
- Do not use file paths, mtimes, mutable names, current working directory, process environment, or wall-clock time as sufficient cache keys.
- Do not let cache hits mint authority or bypass policy/capability admission.
- Do not require distributed cache coherence in this slice.
- Do not claim cache entries are proofs unless the cached result carries validated evidence refs.
- Do not share cache entries over Iroh until provenance and confidentiality policy are explicit.

## Cache key model

Introduce a canonical cache key record:

```preserves
<eval-cache-key-v1 "molten.eval-cache.key.v1"
  <operation "schema-fingerprint" | "schema-compat" | "artifact-closure" | "wasm-inspection" | "transcript-run" | ...>
  <version "v1">
  <input <input-ref>>
  <dependencies <closure-hash> [<dependency-ref> ...]>
  <handler-profile <none> | <some <handler-profile-ref>>>
  <policy [<policy-ref> ...]>
  <capability [<capability-ref> ...]>
  <revocation [<revocation-ref> ...]>
  <tool <tool-ref> <tool-version>>
  <assumptions [<assumption-ref> ...]>
  <checks [<check "no-name-key" "pass"> ...]>>
```

The cache key ref is the canonical hash of this record. The `input-ref` must be a canonical value/artifact/schema/report ref, never a path or mutable name. `closure-hash` can come from the local artifact registry closure receipt or a local deterministic closure over supplied refs.

## Cache value model

Cache values should be canonical records:

```preserves
<eval-cache-value-v1 "molten.eval-cache.value.v1"
  <key <cache-key-ref>>
  <tier "pure" | "simulated" | "policy-current" | "production-effectful-trace-only">
  <status "pass" | "deny" | "error" | "trace-only">
  <output <inline <output-ref>> | <content-ref <manifest-ref>> | <none>>
  <dependencies [<dependency-ref> ...]>
  <policy [<policy-ref> ...]>
  <evidence [<receipt-ref> ...]>
  <diagnostics ["..."]>
  <checks [<check "determinism-inputs-bound" "pass"> ...]>>
```

For large outputs, store canonical bytes through chunk/content refs. Negative results may be cached only when all denial inputs are represented by key fields and the tier is not `production-effectful-trace-only`.

## Determinism tiers

- `pure`: no effects and no current-policy dependence. Cache hit is valid by matching canonical key/value refs.
- `simulated`: deterministic local/mock/chaos handler profile with seed/config included in the key.
- `policy-current`: result depends on current policy/capability/revocation artifacts. Hit requires revalidation that current refs match key refs and no expiry/revocation evidence changed.
- `production-effectful-trace-only`: may store trace/performance metadata but must not be returned as semantic output.

Validators must reject unknown tiers and must reject semantic gets for trace-only entries.

## Redb index

The local cache index should include:

- cache key ref -> key bytes,
- cache key ref -> value bytes,
- operation kind -> cache key refs,
- dependency ref -> cache key refs,
- policy/capability/revocation ref -> cache key refs,
- evidence ref -> cache key refs,
- status/tier -> cache key refs,
- receipt ref -> receipt bytes.

Indexes are derived from canonical key/value records and should be rebuildable. Rebuild must preserve historical receipts while recomputing derived indexes.

## Receipts

Cache receipts should be canonical:

```preserves
<eval-cache-receipt-v1 "molten.eval-cache.receipt.v1"
  <operation "put" | "get" | "hit" | "miss" | "invalidate" | "stale-deny" | "trace-only">
  <decision "pass" | "deny">
  <key <cache-key-ref-or-none>>
  <value <cache-value-ref-or-none>>
  <refs [<input-ref> <dependency-ref> <policy-ref> ...]>
  <diagnostics ["..."]>
  <checks [<check "no-name-key" "pass"> ...]>>
```

When a cache hit influences an admission or upgrade decision, the receipt must reference the original result evidence and the revalidation evidence for policy-current entries.

## First cached operations

The first implementation should support generic put/get plus helpers for:

- `schema-fingerprint`: key over normalized shape ref, tool version, and schema identity mode.
- `schema-compat`: key over expected identity ref, actual identity ref, alias/migration refs, policy refs, and schema-identity tool version.
- `artifact-closure`: key over root artifact refs, closure hash, artifact registry version, and policy refs.

Hooks should be present for later:

- `wasm-inspection`, keyed by module artifact ref, WIT refs, inspector version, and allowed hostcall schema refs.
- `transcript-run`, keyed by transcript artifact, dependency closure, handler profile, seed/config, schema/policy refs, and harness version.

## Policy-current revalidation

For `policy-current` entries, a get operation must compare current policy/capability/revocation refs with the key and any caller-supplied current refs. If current refs differ or are incomplete, the get returns a stale-deny receipt rather than a hit. This is a local conservative rule until richer authority/revocation expiry indexes are available.

## CLI

Add `molten test cache` commands:

- `put` with key/value files and receipt output,
- `get` by cache key ref with optional current policy/capability/revocation refs,
- `status` summary counts by operation/status/tier,
- `invalidate` by dependency/policy/ref or explicit key,
- `list` optionally filtered by operation/tier/status,
- `show` key/value/receipt by ref.

All CLI commands should print full refs and never resolve names as cache identity.

## Integration with upgrades

Upgrade transcript gates can later require either a fresh deterministic run or a valid cache hit whose key binds the transcript artifact, dependency closure, handler profile, seed/config, and current policy refs. This slice should expose enough API for upgrade sessions to query a cache receipt without treating it as proof by itself.

## Tests and properties

Required tests:

- unchanged canonical key returns hit and matching output ref,
- changed input or dependency closure produces a miss,
- mutable display names do not affect keys,
- policy-current hit denies when current policy refs differ,
- negative deterministic denials can be cached only with denial refs in the key,
- trace-only production entries cannot be returned as semantic values,
- invalidation by dependency/policy removes or tombstones matching keys,
- Hegel properties for key determinism, dependency invalidation monotonicity, and no-name-key invariants.

## Open Questions

- Should cache outputs be promoted into the artifact registry or remain cache-local values with optional chunk refs?
- How should expiry times be modeled without introducing wall-clock nondeterminism?
- Which Wasm inspection details are stable enough to cache across wasmtime/wasmparser versions?
- When remote sharing arrives, which cache tiers are safe to publish under provenance/confidentiality policy?
