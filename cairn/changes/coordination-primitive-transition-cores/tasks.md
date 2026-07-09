# Tasks: coordination-primitive-transition-cores

- [ ] [serial] r[molten.coordination_state_machine_proof.primitive_transition_cores] Define pure transition-result types for coordination primitives with decision, next or preserved state, token/output facts, status assertion facts, diagnostics, checks, and shell intents.
- [ ] [serial] r[molten.coordination_state_machine_proof.primitive_transition_cores] Refactor lock, queue, semaphore, rate-limit, election, barrier, and registry mutation preparation into primitive transition cores.
- [ ] [parallel] r[molten.coordination_state_machine_proof.replay_transition_kind] Model operation-id duplicate replay and conflicting duplicate denial as explicit no-advance transition results.
- [ ] [parallel] r[molten.coordination_state_machine_proof.transition_receipt_binding] Bind coordination receipts and status assertions to transition-kind, before-state, after-state or preserved-state, token/output facts, decision, diagnostics, and control-plane intent refs.
- [ ] [parallel] r[molten.coordination_state_machine_proof.transition_matrix_tests] Extend generated traces to cover pass, deny, duplicate replay, and conflicting duplicate paths for each primitive.
- [ ] [serial] r[molten.coordination_state_machine_proof.transition_matrix_tests] Run focused coordination tests and Cairn validation, then record evidence in implementation notes.