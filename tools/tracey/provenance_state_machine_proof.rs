//! Tracey markers for provenance trust-state proof coverage.
//!
//! Implementation and tests live in `src/provenance/**` plus focused node/job/
//! remote admission checks; Aspen's current Cairn Tracey scanner records
//! references from `tools/**`, so this file anchors the accepted proof IDs.
//!
//! Implementation evidence:
//! - `src/provenance/parts/mod/p000/body.rs`
//! - `src/provenance/parts/mod/p001/body.rs`
//! - `src/provenance/parts/mod/p002/body.rs`
//! Verification evidence:
//! - `src/provenance/parts/mod/tests/m000/p000/body.rs`
//! - `src/provenance/parts/mod/tests/m000/p001/body.rs`

// r[verify molten.provenance_state_proof.profile_thresholds]
// r[verify molten.provenance_state_proof.build_verification_binding]
// r[verify molten.provenance_state_proof.evidence_only_boundary]
