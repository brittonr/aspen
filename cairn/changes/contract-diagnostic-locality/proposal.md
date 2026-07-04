# Change: contract-diagnostic-locality

## Why

Large `std.contract.from_predicate` checks can reject invalid data correctly while still giving opaque failure locations. As contracts become stricter, poor diagnostics make fixture failures harder to triage and increase the chance that contributors weaken a predicate to get unstuck.

## What

- Refactor contract modules toward field-level contracts plus small named cross-field predicates.
- Name failure classes in fixtures and validation output where practical.
- Preserve existing fail-closed behavior; diagnostic improvements must not make invalid exports pass.

## Impact

Contract failures become easier to localize without weakening validation. Contributors can identify the bad field or invariant and update fixtures, source, or documentation intentionally.
