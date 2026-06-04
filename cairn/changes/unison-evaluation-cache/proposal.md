## Why

Molten will repeatedly run pure validation, schema checks, choreography projection, Wasm inspection, policy normalization, transcript execution, and property tests. If these results are recomputed on every run, the runtime and developer workflow will be slow. If they are cached by filenames or mutable names, the cache will be unsound.

Unison caches parsed/typechecked definitions and pure test results by dependency hash. Molten should adopt the same principle: deterministic computations can be cached by artifact id, dependency closure, inputs, and handler profile.

## What Changes

- Add a content-addressed evaluation cache for deterministic Molten computations.
- Cache results by operation kind, canonical input hash, dependency closure hash, tool/adapter version, and handler profile where relevant.
- Mark cache entries as deterministic-only unless all effects are handled by reproducible local/mock handlers.
- Cache schema validation, artifact canonicalization, dependency closure computation, Trellis projectability/projection, Wasm inspection, policy normalization, executable transcript results, and pure property-test results.
- Record cache hits and misses as trace/evidence data when they influence admission or testing.
- Invalidate by construction when any input, dependency, policy artifact, handler profile, or tool version changes.

## Impact

Molten can make expensive validation and test workflows fast without weakening correctness. The first milestone can cache pure schema validation and choreography projection results in Redb by canonical input and dependency hashes.
