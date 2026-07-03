//! Tracey markers for release evidence workflow state-machine proof coverage.
//!
//! Implementation and tests live in `src/operator/parts/dogfood/**`; Aspen's
//! current Cairn Tracey scanner records references from `tools/**`, so this file
//! anchors the accepted proof IDs.
//!
//! Implementation evidence:
//! - `src/operator/parts/dogfood/p000/body.rs`
//! - `src/operator/parts/dogfood/p004/body.rs`
//! - `src/operator/parts/dogfood/p006/body.rs`
//! - `src/operator/parts/dogfood/p008/body.rs`
//! Verification evidence:
//! - `src/operator/parts/dogfood/tests/m000/p000/body.rs`
//! - `src/operator/parts/dogfood/tests/m000/p001/body.rs`
//! - `src/operator/parts/dogfood/tests/m000/p002/body.rs`

// r[verify molten.release_workflow_state_proof.ordered_workflow]
// r[verify molten.release_workflow_state_proof.signature_binding]
// r[verify molten.release_workflow_state_proof.evidence_only_boundary]
