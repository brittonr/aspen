# Proposal: Publish withdrawable service demand

## Why

`evaluate_service_dependencies` already distinguishes demanded service refs from force-run refs and checks dependency
subsets (`src/runtime/predicates/parts/mod/p004/body.rs`). Demand arrives as an input list to a pure predicate, so it
has no lifetime: a caller cannot withdraw demand, and a shutdown request has no observable form.

The manual models demand as retractable state. `require-service` starts a service only after its dependencies are
satisfied and then asserts `run-service` for it. `run-service` starts immediately. A service shuts down when the last
`run-service` assertion for it is withdrawn, and `restart-service` is a message rather than an assertion
(`12-operation__service.md → Details`, `→ Request a service restart`).

Demand without a lifetime also hides the difference between "no provider exists" and "nobody asked". The accepted
requirement `molten.runtime_spine.demand_driven_startup` is listed as implementation-unestablished in the tracey
baseline.

## What Changes

- Represent service demand as a retractable dataspace assertion owned by the demanding scope, and permit shutdown when
  the last demand assertion for a service is withdrawn. r[molten.runtime_spine.demand_assertion_lifetime]
- Keep the two demand forms distinct: dependency-gated demand waits for declared dependencies, and force-run demand
  bypasses dependency order by explicit declaration only.
- Deduplicate demand per owner, so two demanders of one service count as two live demands and one owner cannot
  double-count.
- Keep a restart request a message. A restart MUST NOT create, extend, or satisfy demand.
- Keep the predicate pure: it receives the demand facts derived from the assertion set, and it never reads ambient
  state.

## Impact

- **Files**: `src/runtime/predicates/parts/mod/p004/body.rs`, `src/runtime/dataspace/state.rs`, `src/lifecycle/`,
  `docs/architecture.md` service section, and the referenced tests.
- **Testing**: positive cases for withdrawable demand and for dependency-gated start; negative cases for a
  double-counted owner, an unverified provider satisfying demand, a restart message creating state, and shutdown while
  another owner still demands the service.
- **Non-goals**: no capacity inference from concurrency, no placement decision, no exactly-once start claim, and no
  authority change.
