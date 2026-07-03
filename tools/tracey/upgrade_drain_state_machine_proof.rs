//! Tracey markers for upgrade drain state-machine proof coverage.
//!
//! Implementation and tests live in `src/upgrades/**`; Aspen's current Cairn
//! Tracey scanner records references from `tools/**`, so this file anchors the
//! accepted proof IDs to focused upgrade drain tests.
//!
//! Verification evidence:
//! - `src/upgrades/parts/mod/p002/body.rs`
//! - `src/upgrades/parts/mod/p004/body.rs`
//! - `src/upgrades/parts/mod/tests/m000/p000/body.rs`
//! - `src/upgrades/parts/mod/tests/m000/p001/body.rs`

// r[verify molten.upgrade_drain_state_proof.terminal_protocol_gate]
// r[verify molten.upgrade_drain_state_proof.protocol_ref_binding]
// r[verify molten.upgrade_drain_state_proof.no_mutation_on_deny]
