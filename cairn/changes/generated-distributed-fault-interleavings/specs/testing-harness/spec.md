## ADDED Requirements

### Requirement: Distributed simulation uses generated fault interleaving properties
r[molten.testing.distributed_simulation.generated_fault_interleavings] Molten SHOULD use bounded generated properties to exercise combinations of topology, scheduler profile, command sequence, evidence refs, and fault-plan interleavings at the pure distributed simulation boundary.

#### Scenario: Generated benign interleavings remain deterministic
- GIVEN a generated topology, scheduler profile, deterministic seed, command sequence, and benign fault-plan interleaving within supported bounds
- WHEN the simulator runs the generated case more than once
- THEN run receipt refs, event refs, final-state refs, committed operation ids, and diagnostics remain stable
- AND duplicate, restart, crash, delay, drop, reorder, and rejoin behavior preserves the declared invariants.

#### Scenario: Generated denial interleavings fail before side effects
- GIVEN a generated command sequence with missing authority, unauthorized transport, stale evidence, corrupted receipt, resource pressure, ambient drift, or partitioned quorum inputs
- WHEN the simulator evaluates the generated case
- THEN affected commands deny before side effects
- AND denied operation ids and diagnostics are recorded in canonical evidence.

### Requirement: Generated failures preserve replayable seeds
r[molten.testing.distributed_simulation.generated_repro_seed] Molten MUST preserve enough generated-case data to replay or inspect a failing distributed simulation property without relying on ambient randomness, clocks, host paths, or process state.

#### Scenario: Failing generated case emits repro artifact
- GIVEN a generated distributed simulation property failure
- WHEN the test harness records the failure
- THEN the repro artifact binds seed, topology, scheduler profile, fault plan, commands, invariant name, diagnostics, and receipt refs
- AND the artifact is diagnostic-only unless a later gate validates it as pass or deny evidence.

#### Scenario: Replayed seed reproduces the same canonical refs
- GIVEN a generated-case repro artifact and the same simulator version
- WHEN the harness replays the stored seed and explicit inputs
- THEN the replay produces the same relevant topology, fault-plan, event, final-state, and run receipt refs or reports a schema/version mismatch.
