//! Tracey markers for retention GC lifecycle proof coverage.
//!
//! Implementation and tests live in `src/retention/**`; Aspen's current Cairn
//! Tracey scanner records references from `tools/**`, so this file anchors the
//! accepted proof IDs to focused retention lifecycle tests.
//!
//! Verification evidence:
//! - `src/retention/parts/mod/p013/body.rs` through `p017/body.rs`
//! - `src/retention/parts/mod/tests/m000/p001/body.rs`
//! - `src/retention/parts/mod/tests/m000/p002/body.rs`

// r[verify molten.retention_gc_lifecycle_proof.ordered_chain]
// r[verify molten.retention_gc_lifecycle_proof.drift_no_mutation]
// r[verify molten.retention_gc_lifecycle_proof.execution_scope]
