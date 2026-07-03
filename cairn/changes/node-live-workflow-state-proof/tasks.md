# Tasks: node-live-workflow-state-proof

## Phase 1: Pure workflow validator

- [ ] [serial] r[molten.node_live_workflow_state_proof.ordered_lifecycle] Extract or define pure live workflow lifecycle validation over bundle, gate, apply, reconcile, ack, import, and protocol gate evidence.
- [ ] [parallel] r[molten.node_live_workflow_state_proof.operation_binding] Add exact expected-ref checks for request, operation, envelope, bundle, apply, reconcile, and ack bindings.
- [ ] [parallel] r[molten.node_live_workflow_state_proof.transport_evidence_only] Make transport-only evidence rejection explicit in diagnostics.

## Phase 2: Proof fixtures and tests

- [ ] [parallel] r[molten.node_live_workflow_state_proof.ordered_lifecycle] Add a positive complete live workflow trace test.
- [ ] [parallel] r[molten.node_live_workflow_state_proof.ordered_lifecycle] r[molten.node_live_workflow_state_proof.operation_binding] Add negative tests for out-of-order apply, failed child receipts, wrong operation, stale request, and mismatched ack.
- [ ] [parallel] r[molten.node_live_workflow_state_proof.transport_evidence_only] Add negative tests proving live send/listener/neighbor receipts cannot replace peer admission or authority grants.

## Phase 3: Evidence and validation

- [ ] [serial] r[molten.node_live_workflow_state_proof.ordered_lifecycle] r[molten.node_live_workflow_state_proof.operation_binding] r[molten.node_live_workflow_state_proof.transport_evidence_only] Bind proof trace refs and run `cargo test node`.
