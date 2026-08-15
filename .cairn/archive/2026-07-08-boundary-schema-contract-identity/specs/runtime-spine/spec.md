## ADDED Requirements

### Requirement: Boundary schema refs bind full field contracts
r[molten.boundary_schema_contract_identity.full_contract_ref] Molten MUST derive Preserves boundary schema refs from the full reviewed field contract, including family, version, record label, schema id, ordered field labels, field kinds, and declared constraints.

#### Scenario: Same arity but different field kind changes ref
- GIVEN two boundary schema specs with the same family, version, record label, schema id, and arity
- AND one spec changes a field from a string domain to a content-ref domain
- WHEN Molten derives the schema refs
- THEN the refs differ
- AND the changed spec cannot silently satisfy stale evidence from the previous contract.

### Requirement: Boundary schema validation reports stale schema identity
r[molten.boundary_schema_contract_identity.stale_schema_denial] Molten MUST fail closed when a boundary record or receipt claims a schema ref that does not match the current reviewed boundary field contract for that family and version.

#### Scenario: Stale schema ref denies before semantic admission
- GIVEN a schema-backed boundary record with a claimed schema ref from an older same-arity contract
- WHEN boundary validation runs
- THEN validation is `deny`
- AND diagnostics identify the boundary family and stale schema ref
- AND no authority, policy, resource, provenance, transport, ledger, or execution side effect is admitted.

### Requirement: Boundary receipts bind strengthened schema refs
r[molten.boundary_schema_contract_identity.receipt_binding] Schema validation receipts or diagnostics SHOULD name the strengthened schema ref so reviewers can distinguish record-shape evidence from semantic admission evidence.

#### Scenario: Validation report names contract ref
- GIVEN a boundary record passes schema validation
- WHEN Molten emits validation diagnostics or receipt evidence
- THEN the evidence names the boundary family, value ref, and strengthened schema ref
- AND rendered logs cannot replace that canonical evidence.

### Requirement: Boundary schema ref migrations are recorded
r[molten.boundary_schema_contract_identity.compatibility_note] Expected schema-ref changes from strengthened boundary contracts SHOULD be recorded in tests, fixtures, or release-evidence notes so reviewers can distinguish intentional contract migration from accidental drift.

#### Scenario: Intentional schema ref change is reviewable
- GIVEN a boundary schema ref changes because field labels, kinds, or constraints are now part of the contract identity
- WHEN reviewers inspect the migration evidence
- THEN the tests or notes explain that the ref changed due to strengthened schema identity
- AND no semantic authority is inferred from that compatibility note.
