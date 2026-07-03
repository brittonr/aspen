//! Tracey markers for authority and peer admission state-machine proof coverage.
//!
//! Implementation and tests live in `src/authority/**`, `src/peer/**`, and
//! `src/node/parts/daemon/**`; Aspen's current Cairn Tracey scanner records
//! references from `tools/**`, so this file anchors the accepted proof IDs to
//! focused authority and node live-admission tests.
//!
//! Verification evidence:
//! - `src/authority/parts/mod/p003/body.rs`
//! - `src/node/parts/daemon/tests/m000/p005/body.rs`
//! - `src/node/parts/daemon/tests/m000/p009/body.rs`

// r[verify molten.authority_peer_state_proof.current_scoped_grant]
// r[verify molten.authority_peer_state_proof.import_not_authority]
// r[verify molten.authority_peer_state_proof.replay_no_current_authority]
// r[verify molten.peer_admission_state_proof.ticket_scope]
// r[verify molten.peer_admission_state_proof.transport_not_bootstrap]
