## ADDED Requirements

### Requirement: Octet safety-polish burn-down is tested
r[molten.octet_burndown.safety_polish] Molten MUST track lower-count no-disabled warning families that can affect correctness or clarity as a dedicated Cairn change category, and MUST validate logic-affecting remediation with positive and negative tests before counting it as burn-down progress.

#### Scenario: Safety-polish slice preserves or improves invariants
- GIVEN a remediation for borrowed argument types, platform-dependent casts, unbounded collection growth, boolean naming, unchecked division, ignored results, or nested conditionals
- WHEN focused validation runs
- THEN positive and negative tests cover any changed core behavior
- AND the no-disabled Octet probe records before/after counts for the touched warning family

#### Scenario: Safety warnings are not hidden by mechanical refactors
- GIVEN a broad source-shape refactor touches a lower-count safety warning
- WHEN remediation evidence is reported
- THEN the warning is either validated under this category or remains visible as active safety-polish work
