//! Tracey markers for the runtime turn commit/rollback proof.
//!
//! The implementation and verification code lives in `src/runtime/**`; Aspen's
//! current Cairn Tracey scanner records references from `tools/**`, so this file
//! anchors those proof markers to the focused runtime sources.
//!
//! Implementation evidence:
//! - `src/runtime/predicates/parts/mod/p006/body.rs`
//! - `src/runtime/dataspace/state.rs`
//! Verification evidence:
//! - `src/runtime/predicates/parts/mod/tests/m000/p000/body.rs`
//! - `src/runtime/predicates/parts/mod/tests/m000/p002/body.rs`
//! - `src/runtime/dataspace/tests.rs`

// r[impl molten.runtime_state_machine_proof.turn_commit_delta]
// r[impl molten.runtime_state_machine_proof.turn_rollback_no_mutation]
// r[impl molten.runtime_state_machine_proof.turn_predicate_receipts]
// r[verify molten.runtime_state_machine_proof.turn_commit_delta]
// r[verify molten.runtime_state_machine_proof.turn_rollback_no_mutation]
// r[verify molten.runtime_state_machine_proof.turn_predicate_receipts]
// r[verify molten.runtime_state_machine_proof.generated_turn_traces]
