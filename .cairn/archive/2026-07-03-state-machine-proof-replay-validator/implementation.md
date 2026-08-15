# Implementation notes: state-machine-proof-replay-validator

## Evidence

- Gates: proposal, design, and tasks passed before implementation (pueue task 305).
- Baseline: `nix develop -c cargo test replay` passed before edits: 41 replay-filtered tests passed, 610 filtered (pueue task 306).
- Implemented pure proof trace contracts and `validate_proof_trace` in `src/testing/proof_trace.rs`; the validator bounds trace length/checks/diagnostics, validates canonical refs, checks adjacency/final-state binding, and delegates lifecycle transition receipt validation to `lifecycle::validate_transition_receipt`.
- Exposed the module as `state_machine_proof` in `src/lib.rs`.
- Added positive and negative proof trace fixtures with `r[verify molten.testing.state_machine_proof.trace_contract]`, `r[verify molten.testing.state_machine_proof.trace_validator]`, and `r[verify molten.testing.state_machine_proof.trace_validator_negative]` markers.
- Validation: `nix develop -c cargo fmt && nix develop -c cargo test proof_trace && nix develop -c cargo test replay` passed after edits (pueue task 324). The replay filter reported 46 passing tests after adding the proof trace fixtures.
