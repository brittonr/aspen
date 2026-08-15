# Node Runtime Delta: Production profile contract library split

### Requirement: Production profile contracts are reusable
r[molten.prod_ops.profile_contract_library.reusable_module] Production deployment profile Nickel contracts and constants MUST live in a reusable module that can be imported by the checked-in profile and by validation fixtures.

#### Scenario: Profile and fixtures share one contract
- GIVEN the checked-in production profile and profile validation fixtures
- WHEN they are evaluated through Nickel
- THEN they import the same reusable production profile contract module rather than carrying copied schema definitions

#### Scenario: Contract update applies to all profiles
- GIVEN a production profile contract is tightened or extended
- WHEN profile instances and fixtures are exported
- THEN each import path observes the same reviewed contract behavior

### Requirement: Checked-in profile remains a concrete instance
r[molten.prod_ops.profile_contract_library.instance_profile] The operator-facing production profile file MUST remain a concrete deployment profile instance that applies the reusable contract to reviewed values.

#### Scenario: Operator exports concrete profile
- GIVEN an operator follows the production deployment runbook
- WHEN they export the checked-in production profile file
- THEN the exported JSON represents the concrete reviewed profile instance, not only the reusable contract module

### Requirement: Runtime does not evaluate Nickel for startup
r[molten.prod_ops.profile_contract_library.no_runtime_nickel] Node startup MUST continue to consume checked exported profile JSON and MUST NOT introduce runtime Nickel evaluation as part of production startup side effects.

#### Scenario: Startup uses exported profile evidence
- GIVEN a production node startup receives profile evidence
- WHEN startup validation runs
- THEN it validates the exported profile JSON and bound receipts without invoking a Nickel interpreter at runtime
