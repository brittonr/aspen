## Why

Core proof claims should be exercised by generated laws, not only example fixtures. The repo already uses Hegel RS; a dedicated proof-property package makes the essential laws explicit for traceability, canonical refs, replay, deny monotonicity, and diagnostic/non-pass boundaries.

## What Changes

- Add a catalog of Hegel RS proof laws for core evidence behavior.
- Require generated property coverage for canonical ref stability, traceability decisions, stale-evidence monotonicity, replay ref comparisons, and diagnostic evidence boundaries.
- Persist shrunk counterexample fixtures as canonical evidence when they cross proof or release boundaries.
- Add traceability coverage for the Hegel property suites.

## Impact

- **Files**: runtime-pattern specs, Hegel property tests, proof fixtures, docs.
- **Testing**: Hegel RS suites with both generated positive and generated negative cases.
