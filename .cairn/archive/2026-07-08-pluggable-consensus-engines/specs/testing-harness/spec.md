## ADDED Requirements

### Requirement: Consensus engine conformance fixtures are deterministic
r[molten.testing.consensus_engine_conformance] Molten MUST include deterministic conformance fixtures for each registered consensus engine profile. Fixtures MUST cover admitted proposal, duplicate operation denial, linearizable read freshness, local-stale read classification, snapshot and recovery, membership/config transition denial, canonical application-state replay, and normalized receipt shape.

#### Scenario: Registered engine passes conformance suite
- GIVEN a registered consensus engine profile with complete test fixture inputs
- WHEN the deterministic conformance suite runs for that profile
- THEN the suite emits pass receipts for proposal, read, snapshot, recovery, membership/config, and canonical replay cases
- AND each receipt binds engine profile, profile version, fixture id, input refs, final-state ref, and normalized evidence fields.

#### Scenario: Divergent application replay fails conformance
- GIVEN an engine produces a final application state ref that differs from canonical replay for the same command sequence
- WHEN the deterministic conformance suite compares expected and actual state refs
- THEN the suite fails the engine profile conformance receipt
- AND diagnostics identify the fixture id, command sequence ref, expected state ref, and actual state ref.

### Requirement: Consensus registry negative fixtures fail closed
r[molten.testing.consensus_registry_negative_fixtures] Molten MUST include negative fixtures for unknown engine profile, disabled engine profile, experimental profile requested for production, missing conformance refs, missing proof/model evidence, unsupported read consistency mode, mismatched profile version, missing placement requirements, and unsupported membership/config capability.

#### Scenario: Unknown profile fixture denies runtime construction
- GIVEN a fixture manifest names an unknown consensus engine profile
- WHEN runtime construction resolves the profile through the engine registry
- THEN the fixture emits denial evidence before opening control-plane state
- AND diagnostics identify the unsupported profile without fallback.

#### Scenario: Missing evidence fixture denies production admission
- GIVEN a registry entry lacks required conformance, proof/model, placement, membership, or policy evidence
- WHEN engine admission policy evaluates production status
- THEN the fixture emits denial evidence
- AND diagnostics name each missing evidence class.

### Requirement: Consensus switchover fixtures cover safe and unsafe transitions
r[molten.testing.consensus_switchover_fixtures] Molten SHOULD include deterministic switchover fixtures for safe source-to-target bootstrap, stale source-state denial, target admission denial, incompatible membership denial, placement drift denial, failed replay/conformance denial, stale writer fencing, and target read denial before activation.

#### Scenario: Safe switchover fixture activates target epoch
- GIVEN a source engine state ref, target engine profile, compatible membership and placement evidence, replay/conformance evidence, and operator approval refs
- WHEN the switchover fixture evaluates the plan
- THEN it emits a committed switchover receipt with target engine epoch, target bootstrap state ref, currentness evidence, and rollback posture
- AND subsequent normalized reads use the activated target epoch.

#### Scenario: Stale writer fixture denies after activation
- GIVEN a switchover fixture has activated a target engine epoch
- WHEN a delayed source-engine write receipt from the prior epoch is replayed
- THEN the fixture denies mutation authority for that receipt
- AND diagnostics identify the superseded engine epoch.
