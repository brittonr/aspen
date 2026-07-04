## ADDED Requirements

### Requirement: Preserves schema adapter is pure and deterministic
r[molten.preserves_schema_boundaries.schema_adapter] Molten MUST provide a deterministic schema validation adapter for Preserves values that performs no filesystem, network, clock, process, or ambient policy access in core validation.

#### Scenario: Same value and schema produce same result
- GIVEN the same Preserves value and schema artifact
- WHEN schema validation runs repeatedly
- THEN the validation decision and diagnostics are identical
- AND the value is not mutated by validation.

### Requirement: Boundary schemas are versioned artifacts
r[molten.preserves_schema_boundaries.schema_artifacts] Molten MUST treat schemas for external Preserves boundary records as versioned artifacts with canonical refs available to receipts or diagnostics.

#### Scenario: Receipt names schema ref
- GIVEN a boundary record accepted after schema validation
- WHEN Molten emits validation or admission evidence
- THEN the evidence names the schema family and schema artifact ref
- AND rendered logs cannot replace the schema-bound evidence.

### Requirement: High-risk Preserves records validate before semantic admission
r[molten.preserves_schema_boundaries.high_risk_records] Molten MUST run schema validation before semantic admission for node-control ingress, plugin hostcalls, evidence-chain bundles, retention receipts, and release evidence bundles.

#### Scenario: Malformed high-risk record denies
- GIVEN a high-risk boundary record with a missing required field
- WHEN the boundary evaluates the record
- THEN schema validation is `deny`
- AND no authority, provenance, policy, resource, transport, ledger, or execution side effect is admitted.

### Requirement: Schema denials are negatively covered
r[molten.preserves_schema_boundaries.schema_denials] Molten MUST include negative tests for wrong record labels, missing fields, wrong field types, malformed checks, unsupported versions, and extra critical fields for every schema-backed boundary family.

#### Scenario: Wrong field type denies
- GIVEN a schema-backed boundary fixture whose content ref field is a sequence instead of a canonical ref string
- WHEN schema validation runs
- THEN validation is `deny`
- AND diagnostics identify the field and expected class.
