## Why

The no-disabled Octet probe shows `non_trait_imports` as the largest remaining warning family. Keeping it inside the broad `octet-source-remediated-zero` package makes the work hard to schedule and blocks archive of already-completed source-shape slices.

This change isolates import-hygiene burn-down so each refactor can reduce import debt without coupling it to path-shape, file-size, source-scope, or smaller safety lint work.

## What Changes

- Track `non_trait_imports` as a dedicated active Cairn change.
- Refactor import-heavy modules in small behavior-preserving slices using local qualification, narrower imports, or module-local helper/type boundaries.
- Preserve public Rust paths, CLI syntax, canonical receipt schemas, denial behavior, and existing evidence contracts.
- Keep the disabled-lint caveat visible until refreshed no-disabled evidence proves this family is clean or safely narrowed.

## Impact

This is source-shape work only. It should reduce broad import warnings without changing runtime semantics, release evidence contracts, or subsystem authority boundaries.
