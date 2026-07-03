## ADDED Requirements

### Requirement: Delivery first commit and duplicate suppression are proved
r[molten.delivery_state_machine_proof.first_commit_duplicate_suppression] Molten MUST prove that the first admitted delivery for a scoped operation commits once and that duplicate deliveries return prior receipt evidence without committing the side effect a second time.

#### Scenario: Duplicate delivery returns prior receipt
- GIVEN an admitted delivery operation that has already committed
- WHEN the same scoped operation id is delivered again
- THEN Molten returns the prior receipt ref or duplicate receipt evidence
- AND the runtime side effect is not applied again.

### Requirement: Delivery denials happen before side effects
r[molten.delivery_state_machine_proof.denial_no_side_effect] Molten MUST prove that stale, gap, conflict, malformed, or missing-evidence delivery attempts deny before runtime, adapter, protocol, service, or job side effects occur.

#### Scenario: Stale delivery preserves state
- GIVEN a delivery sequence window whose current state has advanced beyond a stale delivery
- WHEN Molten evaluates the stale delivery
- THEN the receipt decision is `deny`
- AND committed runtime state remains unchanged.

### Requirement: Delivery replay logs reproduce committed events
r[molten.delivery_state_machine_proof.replay_log_equivalence] Molten MUST prove that replayable delivery logs reproduce the same committed runtime events and state refs without live network reads, and that non-replayable or tampered logs fail closed.

#### Scenario: Replayable log matches observed events
- GIVEN a replayable delivery log with recorded idempotency receipts
- WHEN Molten replays the log from the same initial runtime state
- THEN replayed events match the recorded committed events
- AND no live network delivery is required.

### Requirement: Generated delivery traces preserve idempotency invariants
r[molten.delivery_state_machine_proof.generated_delivery_traces] Molten SHOULD include bounded generated delivery traces that mix first delivery, duplicate, retry, stale, gap, and conflict cases while asserting idempotency invariants after each step.

#### Scenario: Generated delivery trace suppresses duplicates
- GIVEN a generated bounded delivery trace with repeated operation ids
- WHEN Molten applies the trace through the idempotency decision core
- THEN duplicate operation ids do not advance committed state twice
- AND every denial carries deterministic diagnostics.
