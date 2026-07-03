# Tasks: lifecycle-receipt-determinism-proof

## Phase 1: Determinism law

- [ ] [serial] r[molten.lifecycle_state_machine_proof.receipt_determinism] Add tests proving repeated construction of the same lifecycle transition record and receipt yields identical refs and values.
- [ ] [parallel] r[molten.lifecycle_state_machine_proof.receipt_determinism] Add negative drift tests proving changes to state, action, cause, refs, supervisor ref, or logical step change transition or receipt evidence.

## Phase 2: Evidence binding

- [ ] [serial] r[molten.lifecycle_state_machine_proof.receipt_evidence_binding] Add or strengthen a pure receipt validator/parser that verifies receipt refs, transition refs, decisions, diagnostics, and checks from in-memory values.
- [ ] [parallel] r[molten.lifecycle_state_machine_proof.receipt_evidence_binding] Add negative tests for tampered transition refs, tampered decisions, dropped diagnostics, and mismatched checks.

## Phase 3: Validation

- [ ] [serial] r[molten.lifecycle_state_machine_proof.receipt_determinism] r[molten.lifecycle_state_machine_proof.receipt_evidence_binding] Add traceability evidence and run `cargo test lifecycle`.
