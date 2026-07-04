## ADDED Requirements

### Requirement: Multinode scenario fixtures are declarative and typed
r[molten.testing.multinode.declarative_scenario_fixtures] Molten SHOULD define typed, repository-owned multinode scenario fixtures that declare topology, profile, command surface, expected artifact kinds, deterministic seed, fault-plan refs, variance declarations, unavailable policy, and evidence-only caveats before execution.

#### Scenario: Valid fixture derives canonical metadata
- GIVEN a typed multinode scenario fixture with declared topology, profile, command surface, expected artifacts, receipt refs, variance refs, and caveats
- WHEN the testing harness validates the fixture and derives distributed CI metadata
- THEN the derived metadata binds the fixture values without reading ambient runtime state
- AND the fixture ref and metadata ref are stable for equivalent fixture content.

#### Scenario: Fixture authoring remains typed
- GIVEN a multinode scenario fixture authored in Nickel
- WHEN the fixture is exported for use by Rust validation or a NixOS VM check
- THEN the export must satisfy the repository-owned fixture contract before any pass evidence can be accepted.

### Requirement: Multinode scenario fixture validation fails closed
r[molten.testing.multinode.scenario_fixture_validation] Molten MUST reject multinode scenario fixtures that omit required topology, profile, receipt, variance, unavailable-policy, or artifact-kind bindings, or that claim unsupported execution as pass evidence.

#### Scenario: Missing or mismatched fixture fields deny
- GIVEN a fixture with a missing topology, missing command surface, stale receipt ref, undeclared variance, unsupported pass claim, or mismatched artifact kind
- WHEN the fixture validator evaluates it
- THEN validation denies before generating pass metadata
- AND diagnostics identify the invalid fixture binding.

#### Scenario: Diagnostic logs do not repair invalid fixtures
- GIVEN an invalid multinode scenario fixture and diagnostic logs that appear to show success
- WHEN the evidence gate evaluates the fixture
- THEN the gate rejects the fixture because canonical fixture and receipt bindings, not logs, determine pass evidence.
