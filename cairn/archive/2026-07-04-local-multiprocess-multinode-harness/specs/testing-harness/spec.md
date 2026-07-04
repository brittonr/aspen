## ADDED Requirements

### Requirement: Local multiprocess multinode harness exercises real node processes
r[molten.testing.multinode.local_multiprocess_harness] Molten SHOULD provide a local multiprocess multinode harness that runs isolated `molten node` processes from an explicit scenario fixture and records canonical startup, workflow, shutdown, and run receipts.

#### Scenario: Cross-process control workflow records local integration evidence
- GIVEN a local multiprocess scenario fixture with separate node identities, isolated state roots, admitted local transport handles, and a valid control command
- WHEN the harness starts the node processes and runs the workflow
- THEN the run receipt binds the fixture ref, process-plan ref, startup refs, workflow receipt refs, shutdown refs, diagnostics, and evidence-only caveats
- AND the receipt states that local multiprocess evidence does not replace VM or production live evidence.

#### Scenario: Process planning stays deterministic
- GIVEN equivalent explicit process plans with the same node identities, state-root handles, command plan, and expected receipts
- WHEN the pure planner canonicalizes them
- THEN both plans produce the same plan ref without reading ports, process ids, clocks, or environment variables.

### Requirement: Local multiprocess harness isolates state and cleans up failures
r[molten.testing.multinode.process_isolation_cleanup] Molten MUST reject state-root collisions, transport-handle collisions, missing receipt bindings, stale tickets, and orphaned process or state evidence before accepting local multiprocess pass evidence.

#### Scenario: Collision fails before process start
- GIVEN a local multiprocess scenario where two nodes share a state-root handle or transport handle that must be isolated
- WHEN the harness validates the process plan
- THEN validation denies before starting the affected process
- AND diagnostics identify the colliding handle.

#### Scenario: Crash cleanup is recorded
- GIVEN a local multiprocess run where a child process crashes or is stopped during the workflow
- WHEN cleanup runs
- THEN the harness records cleanup or denial evidence
- AND no pass receipt is accepted unless required shutdown and cleanup receipts are present.
