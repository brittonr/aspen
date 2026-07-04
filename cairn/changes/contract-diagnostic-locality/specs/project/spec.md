# Project Delta: Contract diagnostic locality

### Requirement: Contract validation diagnostics identify fields or invariants
r[molten.project.contract_diagnostics.locality] Repository-owned Nickel contract validation SHOULD report or name the failing field, domain helper, fixture, or cross-field invariant closely enough that reviewers can distinguish malformed input from unrelated import, parse, or tooling failures.

#### Scenario: Field-domain failure is local
- GIVEN a contract fixture with one malformed BLAKE3 ref, invalid enum value, unsafe path, or empty required array
- WHEN fixture validation fails
- THEN the failure output or fixture expectation identifies the intended field-domain invariant

#### Scenario: Cross-field failure is local
- GIVEN a contract fixture with an inverted validity window, duplicate descriptor, stale internal reference, or contradictory resource limit
- WHEN fixture validation fails
- THEN the failure output or fixture expectation identifies the intended cross-field invariant

### Requirement: Diagnostic improvements preserve fail-closed behavior
r[molten.project.contract_diagnostics.no_validation_weakening] Refactoring contracts for clearer diagnostics MUST NOT cause previously rejected malformed fixtures to export successfully or weaken runtime Rust admission of checked-in evidence.

#### Scenario: Diagnostic refactor keeps negative fixtures failing
- GIVEN a contract module is refactored into field-level contracts and named predicates
- WHEN the positive and negative fixture suite runs
- THEN valid fixtures still export and malformed fixtures still fail for the expected invariant classes
