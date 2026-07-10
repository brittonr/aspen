## ADDED Requirements

### Requirement: Local multiprocess cluster tier bridges CLI and VM testing
r[molten.testing.local_multiprocess_cluster_tier.middle_tier] Molten MUST provide a local multiprocess cluster testing tier that runs real child processes from an explicit fixture-derived plan and emits canonical local executable-run receipts binding node ids, isolated state-root handles, transport handles, command-plan refs, expected receipt refs, timeout policy, cleanup policy, and local-evidence caveats.

#### Scenario: Local multiprocess cluster run records child evidence
- GIVEN a fixture-derived local multiprocess plan with isolated node state roots and transport handles
- WHEN the runner executes a cluster workflow through child processes
- THEN it emits a canonical run receipt binding startup, workflow, shutdown, and cleanup refs
- AND the receipt states that the evidence is local integration evidence, not VM or WAN evidence.

#### Scenario: Plan denies collisions before launch
- GIVEN a local multiprocess plan with duplicate state-root handles or transport handles
- WHEN the planner validates the run
- THEN it denies before spawning child processes
- AND diagnostics identify the colliding handles.

### Requirement: Local multiprocess negatives fail closed before pass evidence
r[molten.testing.local_multiprocess_cluster_tier.cleanup_negatives] Molten MUST deny local multiprocess cluster pass evidence when tickets are stale, children time out, orphaned children remain, required workflow receipts are missing, shutdown receipts are missing, or cleanup fails.

#### Scenario: Child timeout and orphan deny the run
- GIVEN a local multiprocess cluster run where a child times out and remains orphaned
- WHEN the run receipt is finalized
- THEN the decision is deny
- AND diagnostics bind the timed-out child and orphan observation.

#### Scenario: Cleanup failure is not hidden by successful workflow output
- GIVEN a local multiprocess workflow whose command receipts pass but cleanup fails
- WHEN the run receipt is evaluated
- THEN pass evidence is denied
- AND rendered command output remains diagnostic-only.
