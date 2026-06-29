## Why

`non_trait_imports` remains the largest no-disabled Octet warning family. Prior slices showed that module-local owner namespaces can reduce import noise while preserving behavior.

This change continues import-hygiene burn-down without changing public APIs, command syntax, or receipt evidence.

## What Changes

- Refresh the no-disabled probe and rank import-hygiene hotspots by domain.
- Replace broad concrete imports with module-local owner namespaces or focused aliases where that improves clarity.
- Keep behavior and canonical output unchanged.
- Record before/after import warning movement for each accepted slice.

## Impact

This is mechanical readability and source-gate cleanup. It should lower import-hygiene debt while making future diffs easier to review.
