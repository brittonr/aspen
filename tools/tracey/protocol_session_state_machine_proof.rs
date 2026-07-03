//! Tracey markers for protocol session state-machine proof coverage.
//!
//! Implementation and tests live in `src/protocol/**`; Aspen's current Cairn
//! Tracey scanner records references from `tools/**`, so this file anchors the
//! accepted proof IDs to focused protocol session tests.
//!
//! Verification evidence:
//! - `src/protocol/parts/session/p001/body.rs`
//! - `src/protocol/parts/session/p002/body.rs`
//! - `src/protocol/parts/session/p007/body.rs`
//! - `src/protocol/parts/session/tests/m000/p000/body.rs`
//! - `src/protocol/parts/session/tests/m000/p001/body.rs`

// r[verify molten.protocol_state_machine_proof.endpoint_transition_legality]
// r[verify molten.protocol_state_machine_proof.lifecycle_replay_completeness]
// r[verify molten.protocol_state_machine_proof.generated_session_traces]
