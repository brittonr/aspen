## ADDED Requirements

### Requirement: Octet module-file-count caveat closure is explicit
r[molten.octet_burndown.module_file_count_scope_closure] Molten MUST NOT remove the global `module_file_count` Octet disabled-lint caveat unless a current no-disabled probe and deterministic source inventory prove that every remaining `module_file_count` row is external/remapped, registry/rustlib, or integration-test residue, with zero Molten-owned and zero unknown rows hidden from strict source-gate evidence.

#### Scenario: Broad module-file-count disable is removed only after proof
- GIVEN a current no-disabled Octet probe reports `module_file_count` rows
- WHEN Molten removes or narrows the global disabled-lint caveat
- THEN the evidence classifies every reported row
- AND the evidence shows zero Molten-owned source rows and zero unknown rows are hidden by the final source-gate configuration

#### Scenario: Unknown rows keep the gate blocked
- GIVEN a `module_file_count` row cannot be classified as external/remapped, registry/rustlib, or integration-test residue
- WHEN source-gate evidence is refreshed
- THEN Molten keeps the finding actionable or blocked and MUST NOT claim strict source-gate closure
