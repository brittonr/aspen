# Runtime Spine Delta

## MODIFIED Requirements

### Requirement: Check-set parsing is consistent
r[molten.preserves_rail_toolkit.check_sets] Molten MUST parse and build common `<checks [...]>` fields consistently across migrated record families. Check lists are producer-described labels: a parser that requires check names establishes record shape only, and admission MUST NOT treat a check's presence or a constant `pass` status as verification unless the family computes that status from its inputs and the consumer verifies the value.

#### Scenario: Missing required check denies
- GIVEN a migrated parser receives a record missing a required check
- WHEN check-set validation runs
- THEN parsing is `deny` or returns a structured invalid-input error
- AND diagnostics name the missing check and record family.

#### Scenario: Labels do not override a denial
- GIVEN a receipt whose decision is `deny` and whose check list names every required check with status `pass`
- WHEN a consumer admits it
- THEN admission denies on the decision
- AND the required-name parse establishes only the record's shape.
