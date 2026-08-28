# Fabric Simulation Specification Delta

## ADDED Requirements

### Requirement: One scheduler controls every modeled communication choice

r[molten.message_boundary.scheduler_closure]

Deterministic simulation MUST make runnable selection, message delivery, timer firing, storage completion, process lifecycle completion, fault activation, authority change, resource outcome, and every other modeled nondeterministic completion explicit under one bounded scheduler.

Each choice MUST bind canonical position, virtual time, eligible-set identity, selected alternative, and replay behavior.

#### Scenario: Message delivery is selected explicitly

- GIVEN multiple messages are eligible for different state owners
- WHEN deterministic simulation advances
- THEN the scheduler MUST record the complete eligible set and selected delivery
- AND no adapter MAY invoke a core transition outside that selected delivery.

#### Scenario: Storage completion bypasses the scheduler

- GIVEN a deterministic adapter completes a write and calls the core directly
- WHEN scheduler-closure validation runs
- THEN validation MUST fail at the unrecorded completion boundary.

### Requirement: Simulation and live execution use the same message core

r[molten.message_boundary.same_core]

Deterministic simulation and supported live profiles MUST use the same state-transition artifact identity, application state types, message schemas, callback dispatcher, effect-plan types, and protocol invariants.

Simulation-specific replacement MUST remain limited to admitted shell adapters and top-level scheduling.

#### Scenario: Same core identity passes

- GIVEN one admitted extension runs through deterministic and live compositions
- WHEN same-core conformance compares their identities
- THEN transition, message, callback, state, effect-plan, and invariant identities MUST match apart from declared shell bindings.

#### Scenario: Mock-only service reproduces expected output

- GIVEN a simulator returns expected fixture outputs without invoking the admitted transition core
- WHEN same-core conformance runs
- THEN conformance MUST fail even when output bytes match.

### Requirement: Message-boundary evidence is replayable and scoped

r[molten.message_boundary.evidence]

Deterministic message-boundary evidence MUST bind world, state-owner, message-contract, transition-core, scheduler, eligible-choice, adapter, workload, time, entropy, storage, process, authority, resource, fault, trace, final-state, divergence, and non-claim refs required by the selected profile.

#### Scenario: Complete deterministic run repeats

- GIVEN identical canonical world, core, message, adapter, scheduler, workload, and fault refs
- WHEN a bounded run repeats
- THEN choices, application traces, invariant results, outputs, and final state refs MUST match.

#### Scenario: Scheduler input is missing

- GIVEN a run omits one behavior-affecting completion source or eligible-set fact
- WHEN evidence validation runs
- THEN validation MUST classify the run as incomplete
- AND it MUST NOT satisfy deterministic-simulation admission.

### Requirement: Simulation validation includes bypass failures

r[molten.message_boundary.verification]

Whole-system simulation validation MUST include positive and negative fixtures for message admission, scheduler closure, same-core identity, live and deterministic parity, handle containment, callback routing, shared-state bypass, retries, uncertain delivery, faults, replay, first divergence, cleanup, and claim boundaries.

#### Scenario: Valid message-oriented world passes

- GIVEN every state-owner boundary uses canonical messages and every modeled choice uses the scheduler
- WHEN whole-system validation runs
- THEN the world MUST pass message-boundary and scheduler-closure checks.

#### Scenario: Hidden callback bypass fails

- GIVEN an adapter invokes a state owner through an unrecorded callback
- WHEN whole-system validation runs
- THEN validation MUST fail and identify the callback bypass.
