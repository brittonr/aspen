## Why

Issues #54 and #55 found two dispatch defects in world promotion. A failed release observation was stored exactly like a successful one (state `observed`, no outcome), so an operator acknowledgement could record a failed effect as proven external completion. A dispatcher error returned before any record was written, leaving the attempt and its reservation in `attempting`, where neither retry nor abandon applies.

## What Changes

- Attempt records and their receipts carry an explicit `unknown`, `succeeded` or `failed` outcome in a new eight-field attempt schema, `molten.world-release-attempt.v2`. Only a succeeded observation can be acknowledged or committed as proven external completion.
- On a dispatcher error the shell records the attempt and reservation as `uncertain` and publishes the uncertain receipt before returning the error. If recording fails too, the returned error carries both failures.
- `reconcile` classifies attempts that a stopped process left `attempting` as uncertain, the same state a dispatcher error now records.
- Seven-field v1 attempt records are rejected: they cannot say whether a release succeeded.

## Impact

- **Files**: `molten-core` world promotion dispatch/observation model and tests; root world promotion records, service and tests; the v2 schema artifact; the world promotion document.
- **Testing**: core classification, acknowledgement and observation-commit denials; wire round trips and byte layout for each outcome; rejection of an unsupported outcome and of the v1 layout; shell tests for a dispatcher error and for a failed uncertainty write.
- **Owner / consumer**: the world promotion release shell. Retry and abandon reconciliation consume the uncertain state. No production dispatcher exists yet, so no stored v1 attempt needs migration.
