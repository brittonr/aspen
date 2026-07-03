//! Tracey markers for vat promise/reference lifecycle proof coverage.
//!
//! Implementation and tests live in `src/runtime/**`; Aspen's current Cairn
//! Tracey scanner records references from `tools/**`, so this file anchors the
//! accepted proof IDs to the focused vat and runtime predicate sources.
//!
//! Implementation evidence:
//! - `src/runtime/predicates/parts/mod/p000/body.rs`
//! - `src/runtime/predicates/parts/mod/p003/body.rs`
//! - `src/runtime/predicates/parts/mod/p005/body.rs`
//! - `src/runtime/vat/parts/mod/p001/body.rs`
//! Verification evidence:
//! - `src/runtime/predicates/parts/mod/tests/m000/p000/body.rs`
//! - `src/runtime/predicates/parts/mod/tests/m000/p001/body.rs`
//! - `src/runtime/vat/parts/mod/tests/m000/p000/body.rs`

// r[verify molten.vat_ref_state_proof.promise_lifecycle]
// r[verify molten.vat_ref_state_proof.reference_lifetime]
// r[verify molten.vat_ref_state_proof.rollback_cleanup]
