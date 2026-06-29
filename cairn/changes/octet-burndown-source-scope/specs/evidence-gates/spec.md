## ADDED Requirements

### Requirement: Octet source-scope burn-down is explicit and fail-closed
r[molten.octet_burndown.source_scope_tooling] Molten MUST classify `module_file_count`, `underscore_in_module_filename`, and other external or remapped no-disabled findings with deterministic evidence before narrowing source-gate scope, and MUST remain fail-closed for Molten-owned source and unknown findings.

#### Scenario: External or remapped findings are classified
- GIVEN a no-disabled Octet finding reported under a `<WORKSPACE>/src/...` path
- WHEN source-scope classification runs
- THEN the finding is classified as Molten-owned source, integration-test source, generated/remapped dependency source, registry/rustlib source, or unknown
- AND the classification evidence explains why the finding is actionable, external, or blocked

#### Scenario: Unknown source-scope findings remain blocked
- GIVEN a no-disabled Octet finding cannot be confidently classified as external or remapped
- WHEN source-scope evidence is reported
- THEN Molten treats the finding as actionable or blocked rather than hiding it from source-gate evidence
