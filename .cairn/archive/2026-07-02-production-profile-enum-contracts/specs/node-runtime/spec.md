# Node Runtime Delta: Production profile enum contracts

### Requirement: Production profile vocabularies are contract-bound
r[molten.prod_ops.profile_enum_contracts.allowed_vocabularies] Production profile arrays for required adapters, redaction settings, live transport settings, startup expectations, and shutdown expectations MUST accept only reviewed vocabulary values through Nickel contracts.

#### Scenario: Reviewed vocabulary values export
- GIVEN a production profile whose adapter, redaction, transport, startup, and shutdown arrays contain only reviewed vocabulary values
- WHEN the operator exports the profile through Nickel
- THEN the export succeeds and the exported values remain the reviewed strings

#### Scenario: Misspelled vocabulary value fails
- GIVEN a production profile with a misspelled or unreviewed adapter, redaction setting, transport setting, startup expectation, or shutdown expectation
- WHEN the operator exports the profile through Nickel
- THEN the export fails before the unreviewed string can be bound into production readiness evidence

### Requirement: Vocabulary growth is reviewed
r[molten.prod_ops.profile_enum_contracts.reviewed_growth] New production profile vocabulary values MUST be added through an explicit contract and documentation update rather than accepted as arbitrary text.

#### Scenario: New adapter requires contract update
- GIVEN an operator wants to require a new production adapter in the deployment profile
- WHEN the adapter name is not present in the reviewed vocabulary contract
- THEN Nickel export rejects the profile until the contract and operator documentation are updated
