## ADDED Requirements

### Requirement: Scenario fixtures are the source of truth for cluster execution plans
r[molten.testing.fixture_driven_cluster_execution.fixture_source_of_truth] Molten MUST derive evidence-bearing cluster, local-multiprocess, or VM execution plans from checked multinode scenario fixture metadata when a fixture is supplied, including topology profile, execution profile, command surface, expected artifact kinds, required receipts, variance refs, unavailable policy, and caveats.

#### Scenario: Fixture-derived plan matches declared execution surface
- GIVEN a checked multinode scenario fixture for a cluster or VM shard
- WHEN the harness derives a run plan
- THEN the plan binds the fixture ref, topology profile, command surface, expected artifact kinds, required receipts, variance refs, unavailable policy, and evidence-only caveats
- AND no pass claim can be accepted from undeclared handwritten scenario shape.

#### Scenario: Runtime code consumes checked metadata rather than evaluating Nickel
- GIVEN a Nickel-authored scenario fixture
- WHEN the Rust harness validates an execution plan
- THEN it consumes checked fixture metadata or exported fixture data
- AND it does not perform ambient runtime Nickel evaluation to decide trust.

### Requirement: Observed cluster runs are gated against fixture metadata
r[molten.testing.fixture_driven_cluster_execution.observation_gate] Molten MUST deny cluster or VM pass evidence when observed topology, command surface, artifact kinds, required child refs, unavailable policy, variance declarations, or caveats diverge from the checked scenario fixture.

#### Scenario: Mismatched artifact kind denies fixture-backed pass evidence
- GIVEN a fixture that expects canonical receipt artifact kinds
- WHEN the observed run reports only logs or a different artifact kind
- THEN the observation gate denies pass evidence
- AND diagnostics name the missing or mismatched artifact kind.

#### Scenario: Unsupported execution remains non-pass evidence
- GIVEN a fixture whose unavailable policy says unsupported execution is diagnostic-only or deny
- WHEN host support is unavailable during a cluster or VM run
- THEN the run emits unavailable or diagnostic evidence according to the fixture policy
- AND unsupported execution is not promoted to pass evidence.
