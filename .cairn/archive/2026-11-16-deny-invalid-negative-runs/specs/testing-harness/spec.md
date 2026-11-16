# Testing Harness Delta

## MODIFIED Requirements

### Requirement: Coverage is derived from receipts
r[molten.testing.receipt_driven_traceability.coverage_derivation] Receipt-driven traceability MUST derive requirement id, coverage kind, target, command identity, artifact refs, and diagnostics from validated canonical receipt fields rather than from rendered logs. Run diagnostics MUST equal the diagnostics implied by the receipt's coverage kind, exit status and produced artifacts.

#### Scenario: Derived entry binds artifact refs
- GIVEN a receipt with produced artifact refs
- WHEN coverage derivation runs
- THEN the derived traceability entry names those refs and validates their content-ref shape.

#### Scenario: A dropped run diagnostic is rejected
- GIVEN a receipt for a negative run that exited zero, or that produced no artifact, assembled without its run diagnostic
- WHEN coverage derivation parses it
- THEN parsing fails instead of producing evidence.

### Requirement: Stale receipt coverage denies
r[molten.testing.receipt_driven_traceability.stale_receipt_denial] Receipt-driven traceability MUST deny stale receipt refs, duplicate coverage receipts for the same slot unless policy permits aggregation, wrong requirement ids, wrong coverage kinds, malformed refs, and receipts whose decision cannot satisfy the requested coverage kind. A receipt recording an invalid run MUST NOT satisfy coverage even when its decision string matches, and its run diagnostics MUST remain visible in the entry diagnostics.

#### Scenario: Wrong requirement receipt fails
- GIVEN a coverage slot for one requirement
- AND a receipt naming another requirement id
- WHEN traceability derives coverage
- THEN the slot remains uncovered and the receipt is reported as stale or mismatched.

#### Scenario: Invalid negative run does not discharge coverage
- GIVEN valid positive coverage and a negative receipt that exited zero or produced no artifact
- WHEN the strict manifest is built
- THEN the entry is reported stale with an invalid-run diagnostic naming the reason
- AND the manifest decision is deny.
