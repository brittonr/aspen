# Testing Harness Delta: Generated Tamper Negative Matrix

## ADDED Requirements

### Requirement: Generated tamper cases
r[molten.testing.tamper_matrix.generated_cases] Molten SHOULD provide a reusable generated or table-driven tamper matrix for evidence artifacts whose parsers or gates accept pass evidence.

#### Scenario: Matrix generates stale-ref case
- GIVEN a valid harness gate receipt fixture
- WHEN the tamper matrix generates a stale subject-ref case
- THEN the resulting fixture preserves the original control metadata and identifies the expected stale-ref denial class

### Requirement: Tampered evidence fails closed
r[molten.testing.tamper_matrix.fail_closed] Parsers and gates exercised by the tamper matrix MUST reject mutated evidence before emitting pass evidence and MUST preserve canonical diagnostics for the denial class.

#### Scenario: Tampered embedded receipt is denied
- GIVEN a valid sealed repro bundle whose embedded gate receipt is changed by the tamper matrix
- WHEN the bundle gate evaluates the mutated bundle
- THEN the gate denies before accepting pass evidence and records a receipt or seal diagnostic

### Requirement: Tamper matrix coverage is traceable
r[molten.testing.tamper_matrix.coverage] Tamper matrix coverage SHOULD be recorded in the checked-in evidence matrix or traceability receipts for the requirements it protects.

#### Scenario: Requirement lists tamper coverage
- GIVEN a requirement that depends on fail-closed bundle validation
- WHEN the evidence matrix is rendered
- THEN it lists the positive control fixture and the generated negative tamper cases that cover the requirement
