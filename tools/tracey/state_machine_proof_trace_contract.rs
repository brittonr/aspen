//! Tracey markers for state-machine proof trace contract coverage.
//!
//! Implementation and tests live in `src/testing/proof_trace.rs`; Aspen's
//! current Cairn Tracey scanner records references from `tools/**`, so this file
//! anchors the accepted proof IDs to focused proof-trace replay and fail-closed
//! tests.
//!
//! Verification evidence:
//! - `src/testing/proof_trace.rs`

// r[verify molten.testing.state_machine_proof.trace_contract]
// r[verify molten.testing.state_machine_proof.trace_validator]
// r[verify molten.testing.state_machine_proof.trace_validator_negative]
