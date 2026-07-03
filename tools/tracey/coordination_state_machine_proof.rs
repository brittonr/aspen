//! Tracey markers for coordination generated trace proof coverage.
//!
//! Implementation and verification live in `src/coordination/**`; Aspen's
//! current Cairn Tracey scanner records references from `tools/**`, so this file
//! anchors the accepted proof IDs to the focused coordination tests.
//!
//! Verification evidence:
//! - `src/coordination/parts/mod/tests/m000/p001/body.rs`

// r[verify molten.coordination_state_machine_proof.generated_traces]
// r[verify molten.coordination_state_machine_proof.deny_no_mutation]
// r[verify molten.coordination_state_machine_proof.duplicate_no_advance]
