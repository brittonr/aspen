# Review testing harness public API Delta

## ADDED Requirements

### Requirement: Testing core default is reusable [r[testing-harness-extraction.testing-core-default-reusable]]
The testing harness review MUST define the reusable default API surface for `aspen-testing-core` without requiring runtime app, network namespace, VM, patchbay, or madsim adapters.

#### Scenario: Testing core default is reusable evidence [r[testing-harness-extraction.testing-core-default-reusable.evidence]]
- GIVEN a downstream fixture depends on `aspen-testing-core` with default or minimal features
- WHEN the fixture compiles and records metadata
- THEN it SHALL exercise reusable smoke helpers without importing Aspen app shells or adapter-only crates.

### Requirement: Adapters are explicit and negatively checked [r[testing-harness-extraction.adapters-explicit-negative-checked]]
Testing harness adapters MUST be explicit feature or crate boundaries with negative checks proving they do not leak into reusable defaults.

#### Scenario: Adapters are explicit and negatively checked evidence [r[testing-harness-extraction.adapters-explicit-negative-checked.evidence]]
- GIVEN madsim, network, patchbay, VM, or runtime adapters are present
- WHEN dependency policy checks run
- THEN each adapter dependency SHALL be absent from reusable defaults and documented as adapter-owned when enabled.
