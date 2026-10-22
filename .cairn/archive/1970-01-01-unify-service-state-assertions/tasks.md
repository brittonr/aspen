# Tasks: Unify service state assertions

## Inventory

- [x] [serial] Record the three current vocabularies with every local state value, and write one mapping table to the shared set. r[molten.runtime_spine.service_state_union]

## Projection

- [x] [serial] Add the pure projection from committed lifecycle, extension, and readiness state to the assertion set, including `started`, `ready`, `complete`, and `failed`. r[molten.runtime_spine.service_state_union]
- [x] [serial] Derive the `up` alias over `ready` or `complete`. r[molten.runtime_spine.service_state_union]
- [x] [serial] Make the service dependency predicate read the projected set, and reject a user-defined state as a match for a built-in dependency state. r[molten.runtime_spine.service_state_union]

## Validation

- [x] [parallel] Add positive tests: a one-shot service reaches `complete` on a normal exit; a started service holds `started` and `ready` together; a dependent waiting on `up` proceeds after `ready` and after `complete`. r[molten.runtime_spine.service_state_union]
- [x] [parallel] Add negative tests: a user-defined state does not satisfy a `ready` dependency; a process that exits before readiness reports `failed`, not `ready`; the projection never changes FSM state. r[molten.runtime_spine.service_state_union]
- [x] [parallel] Add the mapping-table test with one row per local vocabulary value in both directions. r[molten.runtime_spine.service_state_union]
- [x] [serial] Run focused lifecycle, extension, and predicate tests plus Clippy before and after the change, then the workspace checks the repository requires. r[molten.runtime_spine.service_state_union]
- [x] [serial] Update the lifecycle and architecture documentation with the union model and its non-claims. r[molten.runtime_spine.service_state_union]
