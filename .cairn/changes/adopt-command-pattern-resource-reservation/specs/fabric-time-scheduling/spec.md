# Fabric time scheduling: command reservation delta

## ADDED Requirements

### Requirement: Reservations evaluate atomically against authoritative state
r[molten.reservation_command.atomic_eval] Molten MUST evaluate a resource reservation as a single command carrying the job, logical operation, requirements, and expected generation against current authoritative state, and MUST reject with preserved state when the generation is stale or capacity is insufficient.

#### Scenario: Contended reservations serialize correctly
- GIVEN two actors reserving the same bounded capacity concurrently
- WHEN both commands evaluate against authoritative state
- THEN at most one commits and the other receives a typed rejection with unchanged state.

#### Scenario: Stale generation cannot reserve
- GIVEN a command with an expected generation older than the current one
- WHEN the command evaluates
- THEN it rejects without any capacity mutation.

### Requirement: Reservation decisions stay in the pure core
r[molten.reservation_command.pure_core] Molten MUST keep capacity, worker selection, and allocation decisions in the pure transition over authoritative state, and the shell MUST only execute decided effects.

#### Scenario: No client-visible read-then-write window
- GIVEN a reservation command in flight
- WHEN another actor mutates authoritative state before the commit
- THEN the command re-evaluates against the new state rather than proceeding on the stale observation.

#### Scenario: Rejections leave no shell effects
- GIVEN a rejected reservation command
- WHEN the shell handles the outcome
- THEN no allocation, dispatch, or capacity effect executes.

### Requirement: Batches are bounded and per-item
r[molten.reservation_command.bounded_batches] Molten batch admission MUST enforce explicit item-count, byte-count, and waiting-time limits, MUST keep per-item identities and results, and MUST NOT enlarge the set of operations that commit atomically.

#### Scenario: Over-limit batches are rejected whole
- GIVEN a batch exceeding any declared limit
- WHEN admission evaluates it
- THEN the batch is rejected with a typed reason and no item commits.

#### Scenario: One item does not poison the batch
- GIVEN a batch with one invalid item and otherwise valid items
- WHEN the batch processes
- THEN each item keeps its own outcome and invalidity does not change other items' results.

### Requirement: Expiry alone does not free capacity
r[molten.reservation_command.expiry_not_release] Molten MUST treat expired-lease capacity as reusable only through a transition whose precondition includes an enforcement or termination fact, and MUST keep the capacity counted against the stale reservation otherwise.

#### Scenario: Expiry without enforcement keeps capacity counted
- GIVEN a lease that expired while its external job may still run
- WHEN no enforcement or termination fact exists
- THEN the capacity remains counted against the stale reservation and the shortfall is visible in operator status.

#### Scenario: Termination fact enables reuse
- GIVEN a lease expiry plus an admitted stop observation or fencing takeover with termination authority
- WHEN the release transition evaluates
- THEN the capacity becomes available through the bounded accounting transition.

### Requirement: Validation limits reservation evidence
r[molten.reservation_command.validation] Molten MUST retain positive and negative repository tests for the command shape, batch limits, and capacity invariants, and MUST NOT claim throughput, performance, or external-job-termination characteristics.

#### Scenario: Capacity invariants hold on every path
- GIVEN the full reservation, batch, rejection, and expiry fixture set
- WHEN normal core and scheduler tests execute them
- THEN available, reserved, and allocated quantities remain consistent after every path including rejections.

#### Scenario: No performance or termination claim follows
- GIVEN the executed command-pattern fixtures
- WHEN results are reported
- THEN they claim no throughput improvement and no ability to stop external jobs.
