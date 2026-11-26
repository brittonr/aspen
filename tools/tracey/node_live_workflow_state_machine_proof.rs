//! Tracey markers for node live workflow state-machine proof coverage.
//!
//! Implementation and tests live in `src/node/**`; Aspen's current Cairn Tracey
//! scanner records references from `tools/**`, so this file anchors the accepted
//! proof IDs to focused node live workflow tests.
//!
//! Verification evidence:
//! - `src/node/daemon/{access,workflow,protocol,encoding}/` and `src/node/daemon/records/`
//! - `src/node/daemon/listen/endpoints.rs`, `src/node/daemon/trust/`, and `src/node/daemon/preflight/delivery.rs`
//! - `src/node/daemon/tests/{reconcile,flows,probes}/`

// r[verify molten.node_live_workflow_state_proof.ordered_lifecycle]
// r[verify molten.node_live_workflow_state_proof.operation_binding]
// r[verify molten.node_live_workflow_state_proof.transport_evidence_only]
