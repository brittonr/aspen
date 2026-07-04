//! Tracey markers for lifecycle state-machine proof coverage.
//!
//! Implementation and tests live in `src/lifecycle/**`; Aspen's current Cairn
//! Tracey scanner records references from `tools/**`, so this file anchors the
//! accepted proof IDs to focused lifecycle transition, reachability, diagnostic,
//! and receipt-binding tests.
//!
//! Verification evidence:
//! - `src/lifecycle/parts/mod/tests/m000/p001/body.rs`

// r[verify molten.lifecycle_state_machine_proof.transition_relation_table]
// r[verify molten.lifecycle_state_machine_proof.action_target_matrix]
// r[verify molten.lifecycle_state_machine_proof.reachability]
// r[verify molten.lifecycle_state_machine_proof.terminal_cleanup]
// r[verify molten.lifecycle_state_machine_proof.denial_diagnostics]
// r[verify molten.lifecycle_state_machine_proof.denial_receipt_binding]
// r[verify molten.lifecycle_state_machine_proof.receipt_determinism]
// r[verify molten.lifecycle_state_machine_proof.receipt_evidence_binding]
