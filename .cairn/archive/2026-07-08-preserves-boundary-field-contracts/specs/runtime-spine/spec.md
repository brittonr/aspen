## ADDED Requirements

### Requirement: Boundary fields declare narrow contracts
r[molten.preserves_boundary_field_contracts.field_contracts] Molten MUST support boundary field contracts that express non-empty strings, stable ids, exact strings, allowed string vocabularies, bounded sequences, non-empty ref sequences, unique ref sequences, and typed embedded records where those domains are part of the reviewed boundary shape.

#### Scenario: Invalid boundary vocabulary denies
- GIVEN a schema-backed boundary field declared as an allowed decision vocabulary
- WHEN the field contains an unsupported string
- THEN boundary validation is `deny`
- AND semantic admission for that record is not attempted.

### Requirement: High-risk boundaries avoid over-broad field classes
r[molten.preserves_boundary_field_contracts.high_risk_tightening] High-risk external Preserves boundaries SHOULD use the narrowest reviewed boundary field contract instead of `StringRecord`, `RefSequenceRecord`, `AnyRecord`, or `AnySequenceRecord` when the field's domain is known.

#### Scenario: Required authoring ref set is empty
- GIVEN a high-risk authoring boundary record whose reviewed contract requires non-empty policy or supply-chain refs
- WHEN that required ref field is an empty sequence
- THEN boundary validation denies before authority, policy, provenance, resource, transport, replay, ledger, or execution side effects.

### Requirement: Shape contracts do not grant semantic trust
r[molten.preserves_boundary_field_contracts.semantic_boundary] Passing boundary field contracts MUST remain shape evidence only and MUST NOT grant authority, provenance, policy, resource, transport, replay, retention, deletion, or execution trust.

#### Scenario: Shape-valid record still lacks authority
- GIVEN a boundary record that is shape-valid but lacks the subsystem's required authority evidence
- WHEN semantic admission evaluates it
- THEN the subsystem denies as required by its authority gate
- AND the shape validation report cannot override that denial.

### Requirement: Boundary contract denials are negatively covered
r[molten.preserves_boundary_field_contracts.field_contract_denials] Molten MUST include positive and negative tests for every reusable boundary field contract before using it at a new trust boundary.

#### Scenario: Duplicate unique ref fixture denies
- GIVEN a boundary field declared as a unique content-ref sequence
- WHEN a fixture repeats the same ref
- THEN validation denies with diagnostics naming the duplicate-ref invariant.
