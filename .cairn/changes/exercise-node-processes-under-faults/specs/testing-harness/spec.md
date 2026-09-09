# Testing harness: outside-in fault track delta

## ADDED Requirements

### Requirement: Scenarios drive the real stack
r[molten.outside_in.real_stack] Molten outside-in scenarios MUST run ordinary node processes over actual Iroh transport and actual local Redb stores through public entry points, with bounded declared budgets and harness-owned teardown.

#### Scenario: No-fault baseline passes
- GIVEN the declared node processes started on the real stack
- WHEN the no-fault scenario runs a workload through public entry points
- THEN externally observed histories match the declared expected outcomes.

#### Scenario: Budgets bound every scenario
- GIVEN a scenario with declared process, wall-clock, and output-retention budgets
- WHEN the scenario exceeds any budget
- THEN the harness stops the scenario, records the typed result, and tears down owned processes and scratch paths.

### Requirement: Fault profiles change real execution
r[molten.outside_in.fault_profiles] Molten outside-in scenarios MUST support declared kill-and-restart, pause-and-resume, partition-and-heal, and lost-response profiles at stated boundaries, with restart on the same storage and no repaired state.

#### Scenario: Kill and restart preserves durability
- GIVEN processes killed at a declared boundary after a committed operation
- WHEN the scenario restarts them on the same storage
- THEN the committed operation is still externally observable and no state was reset.

#### Scenario: Lost response stays uncertain
- GIVEN a client request whose response was lost at a declared boundary
- WHEN the scenario reports the client outcome
- THEN the outcome remains unresolved rather than reported as success.

### Requirement: Observation uses external histories
r[molten.outside_in.observation] Molten outside-in scenario agreement MUST be judged on externally observed histories through public interfaces, with protocol projections collected only alongside, and MUST NOT compare physical timing.

#### Scenario: Histories agree after stabilization
- GIVEN a completed fault phase and declared stabilization
- WHEN external histories are collected
- THEN agreement is decided on observed outcomes, not timing or internal state.

#### Scenario: Divergence is typed, not fatal
- GIVEN observed histories that differ within the contract
- WHEN the scenario result is produced
- THEN the result is a typed permitted divergence with a recorded reason or a typed failure naming the first mismatch.

### Requirement: Validation limits outside-in evidence
r[molten.outside_in.validation] Molten MUST retain positive and negative repository or harness tests for the track and MUST NOT claim production admission, simulation replacement, or performance characteristics from outside-in scenarios.

#### Scenario: Non-claims accompany results
- GIVEN any executed outside-in scenario
- WHEN results are reported
- THEN they carry the no-production, no-simulation-replacement, and no-performance non-claims.

#### Scenario: Consensus path stays excluded
- GIVEN the declared scope boundary
- WHEN scenarios are selected
- THEN no consensus-path scenario runs under this track.
