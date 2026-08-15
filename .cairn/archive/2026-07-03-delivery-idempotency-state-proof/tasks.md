# Tasks: delivery-idempotency-state-proof

## Phase 1: Idempotency decision law

- [x] [serial] r[molten.delivery_state_machine_proof.first_commit_duplicate_suppression] Add or expose a pure idempotency decision law for first commit, duplicate suppression, retry, stale, gap, and conflict outcomes.
- [x] [parallel] r[molten.delivery_state_machine_proof.first_commit_duplicate_suppression] Add positive tests for first delivery commit and duplicate replay returning prior receipt refs.
- [x] [parallel] r[molten.delivery_state_machine_proof.denial_no_side_effect] Add negative tests for stale, gap, conflict, malformed operation id, and missing evidence cases denying before side effects.

## Phase 2: Replay log equivalence

- [x] [serial] r[molten.delivery_state_machine_proof.replay_log_equivalence] Add tests proving replayable delivery logs reproduce the same runtime events and state refs without live network reads.
- [x] [parallel] r[molten.delivery_state_machine_proof.replay_log_equivalence] Add negative tests for non-replayable logs, tampered entries, stale idempotency receipts, and missing prior receipt refs.

## Phase 3: Generated traces

- [x] [serial] r[molten.delivery_state_machine_proof.generated_delivery_traces] Add bounded generated delivery traces over operation ids and sequence windows.
- [x] [serial] r[molten.delivery_state_machine_proof.first_commit_duplicate_suppression] r[molten.delivery_state_machine_proof.denial_no_side_effect] r[molten.delivery_state_machine_proof.replay_log_equivalence] r[molten.delivery_state_machine_proof.generated_delivery_traces] Add traceability evidence and run focused delivery/replay tests.
