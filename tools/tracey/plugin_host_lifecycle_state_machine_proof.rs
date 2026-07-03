//! Tracey markers for plugin host lifecycle state-machine proof coverage.
//!
//! Implementation and tests live in `src/plugin/**`; Aspen's current Cairn
//! Tracey scanner records references from `tools/**`, so this file anchors the
//! accepted proof IDs to focused plugin lifecycle tests.
//!
//! Verification evidence:
//! - `src/plugin/parts/host/p000/body.rs`
//! - `src/plugin/parts/host/p003/body.rs`
//! - `src/plugin/parts/host/p004/body.rs`
//! - `src/plugin/parts/host/p006/body.rs`

// r[verify molten.plugin_lifecycle_state_proof.ordered_lifecycle]
// r[verify molten.plugin_lifecycle_state_proof.health_gate]
// r[verify molten.plugin_lifecycle_state_proof.cleanup_closes_authority]
