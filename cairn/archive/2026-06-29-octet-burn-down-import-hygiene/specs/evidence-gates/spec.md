## ADDED Requirements

### Requirement: Remaining Octet import-hygiene warnings are reduced without semantic churn
r[molten.octet_burndown.import_hygiene_finish] Molten MUST burn down remaining no-disabled `non_trait_imports` warnings with behavior-preserving namespace cleanup, and MUST keep command syntax, receipt schemas, canonical output values, and fail-closed behavior stable.

#### Scenario: Import cleanup preserves receipt behavior
- GIVEN a module is refactored to reduce concrete non-trait imports
- WHEN focused validation and no-disabled Octet evidence run
- THEN public behavior and canonical receipt output remain stable
- AND before/after import-hygiene counts are recorded for the touched domain.

#### Scenario: Import caveat remains visible until clean
- GIVEN `non_trait_imports` remains warning-only in no-disabled evidence
- WHEN remediation evidence is reported
- THEN Molten labels import hygiene as active burn-down work rather than source-remediated zero.
