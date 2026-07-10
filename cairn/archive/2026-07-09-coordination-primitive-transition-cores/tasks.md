# Tasks: coordination-primitive-transition-cores

- [x] [serial] r[molten.coordination_state_machine_proof.primitive_transition_cores] Define pure transition-result types for coordination primitives with decision, next or preserved state, token/output facts, status assertion facts, diagnostics, checks, and shell intents.
- [x] [serial] r[molten.coordination_state_machine_proof.primitive_transition_cores] Refactor lock, queue, semaphore, rate-limit, election, barrier, and registry mutation preparation into primitive transition cores.
- [x] [parallel] r[molten.coordination_state_machine_proof.replay_transition_kind] Model operation-id duplicate replay and conflicting duplicate denial as explicit no-advance transition results.
- [x] [parallel] r[molten.coordination_state_machine_proof.transition_receipt_binding] Bind coordination receipts and status assertions to transition-kind, before-state, after-state or preserved-state, token/output facts, decision, diagnostics, and control-plane intent refs.
- [x] [parallel] r[molten.coordination_state_machine_proof.transition_matrix_tests] Extend generated traces to cover pass, deny, duplicate replay, and conflicting duplicate paths for each primitive.
- [x] [serial] r[molten.coordination_state_machine_proof.transition_matrix_tests] Run focused coordination tests and Cairn validation, then record evidence in implementation notes.

## Implementation Notes

- Added `PrimitiveTransitionResult` and transition-bound coordination receipts for advance, deny-preserve, duplicate-replay, conflicting-duplicate-deny, and read-observe outcomes.
- Added exact duplicate replay receipts that preserve state and point at the prior receipt/output refs, plus conflicting duplicate denial without replacing the original operation-id record.
- Verification evidence: `nix develop -c cargo fmt --check`; `nix develop -c cargo test coordination -- --nocapture`; `nix develop -c cargo test duplicate -- --nocapture`; `nix develop -c cargo test hegel_generated_coordination_trace_preserves_state_machine_invariants -- --nocapture`; `nix develop -c cargo check --workspace --all-targets`.