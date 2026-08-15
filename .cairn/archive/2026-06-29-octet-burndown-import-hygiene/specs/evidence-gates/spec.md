## ADDED Requirements

### Requirement: Octet import-hygiene burn-down is isolated
r[molten.octet_burndown.import_hygiene] Molten MUST track `non_trait_imports` burn-down as a dedicated Cairn change category, preserving command behavior, receipt schemas, canonical Preserves values, and source-gate caveats until refreshed no-disabled evidence proves the category is clean or explicitly scoped.

#### Scenario: Import-hygiene slice preserves behavior
- GIVEN a refactor whose primary purpose is reducing `non_trait_imports`
- WHEN focused validation and the no-disabled Octet probe run
- THEN the evidence records the before/after import warning count
- AND public command syntax, receipt labels, and canonical output values remain stable

#### Scenario: Import caveat remains visible until clean
- GIVEN `non_trait_imports` remains disabled or warning-only in no-disabled evidence
- WHEN Octet remediation evidence is reported
- THEN Molten labels import hygiene as an active burn-down category rather than source-remediated zero
