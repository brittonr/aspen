## ADDED Requirements

### Requirement: Lifecycle state model
r[molten.lifecycle.state_model] Molten MUST define canonical lifecycle states and transition records for actors, services, vats, sessions, handlers, and jobs.

#### Scenario: Transition binds entity and state
- GIVEN a runtime entity with a current lifecycle state
- WHEN the entity records a lifecycle transition
- THEN the transition binds the entity kind, entity id, prior state, next state, action, cause, policy refs, resource refs, evidence refs, optional supervisor ref, and logical step

#### Scenario: Invalid transition is denied
- GIVEN a lifecycle transition that jumps across required intermediate states
- WHEN Molten evaluates the transition
- THEN the transition receipt is denied with diagnostics and no compatibility claim is made for BEAM, OTP, or Lunatic semantics

### Requirement: Lifecycle transition receipts
r[molten.lifecycle.transition_receipts] Molten MUST emit canonical receipts for spawn, start, ready, degraded, fail, restart, stop, cleanup, and supervisor lifecycle decisions.

#### Scenario: Receipt binds transition ref
- GIVEN a lifecycle transition value
- WHEN Molten emits a lifecycle receipt
- THEN the receipt binds the canonical transition ref, decision, diagnostics, and lifecycle evidence schema

#### Scenario: Supervisor decision is explicit evidence
- GIVEN a supervisor evaluates a child lifecycle transition
- WHEN the supervisor records its decision
- THEN Molten emits a lifecycle transition receipt instead of hiding the decision behind automatic restart behavior

### Requirement: Lifecycle prior-art boundary
r[molten.lifecycle.no_otp_compat] Molten MUST document BEAM, OTP, and Lunatic as prior art only and MUST NOT claim compatibility with their runtime, distribution, restart, mailbox, or supervision semantics.

#### Scenario: Lifecycle evidence states Molten semantics
- GIVEN lifecycle transition evidence
- WHEN the evidence is inspected
- THEN it identifies Molten-local lifecycle semantics and does not assert OTP or Lunatic compatibility

### Requirement: Lifecycle trace events
r[molten.lifecycle.trace_events] Molten MUST emit tracing events for lifecycle transitions with entity, cause, action, policy refs, transition ref, and logical step.

#### Scenario: Trace event binds cause and policy
- GIVEN a lifecycle transition admitted under policy
- WHEN Molten emits the trace event
- THEN the event binds the transition ref, cause, policy refs, action, and entity identity

### Requirement: Failed turn rollback evidence
r[molten.lifecycle.turn_failure] Molten MUST roll back pending turn actions and vat deltas on panic, denial, or validation failure and MUST emit canonical failure evidence for discarded work.

#### Scenario: Denied turn discards pending actions
- GIVEN a runtime turn with staged messages, assertions, observations, or vat deltas
- WHEN policy denial, panic, or validation failure aborts the turn
- THEN the after-rollback state matches the before state and failure evidence binds the discarded action refs, pending turn ref, policy refs, evidence refs, and any discarded vat delta refs

#### Scenario: Mutated rollback is denied
- GIVEN a failed turn receipt whose after-rollback state differs from the before state
- WHEN Molten validates the turn failure evidence
- THEN the receipt decision is denied with diagnostics instead of treating partial mutation as a successful rollback

### Requirement: Scope cleanup
r[molten.lifecycle.scope_cleanup] Molten MUST retract owned assertions, subscriptions, live references, and admitted resources when an actor, service, vat, session, handler, or job stops, crashes, loses authority, or disconnects.

#### Scenario: Stop retracts owned scope
- GIVEN an entity with owned runtime scope entries
- WHEN the entity stops, crashes, loses authority, or disconnects
- THEN cleanup evidence identifies the retracted assertions, subscriptions, live refs, and released resources

### Requirement: Idempotent cleanup
r[molten.lifecycle.cleanup_idempotent] Molten MUST make lifecycle cleanup idempotent and receipt-backed so repeated cleanup attempts do not reintroduce state or duplicate destructive side effects.

#### Scenario: Repeated cleanup is stable
- GIVEN an entity whose cleanup has already completed
- WHEN cleanup is requested again
- THEN Molten emits stable cleanup evidence and leaves runtime state unchanged

### Requirement: One-shot effect failure traces
r[molten.lifecycle.one_shot_effects] Molten MUST report irreversible one-shot effects explicitly in failure traces instead of implying that external effects were rolled back.

#### Scenario: Irreversible effect is disclosed
- GIVEN a failed turn after an irreversible external effect was requested or completed
- WHEN Molten emits failure evidence
- THEN the evidence distinguishes rolled-back local state from one-shot effects that require compensation or review

### Requirement: Links and monitors
r[molten.lifecycle.links_monitors] Molten MUST provide policy-controlled links and monitors for lifecycle failure propagation and observation.

#### Scenario: Monitor observes failure without authority escalation
- GIVEN a monitor authorized for a child entity
- WHEN the child fails
- THEN the monitor observes failure evidence without gaining child authority

### Requirement: Local supervisors
r[molten.lifecycle.supervisors] Molten MUST provide local supervisors with never, one-for-one, and bounded restart strategies.

#### Scenario: One-for-one supervisor restarts child
- GIVEN a one-for-one supervisor policy and a failed child
- WHEN restart admission passes
- THEN Molten records a supervisor decision and restarts only the failed child

### Requirement: Restart windows
r[molten.lifecycle.restart_windows] Molten MUST use logical-time restart windows and resource budgets to throttle restarts.

#### Scenario: Restart budget exhaustion denies restart
- GIVEN a child exceeding the configured restart budget within a logical-time window
- WHEN the supervisor evaluates restart
- THEN restart is denied with budget diagnostics

### Requirement: Service lifecycle assertions
r[molten.lifecycle.service_assertions] Molten MUST represent service demand, readiness, failure, dependency, exposed references, restart, and stop signals as dataspace assertions.

#### Scenario: Service readiness is a dataspace assertion
- GIVEN a service that reaches readiness
- WHEN it publishes lifecycle state
- THEN readiness is represented as a canonical dataspace assertion with lifecycle evidence refs

### Requirement: Failure rollback tests
r[molten.lifecycle.failure_tests] Molten MUST test that failed turns discard pending actions and emit failure receipts.

#### Scenario: Failed turn test observes no pending mutation
- GIVEN a test turn with staged actions
- WHEN denial or validation failure aborts the turn
- THEN the test observes unchanged state and a failure receipt binding discarded actions

### Requirement: Cleanup tests
r[molten.lifecycle.cleanup_tests] Molten MUST test that actor stop or crash retracts owned assertions and subscriptions.

#### Scenario: Cleanup test observes retractions
- GIVEN a test actor with owned assertions and subscriptions
- WHEN the actor stops or crashes
- THEN the test observes cleanup evidence and no leaked owned entries

### Requirement: Restart tests
r[molten.lifecycle.restart_tests] Molten MUST test deterministic supervisor restarts with bounded restart windows.

#### Scenario: Restart test respects window
- GIVEN a supervisor restart window fixture
- WHEN child failures are replayed deterministically
- THEN allowed and denied restarts match the configured logical-time budget

### Requirement: Lifecycle property tests
r[molten.lifecycle.property_tests] Molten MUST include Hegel property tests for cleanup idempotence, no leaked assertions, and restart bounds.

#### Scenario: Generated cleanup is idempotent
- GIVEN generated lifecycle cleanup inputs
- WHEN cleanup is applied repeatedly
- THEN final state is stable, assertions do not leak, and restart counts remain bounded
