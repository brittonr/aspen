# Coordination Delta: Primitive Transition Cores

### Requirement: Coordination primitives expose pure transition cores
r[molten.coordination_state_machine_proof.primitive_transition_cores] Molten MUST expose coordination lock, queue, semaphore, rate-limit, election, barrier, and registry semantics through pure primitive transition cores that consume current state, manifest limits, request facts, replay/idempotency facts, and admission facts, and return transition results without mutating runtime state or performing control-plane, filesystem, network, ledger, or dataspace effects.

#### Scenario: Lock acquire transition returns candidate state
- GIVEN a lock is free, the request is admitted, and the manifest permits lock acquisition
- WHEN the lock acquire transition core evaluates the request
- THEN it returns a pass decision with a candidate next state, fencing token fact, status assertion fact, and receipt checks
- AND no runtime state is mutated until the shell commits the transition.

#### Scenario: Queue overflow transition preserves state
- GIVEN a queue is already at manifest capacity
- WHEN the queue enqueue transition core evaluates another enqueue request
- THEN it returns a deny decision with diagnostics
- AND the preserved-state ref matches the input state ref.

### Requirement: Coordination duplicate replay is an explicit transition kind
r[molten.coordination_state_machine_proof.replay_transition_kind] Coordination operation-id replay MUST be represented as an explicit no-advance transition kind that returns prior receipt or output refs for exact duplicates, denies conflicting duplicates, and preserves the current coordination state in both cases.

#### Scenario: Exact duplicate acquire does not advance token
- GIVEN a lock acquire operation id has already committed with a fencing token
- WHEN the same request is evaluated again
- THEN the transition kind is duplicate replay
- AND the next fencing token and lock state are not advanced.

#### Scenario: Conflicting duplicate denies without mutation
- GIVEN an operation id has already committed for one coordination request
- WHEN a different request reuses that operation id
- THEN Molten emits a deny transition
- AND no primitive state, token counter, queue contents, or registry entry changes.

### Requirement: Coordination transition receipts bind state movement
r[molten.coordination_state_machine_proof.transition_receipt_binding] Coordination receipts and status assertions MUST bind the primitive transition kind, service, operation, key, request ref, before-state ref, after-state ref or preserved-state ref, token or output facts when present, control-plane intent or commit refs when present, decision, diagnostics, and checks.

#### Scenario: Denial receipt proves no mutation
- GIVEN a stale fencing-token release is denied
- WHEN the coordination receipt is emitted
- THEN the receipt binds the stale-token diagnostic and preserved-state ref
- AND the held lock state remains unchanged.

### Requirement: Coordination generated traces cover transition matrix
r[molten.coordination_state_machine_proof.transition_matrix_tests] Molten SHOULD extend bounded generated coordination traces to cover pass, denial, exact duplicate replay, conflicting duplicate denial, and preserved-state assertions for locks, queues, semaphores, rate limits, elections, barriers, and registry entries.

#### Scenario: Generated matrix covers each primitive denial
- GIVEN the generated coordination trace suite runs
- WHEN each supported primitive receives at least one invalid or over-limit event
- THEN each invalid event emits deny evidence
- AND every denial preserves the prior state ref.