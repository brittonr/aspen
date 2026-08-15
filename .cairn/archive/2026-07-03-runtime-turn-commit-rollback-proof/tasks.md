# Tasks: runtime-turn-commit-rollback-proof

## Phase 1: Turn transition law

- [x] [serial] r[molten.runtime_state_machine_proof.turn_commit_delta] Define or expose the pure law that maps a before snapshot plus pending turn to the committed after snapshot.
- [x] [serial] r[molten.runtime_state_machine_proof.turn_rollback_no_mutation] Define the denial/rollback law that leaves committed runtime state equal to the before snapshot.

## Phase 2: Positive and negative evidence

- [x] [parallel] r[molten.runtime_state_machine_proof.turn_commit_delta] Add positive tests for commits containing assertions, retractions, messages, observations, and recorded effect responses.
- [x] [parallel] r[molten.runtime_state_machine_proof.turn_rollback_no_mutation] Add negative tests for denied turns, failed turns, stale commits, and rollback paths preserving the before-state ref.
- [x] [parallel] r[molten.runtime_state_machine_proof.turn_predicate_receipts] Add tests proving predicate receipts bind before refs, turn inputs, after refs, outcome, decision, checks, and diagnostics.

## Phase 3: Generated traces

- [x] [serial] r[molten.runtime_state_machine_proof.generated_turn_traces] Add bounded Hegel turn traces that mix commit and rollback outcomes and assert invariants after every generated step.
- [x] [serial] r[molten.runtime_state_machine_proof.turn_commit_delta] r[molten.runtime_state_machine_proof.turn_rollback_no_mutation] r[molten.runtime_state_machine_proof.turn_predicate_receipts] r[molten.runtime_state_machine_proof.generated_turn_traces] Add traceability evidence and run the focused runtime tests.
