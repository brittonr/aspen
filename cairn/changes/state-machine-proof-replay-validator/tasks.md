# Tasks: state-machine-proof-replay-validator

## Phase 1: Trace contract

- [ ] [serial] r[molten.testing.state_machine_proof.trace_contract] Define the minimal proof trace step fields: before-state ref, transition or command ref, after-state ref, predicate/check names, decision, diagnostics, and receipt ref.
- [ ] [serial] r[molten.testing.state_machine_proof.trace_validator] Add a pure validator core that checks proof trace adjacency and delegates schema-specific receipt validation to existing receipt validators.

## Phase 2: Positive and negative fixtures

- [ ] [parallel] r[molten.testing.state_machine_proof.trace_validator] Add a positive fixture that replays a bounded trace containing at least one passing step and one denying step.
- [ ] [parallel] r[molten.testing.state_machine_proof.trace_validator_negative] Add negative fixtures for missing receipt refs, tampered diagnostics, stale before-state refs, wrong after-state refs, and out-of-order steps.
- [ ] [parallel] r[molten.testing.state_machine_proof.trace_contract] Add deterministic summary rendering so reviewers can regenerate the same validation evidence.

## Phase 3: Validation

- [ ] [serial] r[molten.testing.state_machine_proof.trace_contract] r[molten.testing.state_machine_proof.trace_validator] r[molten.testing.state_machine_proof.trace_validator_negative] Add traceability evidence and run the focused replay/harness tests.
