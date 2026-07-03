//! Tracey markers for service supervision state-machine proof coverage.
//!
//! Implementation and tests live in `src/lifecycle/**`; Aspen's current Cairn
//! Tracey scanner records references from `tools/**`, so this file anchors the
//! accepted proof IDs to focused service supervision tests.
//!
//! Verification evidence:
//! - `src/lifecycle/parts/mod/p000/body.rs`
//! - `src/lifecycle/parts/mod/p001/body.rs`
//! - `src/lifecycle/parts/mod/tests/m000/p000/body.rs`

// r[verify molten.service_state_machine_proof.dependency_wait_no_start]
// r[verify molten.service_state_machine_proof.bounded_restart_trace]
// r[verify molten.service_state_machine_proof.cleanup_idempotence]
