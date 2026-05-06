# Review testing harness public API Delta

## ADDED Requirements

### Requirement: Testing core default is reusable
The testing harness review MUST define the reusable default API surface for `aspen-testing-core` without requiring runtime app, network namespace, VM, patchbay, or madsim adapters.
ID: testing-harness-extraction.testing-core-default-reusable

#### Scenario: Testing core default is reusable evidence
ID: testing-harness-extraction.testing-core-default-reusable.evidence
- GIVEN a downstream fixture depends on `aspen-testing-core` with default or minimal features
- WHEN the fixture compiles and records metadata
- THEN it SHALL exercise reusable smoke helpers without importing Aspen app shells or adapter-only crates.

### Requirement: Adapters are explicit and negatively checked
Testing harness adapters MUST be explicit feature or crate boundaries with negative checks proving they do not leak into reusable defaults.
ID: testing-harness-extraction.adapters-explicit-negative-checked

#### Scenario: Adapters are explicit and negatively checked evidence
ID: testing-harness-extraction.adapters-explicit-negative-checked.evidence
- GIVEN madsim, network, patchbay, VM, or runtime adapters are present
- WHEN dependency policy checks run
- THEN each adapter dependency SHALL be absent from reusable defaults and documented as adapter-owned when enabled.

### Requirement: Testing harness workspace readiness is evidenced
The testing harness review MUST promote the family only when docs, inventory, policy, fixtures, and readiness-checker outputs agree on the same readiness state.
ID: testing-harness-extraction.workspace-readiness-evidenced

#### Scenario: Testing harness workspace readiness is evidenced evidence
ID: testing-harness-extraction.workspace-readiness-evidenced.evidence
- GIVEN the family remains blocked from publication or repo split by license/publication policy
- WHEN the in-workspace public API review is complete
- THEN the family SHALL be marked `extraction-ready-in-workspace` with verification artifacts and SHALL NOT claim publishable/repo-split readiness.
