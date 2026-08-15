# Tasks: protocol-session-transition-proof

## Phase 1: Endpoint transition law

- [x] [serial] r[molten.protocol_state_machine_proof.endpoint_transition_legality] Define or expose pure transition checks for projected endpoint send, receive, branch, and offer operations.
- [x] [parallel] r[molten.protocol_state_machine_proof.endpoint_transition_legality] Add positive tests for legal projected endpoint operation transitions.
- [x] [parallel] r[molten.protocol_state_machine_proof.endpoint_transition_legality] Add negative tests for wrong role, peer, label, prior state, next state, and ambiguous branch evidence.

## Phase 2: Lifecycle gate replay

- [x] [serial] r[molten.protocol_state_machine_proof.lifecycle_replay_completeness] Add tests proving protocol lifecycle gates replay install and operation receipts into terminal states.
- [x] [parallel] r[molten.protocol_state_machine_proof.lifecycle_replay_completeness] Add negative tests for missing terminal evidence, missing message evidence, stale operation refs, and out-of-order operation evidence.

## Phase 3: Generated protocol traces

- [x] [serial] r[molten.protocol_state_machine_proof.generated_session_traces] Add bounded generated or fixture-derived session traces covering linear send/receive and branch/offer paths.
- [x] [serial] r[molten.protocol_state_machine_proof.endpoint_transition_legality] r[molten.protocol_state_machine_proof.lifecycle_replay_completeness] r[molten.protocol_state_machine_proof.generated_session_traces] Add traceability evidence and run `cargo test protocol`.
