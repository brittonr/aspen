## Why

The no-disabled Octet probe includes a smaller set of warning families such as `borrowed_argument_types`, `platform_dependent_cast`, `unbounded_collection_growth`, `bool_naming`, `unchecked_division`, `ignored_result`, and `nested_conditionals`. These are lower-count than the broad shape lints but closer to correctness and Tiger Style review concerns.

This change isolates safety-polish warnings so they can be addressed deliberately with positive and negative tests instead of being lost inside broad mechanical refactors.

## What Changes

- Track lower-count safety and clarity warning families as a dedicated active Cairn change.
- Replace questionable casts, unchecked arithmetic, ignored results, ambiguous bool names, nested conditionals, and unbounded collections with explicit, testable logic.
- Add positive and negative tests for any changed core behavior.
- Refresh focused validation and no-disabled Octet evidence after each accepted slice.

## Impact

This is targeted correctness and clarity work. It may touch core logic, so each slice should be small, tested, and validated before being counted as burn-down progress.
