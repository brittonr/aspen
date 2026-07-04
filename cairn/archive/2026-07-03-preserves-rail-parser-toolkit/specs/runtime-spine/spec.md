## ADDED Requirements

### Requirement: Preserves rail common parser/builders are shared
r[molten.preserves_rail_toolkit.parser_builders] Molten SHOULD centralize common Preserves parser and builder helpers for simple records, required strings, content refs, optional refs, ref sequences, and schema fields.

#### Scenario: Shared helper preserves canonical value
- GIVEN a record family migrated from local helpers to shared helpers
- WHEN the same logical input is rendered before and after migration
- THEN the canonical Preserves value ref is unchanged
- AND invalid input still fails closed.

### Requirement: Check-set parsing is consistent
r[molten.preserves_rail_toolkit.check_sets] Molten MUST parse and build common `<checks [...]>` fields consistently across migrated record families.

#### Scenario: Missing required check denies
- GIVEN a migrated parser receives a record missing a required check
- WHEN check-set validation runs
- THEN parsing is `deny` or returns a structured invalid-input error
- AND diagnostics name the missing check and record family.

### Requirement: Shared helpers reject malformed shapes
r[molten.preserves_rail_toolkit.negative_shapes] Molten MUST provide negative coverage for wrong labels, wrong arity, wrong field type, invalid refs, missing checks, and unsupported check shapes in the shared helper layer.

#### Scenario: Invalid ref is rejected by shared helper
- GIVEN a record field expected to contain a canonical content ref
- WHEN the field contains a non-canonical ref string
- THEN the shared helper rejects the record
- AND no caller treats the value as admitted.

### Requirement: Helper migrations preserve hashes
r[molten.preserves_rail_toolkit.hash_stability] Molten MUST prove that helper-only migrations do not change canonical hashes for representative public receipt and artifact fixtures.

#### Scenario: Receipt hash remains stable
- GIVEN a representative public receipt fixture
- WHEN its builder migrates from local helpers to shared helpers
- THEN the receipt canonical hash remains unchanged
- AND only diagnostics for invalid inputs may become more specific.
