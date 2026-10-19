# Evidence Gates Specification Delta

## ADDED Requirements

### Requirement: Direct build binding enforces the provenance reference envelope

r[molten.provenance_state_proof.direct_binding_ref_envelope] The public build-verification binding helper MUST apply the existing maximum provenance-reference count to both its supplied receipts and the record's build-record references before searching for a matching receipt or collecting candidate diagnostics. An over-limit input MUST fail closed with a bounded diagnostic even when a matching receipt appears first. Exact-maximum inputs MUST retain the established binding decision.

#### Scenario: Early passing receipt cannot hide over-limit intake

- GIVEN a canonical reproducible record and a passing build-verification receipt repeated once beyond the existing maximum
- WHEN a direct caller requests binding
- THEN the helper MUST deny the oversized intake before accepting the first passing receipt

#### Scenario: Forged record refs and failures stay bounded

- GIVEN either a public record with one build-record ref beyond the existing maximum or an over-limit list of failing receipts
- WHEN a direct caller requests binding
- THEN the helper MUST return one bounded-count denial rather than scanning the forged refs or accumulating per-receipt diagnostics

#### Scenario: Exact-maximum binding remains valid

- GIVEN a canonical record and valid receipts exactly at the existing maximum
- WHEN a direct caller requests binding
- THEN the helper MUST preserve the matching artifact and build-record binding
