//! Tracey markers for delivery idempotency state-machine proof coverage.
//!
//! Implementation and tests live in `src/delivery/**` and
//! `src/remote/parts/dataspace/**`; Aspen's current Cairn Tracey scanner
//! records references from `tools/**`, so this file anchors the accepted proof
//! IDs to focused delivery and replay tests.
//!
//! Verification evidence:
//! - `src/delivery/parts/idempotency/p004/body.rs`
//! - `src/remote/parts/dataspace/tests/m000/p000/body.rs`

// r[verify molten.delivery_state_machine_proof.first_commit_duplicate_suppression]
// r[verify molten.delivery_state_machine_proof.denial_no_side_effect]
// r[verify molten.delivery_state_machine_proof.replay_log_equivalence]
// r[verify molten.delivery_state_machine_proof.generated_delivery_traces]
