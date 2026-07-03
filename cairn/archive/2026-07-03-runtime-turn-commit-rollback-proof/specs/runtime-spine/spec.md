## ADDED Requirements

### Requirement: Runtime committed turn delta is exact
r[molten.runtime_state_machine_proof.turn_commit_delta] Molten MUST prove that a committed runtime turn publishes exactly the predicate-approved state delta for assertions, retractions, messages, observations, and recorded effect responses.

#### Scenario: Committed turn matches computed delta
- GIVEN a runtime snapshot and a pending turn with bounded actions
- WHEN Molten commits the turn through the runtime transition predicate
- THEN the after-state ref matches the pure transition result
- AND no unrecorded pending action becomes visible.

### Requirement: Runtime rollback leaves committed state unchanged
r[molten.runtime_state_machine_proof.turn_rollback_no_mutation] Molten MUST prove that denied, failed, or rolled-back runtime turns leave committed runtime state equal to the before snapshot.

#### Scenario: Denied turn preserves before snapshot
- GIVEN a pending runtime turn that is denied before commit
- WHEN Molten rolls the turn back
- THEN the resulting state ref equals the before-state ref
- AND pending assertions, retractions, messages, and effect intents are not committed.

### Requirement: Runtime turn predicate receipts bind transition evidence
r[molten.runtime_state_machine_proof.turn_predicate_receipts] Molten MUST bind runtime turn predicate receipts to before-state refs, turn inputs, after-state refs, outcomes, decisions, checks, and diagnostics.

#### Scenario: Stale commit receipt denies
- GIVEN a runtime turn receipt whose committed outcome does not match the after snapshot
- WHEN Molten validates the turn transition predicate
- THEN the receipt decision is `deny`
- AND diagnostics identify the transition mismatch.

### Requirement: Generated runtime turn traces preserve invariants
r[molten.runtime_state_machine_proof.generated_turn_traces] Molten SHOULD include bounded generated runtime turn traces that mix commit and rollback outcomes and assert the commit delta and rollback no-mutation laws after every step.

#### Scenario: Generated mixed trace stays deterministic
- GIVEN a generated bounded sequence of runtime turns
- WHEN the sequence is replayed from the same initial snapshot
- THEN the same committed state refs and predicate receipt refs are produced
- AND every denied turn leaves state unchanged.
