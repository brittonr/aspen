# Testing Harness Delta: Checked-in Evidence Matrix

## ADDED Requirements

### Requirement: Checked-in test evidence matrix
r[molten.testing.evidence_matrix.checked_in_manifest] Molten SHOULD maintain a checked-in requirement-to-test evidence matrix for testing-harness requirements, with typed entries for requirement ids, coverage kinds, targets, commands, artifact refs, evidence scope, and caveats.

#### Scenario: Reviewer inspects matrix coverage
- GIVEN a testing-harness requirement that is implemented or changed
- WHEN a reviewer inspects the checked-in matrix
- THEN the matrix identifies the requirement's positive evidence, negative evidence, and any property, CLI, integration, or exemption evidence entries

### Requirement: Changed requirements require positive and negative evidence
r[molten.testing.evidence_matrix.changed_requirement_gate] The matrix gate MUST fail closed for changed evidence-bearing requirements that lack positive coverage, negative coverage, or an accepted exemption.

#### Scenario: Missing negative coverage denies
- GIVEN a changed evidence-bearing testing-harness requirement with positive coverage only
- WHEN the matrix gate evaluates the checked-in matrix
- THEN the gate denies the matrix with a missing-negative diagnostic for that requirement

### Requirement: Matrix entries are receipt-backed or explicitly scoped
r[molten.testing.evidence_matrix.receipt_backed_entries] Matrix entries SHOULD bind canonical receipt refs or deterministic commands and MUST reject stale requirement ids, duplicate entries, missing artifact refs, and unsupported coverage kinds.

#### Scenario: Stale requirement id fails closed
- GIVEN a matrix entry naming a requirement id that is absent from accepted specs and active changes
- WHEN the matrix gate validates the entry
- THEN the gate denies the matrix with a stale-reference diagnostic

### Requirement: Matrix exemptions are explicit and diagnostic-only
r[molten.testing.evidence_matrix.exemptions] Coverage exemptions MUST carry a reason class, evidence path or receipt ref, scope, and review note, and MUST NOT satisfy pass evidence for behavioral requirements unless policy explicitly allows it.

#### Scenario: Documentation-only exemption is visible
- GIVEN a documentation-only testing-harness requirement with no executable coverage
- WHEN the matrix includes an exemption for that requirement
- THEN the matrix records the exemption reason and evidence path without treating it as behavioral pass evidence
