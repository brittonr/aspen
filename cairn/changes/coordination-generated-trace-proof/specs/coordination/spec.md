## ADDED Requirements

### Requirement: Coordination generated traces preserve invariants
r[molten.coordination_state_machine_proof.generated_traces] Molten MUST provide bounded generated coordination traces that exercise implemented coordination primitives and check fencing monotonicity, mutual exclusion, FIFO queue behavior, semaphore bounds, barrier release thresholds, election consistency, and deterministic status assertions after each step.

#### Scenario: Generated trace preserves primitive invariants
- GIVEN a generated bounded sequence of coordination requests
- WHEN Molten applies the sequence through the coordination state machine
- THEN every accepted step preserves the primitive-specific invariant for its key
- AND emitted receipts and assertions bind the resulting state evidence.

### Requirement: Denied coordination operations do not mutate state
r[molten.coordination_state_machine_proof.deny_no_mutation] Molten MUST prove that denied coordination operations leave the coordination state ref unchanged while still emitting deterministic denial receipts.

#### Scenario: Stale token denial leaves state unchanged
- GIVEN a held coordination lock and a generated stale release token
- WHEN Molten applies the stale release request
- THEN the receipt decision is `deny`
- AND the coordination state ref after the request equals the state ref before the request.

### Requirement: Duplicate coordination operations do not advance twice
r[molten.coordination_state_machine_proof.duplicate_no_advance] Molten MUST prove inside generated traces that duplicate coordination operation ids return prior receipt evidence and do not apply the same state-machine mutation a second time.

#### Scenario: Duplicate generated operation replays receipt
- GIVEN a generated coordination operation id that has already committed
- WHEN the same operation id is generated again with the same request identity
- THEN Molten returns the prior receipt ref
- AND the state machine is not advanced again.
