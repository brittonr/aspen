## ADDED Requirements

### Requirement: Distributed simulation fault plans are canonical
r[molten.testing.distributed_simulation.fault_plan_schema] Molten MUST define canonical distributed simulation records for topology, deterministic seed, scheduler profile, and fault plan. Fault plans MUST bind delay, drop, duplicate, reorder, partition, rejoin, crash, restart, and resource-pressure events by explicit peer, channel, operation, or time-window identifiers rather than ambient runtime state.

#### Scenario: Fault plan identity is stable
- GIVEN the same simulated topology, scheduler profile, seed, and ordered fault events
- WHEN Molten canonicalizes the simulation input
- THEN the resulting fault-plan ref is stable
- AND changing any peer, operation, event, or schedule field changes the canonical ref.

### Requirement: Distributed simulation core is deterministic
r[molten.testing.distributed_simulation.simulator_core] Molten MUST provide a pure deterministic distributed simulation core that evaluates explicit topology state, virtual time, queued messages, workflow commands, and fault events without reading clocks, files, networks, process state, environment variables, or ambient randomness.

#### Scenario: Same seed produces same simulated evidence
- GIVEN identical topology, seed, scheduler profile, workflow commands, and fault plan
- WHEN the simulator runs twice in fresh process state
- THEN both runs emit the same semantic event refs, final state refs, decisions, and diagnostics.

#### Scenario: Ambient state cannot affect simulation
- GIVEN a simulation input with no declared host or environment fields
- WHEN host paths, wall-clock time, process ids, or network availability differ
- THEN the simulator output remains unchanged or denies because an explicit required input is missing.

### Requirement: Distributed simulation emits run receipts
r[molten.testing.distributed_simulation.run_receipts] Molten MUST emit canonical `distributed-test-run-v1` or equivalent receipts that bind source or test binary refs, topology ref, seed ref, scheduler profile ref, fault-plan ref, child workflow refs, emitted event refs, final state refs, replay status, allowed variance declarations, diagnostics, and pass or deny decision.

#### Scenario: Run receipt explains a deny decision
- GIVEN a simulated stale-ticket, missing-authority, duplicate-operation, or partitioned workflow denial
- WHEN the simulation run receipt is emitted
- THEN the receipt identifies the first denied invariant, relevant child refs, and the fault-plan event that exposed the denial.

### Requirement: Distributed invariants have model coverage
r[molten.testing.distributed_simulation.property_invariants] Molten SHOULD cover distributed safety invariants with property or model tests, including operation-id idempotency, no authority from transport evidence, duplicate or reordered messages not advancing state twice, deny-before-side-effects, and restart replay preserving canonical refs.

#### Scenario: Duplicate delivery does not double commit
- GIVEN a generated workflow with a duplicate delivery fault for the same operation id
- WHEN the model test evaluates committed state transitions
- THEN at most one semantic commit is accepted for that operation id
- AND any replayed duplicate is represented by explicit idempotency evidence.

### Requirement: Distributed simulation fixtures cover positive and negative paths
r[molten.testing.distributed_simulation.fixtures] Molten SHOULD provide positive fixtures for admitted workflows under bounded benign faults and negative fixtures for stale evidence, unauthorized transport-derived trust, corrupted receipts, undeclared ambient state, and invariant violations.

#### Scenario: Unauthorized transport evidence denies
- GIVEN a simulated message with live or transport identity evidence but no matching authority, policy, or resource evidence
- WHEN the workflow attempts a privileged state transition
- THEN simulation emits a deny decision before side effects
- AND diagnostics state that transport evidence does not grant authority.

### Requirement: Distributed simulation docs explain evidence scope
r[molten.testing.distributed_simulation.docs] User-facing documentation SHOULD explain how distributed simulation evidence complements unit, CLI, VM, and live soak evidence, and MUST state that simulation receipts do not grant authority, policy, provenance, resource, source-gate, retention, transport, destructive-operation, or deployment trust.

#### Scenario: Reviewer distinguishes simulation from VM evidence
- GIVEN a reviewer inspects distributed simulation output
- WHEN they follow the documentation
- THEN they can identify the topology, seed, fault plan, canonical run receipt, covered invariants, and claims that remain reserved for VM or live pilot evidence.
