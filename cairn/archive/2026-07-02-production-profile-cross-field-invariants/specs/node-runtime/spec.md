# Node Runtime Delta: Production profile cross-field invariants

### Requirement: Production profile requires startup evidence inputs
r[molten.prod_ops.profile_invariants.required_evidence] Production deployment profile export MUST fail unless required evidence arrays are non-empty and the required adapter list includes the reviewed core production adapter set.

#### Scenario: Complete startup evidence exports
- GIVEN a production profile with at least one source-gate input and all reviewed core production adapters listed
- WHEN the operator exports the profile through Nickel
- THEN the export succeeds and the startup receipt can bind the declared evidence and adapter refs

#### Scenario: Missing startup evidence fails
- GIVEN a production profile with no source-gate inputs or with a required core production adapter omitted
- WHEN the operator exports the profile through Nickel
- THEN the export fails before startup receipts can claim the profile is deployment-ready

### Requirement: Production state layout directories are distinct
r[molten.prod_ops.profile_invariants.layout_distinct] Production deployment profile export MUST fail when two logical state layout directories resolve to the same relative directory name.

#### Scenario: Distinct layout directories export
- GIVEN a production profile whose ledger, Redb, chunk, identity, retention, and inbox directories are distinct relative directory names
- WHEN the operator exports the profile
- THEN the exported state layout preserves each logical directory mapping

#### Scenario: Layout collision fails
- GIVEN a production profile that assigns the same relative directory name to two logical state layout entries
- WHEN the operator exports the profile through Nickel
- THEN the export fails before runtime state can be initialized with an ambiguous layout

### Requirement: Production resource limits are internally coherent
r[molten.prod_ops.profile_invariants.resource_relationships] Production deployment profile export MUST fail when resource limits contradict each other, including store capacity smaller than receipt capacity or timing limits that invert the reviewed delivery and recovery envelope.

#### Scenario: Coherent limits export
- GIVEN a production profile whose store capacity can contain the maximum receipt size and whose timing limits preserve the reviewed delivery and recovery envelope
- WHEN the operator exports the profile
- THEN the resource-limit block exports as reviewed production profile evidence

#### Scenario: Contradictory limits fail
- GIVEN a production profile whose store limit is smaller than the maximum receipt size or whose timing limits contradict the reviewed delivery and recovery envelope
- WHEN the operator exports the profile through Nickel
- THEN the export fails with a resource-limit invariant diagnostic
