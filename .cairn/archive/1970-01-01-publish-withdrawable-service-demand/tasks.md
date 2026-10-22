# Tasks: Publish withdrawable service demand

## Model

- [x] [serial] Record the current demand inputs, the callers that supply them, and the shutdown decision path. r[molten.runtime_spine.demand_assertion_lifetime]
- [x] [serial] Add the demand assertion record with service ref, demand kind, demander ref, and operation identity, deduplicated per owner and service. r[molten.runtime_spine.demand_assertion_lifetime]
- [x] [serial] Derive the dependency predicate input from the live demand assertion set while keeping the predicate pure. r[molten.runtime_spine.demand_assertion_lifetime]
- [x] [serial] Make shutdown eligible when the last demand assertion for a service is withdrawn, behind the existing shutdown admission gates. r[molten.runtime_spine.demand_assertion_lifetime]

## Validation

- [x] [parallel] Add positive tests: dependency-gated demand waits for readiness and then starts; force-run demand bypasses ordering by declaration; withdrawing the only demand allows shutdown. r[molten.runtime_spine.demand_assertion_lifetime]
- [x] [parallel] Add negative tests: one owner cannot double-count demand; a second live demander prevents shutdown; an unverified provider never satisfies demand; a restart message creates no demand state. r[molten.runtime_spine.demand_assertion_lifetime]
- [x] [serial] Run focused predicate, lifecycle, and dataspace tests plus Clippy before and after the change, then the workspace checks the repository requires. r[molten.runtime_spine.demand_assertion_lifetime]
- [x] [serial] Document the demand forms, the shutdown rule, and the no-capacity-inference non-claim. r[molten.runtime_spine.demand_assertion_lifetime]
