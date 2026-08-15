## ADDED Requirements

### Requirement: Consensus fault matrix is deterministic
r[molten.testing.consensus_fault_matrix] Molten SHOULD include deterministic consensus simulation fixtures for failed leader, slow leader, concurrent proposals, majority partition progress, minority partition denial, stale linearizable read denial, and local-stale read classification. Fixtures MUST bind topology refs, algorithm profile refs, membership refs, fault-plan refs, operation ids, expected decisions, final-state refs, and receipt refs.

#### Scenario: Majority-connected control plane makes progress
- GIVEN a deterministic simulation with an admitted consensus profile, a declared fault plan, and a majority-connected set of replicas
- WHEN a valid control-plane command is proposed through an admitted path
- THEN the simulation emits a pass receipt with committed operation ids, final-state ref, quorum evidence, and fault diagnostics.

#### Scenario: Minority partition denies progress
- GIVEN a deterministic simulation where a replica or partition cannot reach an admitted majority
- WHEN a linearizable read or mutating control-plane command is attempted
- THEN the simulation emits denial evidence before semantic commit
- AND diagnostics name the missing majority or freshness evidence.

#### Scenario: Stale read classification is stable
- GIVEN a replica has lagging local state and a client requests either linearizable or local-stale read behavior
- WHEN the simulation evaluates the read
- THEN the linearizable read denies unless freshness evidence is present
- AND the local-stale read emits a stable non-authoritative receipt classification.

### Requirement: Leaderless experimental fixtures cover positive and negative paths
r[molten.testing.leaderless_experimental_fixtures] If Molten implements an experimental leaderless quorum profile, Molten MUST include deterministic fixtures showing majority-connected non-leader proposal progress, concurrent proposal convergence, denied minority proposals, denied missing experimental evidence, and denied production admission without accepted policy/proof/simulation evidence.

#### Scenario: Non-leader proposal can commit experimentally
- GIVEN an admitted experimental leaderless simulation profile and a majority-connected non-leader replica
- WHEN the replica proposes a valid control-plane command
- THEN the command commits only through the profile's quorum rule
- AND the receipt records the proposer, quorum evidence, final-state ref, and experimental caveat.

#### Scenario: Concurrent proposals converge or deny deterministically
- GIVEN multiple replicas propose concurrent commands for the same decision point under the experimental profile
- WHEN the deterministic scheduler explores the declared ordering
- THEN the simulation either decides one canonical outcome with matching replica state refs or denies unsafe progress
- AND no fixture accepts divergent decided values for the same slot or log position.

#### Scenario: Experimental evidence missing denies production admission
- GIVEN a manifest requests production use of the experimental leaderless profile without accepted proof/model, policy, simulation, placement, or membership evidence
- WHEN gate validation evaluates the manifest
- THEN admission denies
- AND diagnostics state which evidence class is missing.

### Requirement: Consensus placement fixtures cover safe and unsafe plans
r[molten.testing.consensus_placement_fixtures] Molten SHOULD include placement fixtures for admitted fault-domain placement, missing placement evidence, unsafe member concentration, membership-policy drift, stale placement refs, and latency-diagnostic readback.

#### Scenario: Admitted placement binds group evidence
- GIVEN a consensus group placement plan satisfies configured fault-domain and membership policy
- WHEN the placement fixture renders evidence
- THEN the placement report binds selected members, policy refs, membership refs, majority-reachability assumptions, and diagnostics
- AND the group manifest can reference that placement report.

#### Scenario: Unsafe placement is rejected
- GIVEN a placement plan has missing evidence, stale membership refs, disallowed concentration, or policy drift
- WHEN group installation or placement validation runs
- THEN Molten denies the placement before group installation
- AND diagnostics identify the unsafe placement reason.
