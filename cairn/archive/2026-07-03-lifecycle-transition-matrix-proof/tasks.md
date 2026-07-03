# Tasks: lifecycle-transition-matrix-proof

## Phase 1: Finite relation surface

- [x] [serial] r[molten.lifecycle_state_machine_proof.transition_relation_table] Expose or centralize the finite lifecycle state list, action list, and allowed transition relation for pure tests.
- [x] [serial] r[molten.lifecycle_state_machine_proof.action_target_matrix] Expose or centralize the finite action-target compatibility relation, including supervisor-decision handling.

## Phase 2: Matrix proof tests

- [x] [parallel] r[molten.lifecycle_state_machine_proof.transition_relation_table] Add positive tests asserting every allowed `(from_state, to_state)` edge can pass with a valid action.
- [x] [parallel] r[molten.lifecycle_state_machine_proof.transition_relation_table] Add negative tests asserting every unlisted lifecycle edge denies.
- [x] [parallel] r[molten.lifecycle_state_machine_proof.action_target_matrix] Add negative tests asserting mismatched actions deny even when the state edge is otherwise allowed.

## Phase 3: Evidence and validation

- [x] [serial] r[molten.lifecycle_state_machine_proof.transition_relation_table] r[molten.lifecycle_state_machine_proof.action_target_matrix] Add traceability markers or evidence notes for lifecycle matrix proof coverage.
- [x] [serial] r[molten.lifecycle_state_machine_proof.transition_relation_table] r[molten.lifecycle_state_machine_proof.action_target_matrix] Run `cargo test lifecycle` and record the proof evidence refs or command output in the implementation notes.
