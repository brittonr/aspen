//! Tracey markers for node live workflow state-machine proof coverage.
//!
//! Implementation and tests live in `src/node/**`; Aspen's current Cairn Tracey
//! scanner records references from `tools/**`, so this file anchors the accepted
//! proof IDs to focused node live workflow tests.
//!
//! Verification evidence:
//! - `src/node/parts/daemon/p006/body.rs` through `p017/body.rs`
//! - `src/node/parts/daemon/p025/body.rs` through `p027/body.rs`
//! - `src/node/parts/daemon/tests/m000/p002/body.rs` through `p009/body.rs`

// r[verify molten.node_live_workflow_state_proof.ordered_lifecycle]
// r[verify molten.node_live_workflow_state_proof.operation_binding]
// r[verify molten.node_live_workflow_state_proof.transport_evidence_only]
