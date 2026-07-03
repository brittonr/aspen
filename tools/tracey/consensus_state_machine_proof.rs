//! Tracey markers for Raft control-registry determinism proof coverage.
//!
//! Implementation and verification live in `src/raft/control/**`; Aspen's
//! current Cairn Tracey scanner records references from `tools/**`, so this file
//! anchors the accepted proof IDs to focused Raft tests.
//!
//! Verification evidence:
//! - `src/raft/control/parts/plane/p007/body.rs`

// r[verify molten.consensus_state_machine_proof.registry_log_determinism]
// r[verify molten.consensus_state_machine_proof.duplicate_client_sequence]
// r[verify molten.consensus_state_machine_proof.snapshot_restore_equivalence]
