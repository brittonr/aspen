# Tasks: multinode-declarative-scenario-fixtures

## Phase 1: Fixture contract

- [ ] [parallel] r[molten.testing.multinode.declarative_scenario_fixtures] Add a typed Nickel contract for multinode scenario fixtures and checked-in positive fixtures for the existing fast, protocol, VM smoke, VM fault, and soak evidence surfaces.
- [ ] [parallel] r[molten.testing.multinode.scenario_fixture_validation] Add negative fixture examples for missing topology, missing command surface, stale receipt ref, undeclared variance, unsupported pass claim, and mismatched artifact kind.

## Phase 2: Pure validation and metadata derivation

- [ ] [serial] r[molten.testing.multinode.declarative_scenario_fixtures] Add a pure fixture validator and metadata builder that consumes explicit fixture values and returns canonical refs, diagnostics, and derived distributed CI metadata.
- [ ] [serial] r[molten.testing.multinode.scenario_fixture_validation] Add positive and negative tests proving invalid fixtures deny before pass evidence and valid fixtures produce stable refs.

## Phase 3: Documentation and gates

- [ ] [parallel] r[molten.testing.multinode.declarative_scenario_fixtures] Document how reviewers inspect fixture refs, metadata refs, and caveats.
- [ ] [serial] r[molten.testing.multinode.scenario_fixture_validation] Run focused fixture tests and `cairn validate --root .`, or record the blocker and next best check.
