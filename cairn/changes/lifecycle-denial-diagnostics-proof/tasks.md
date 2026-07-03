# Tasks: lifecycle-denial-diagnostics-proof

## Phase 1: Diagnostic laws

- [ ] [serial] r[molten.lifecycle_state_machine_proof.denial_diagnostics] Define the stable ordering and contents for lifecycle transition diagnostics.
- [ ] [serial] r[molten.lifecycle_state_machine_proof.denial_receipt_binding] Verify the receipt decision law: empty diagnostics pass, non-empty diagnostics deny.

## Phase 2: Positive and negative evidence

- [ ] [parallel] r[molten.lifecycle_state_machine_proof.denial_diagnostics] Add positive tests for valid transitions with empty diagnostics.
- [ ] [parallel] r[molten.lifecycle_state_machine_proof.denial_diagnostics] Add negative tests for invalid jumps, action-target mismatches, and combined invalid jump plus action mismatch.
- [ ] [parallel] r[molten.lifecycle_state_machine_proof.denial_receipt_binding] Add negative tests for malformed refs, empty entity ids, and empty causes failing closed before a false proof receipt can be produced.

## Phase 3: Validation

- [ ] [serial] r[molten.lifecycle_state_machine_proof.denial_diagnostics] r[molten.lifecycle_state_machine_proof.denial_receipt_binding] Add traceability evidence and run `cargo test lifecycle`.
