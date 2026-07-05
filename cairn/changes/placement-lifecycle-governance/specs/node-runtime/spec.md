# Node Runtime Delta: placement lifecycle governance

### Requirement: Lifecycle probes and restart decisions are evidence-bound
r[molten.lifecycle.probes_restart_backoff] Molten MUST represent startup, readiness, liveness, graceful shutdown, restart, and terminal lifecycle decisions as evidence-bound records with observed generation, probe evidence refs, status condition refs, restart attempt summaries, named backoff profile refs, and policy refs. Restart decisions MUST deny when probe evidence is missing, restart budgets are exhausted, backoff values are unnamed, or status claims do not match observed lifecycle evidence.

#### Scenario: Readiness probe updates status
- GIVEN a running service resource with current-generation readiness probe evidence
- WHEN lifecycle evaluation records the service as ready
- THEN Molten emits a status condition bound to the probe evidence, observed generation, and lifecycle policy refs.

#### Scenario: Restart loop without budget denies
- GIVEN a service with repeated liveness failures and no remaining restart budget under the named backoff profile
- WHEN lifecycle evaluation considers another restart
- THEN Molten denies the restart plan
- AND diagnostics identify the exhausted restart governance input.

### Requirement: GC and cleanup plans respect lifecycle blockers
r[molten.lifecycle.gc_cleanup_gates] Molten MUST gate deletion, cleanup, and garbage collection through explicit owner refs, finalizer cleanup receipt refs, pin refs, retention policy refs, and deletion authority evidence. Cleanup plans MUST deny while any required blocker remains live or unresolved.

#### Scenario: Cleanup plan passes after blockers clear
- GIVEN a resource marked for cleanup with no live owners, all finalizer receipts present, no pins, no retention hold, and valid deletion authority
- WHEN GC evaluates the cleanup plan
- THEN Molten emits a cleanup pass receipt binding each cleared blocker and authority evidence.

#### Scenario: Pinned artifact blocks GC
- GIVEN a resource cleanup plan that would remove an artifact still protected by a pin ref
- WHEN GC evaluates the plan
- THEN Molten denies the cleanup plan
- AND diagnostics identify the pin blocker.
