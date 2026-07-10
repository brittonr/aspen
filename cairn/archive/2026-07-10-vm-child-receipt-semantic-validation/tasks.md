# Tasks: vm-child-receipt-semantic-validation

## Phase 1: Child parser core

- [x] [parallel] r[molten.testing.vm_evidence.child_receipt_semantic_validation] Add pure parsers/classifiers for live-control, transport, ingress, queue, dispatch, reconcile, ack, protocol, service/job, coordination, soak, and fault validation child receipts.
- [x] [parallel] r[molten.testing.vm_evidence.expected_child_ref_gate] Extend validation input with explicit expected child refs and expected workflow bindings.

## Phase 2: Validation logic

- [x] [serial] r[molten.testing.vm_evidence.child_receipt_semantic_validation] Check child receipt topology, node, peer, operation, decision, and receipt-class fields against expected bindings.
- [x] [serial] r[molten.testing.vm_evidence.expected_child_ref_gate] Deny missing, duplicate, stale, undeclared, or log-only child refs before accepting VM pass evidence.

## Phase 3: Coverage

- [x] [parallel] r[molten.testing.vm_evidence.child_receipt_semantic_validation] Add positive fixtures for a complete live-control child chain and service/job/coordination child evidence.
- [x] [parallel] r[molten.testing.vm_evidence.expected_child_ref_gate] Add negative fixtures for wrong node, wrong peer, wrong operation, wrong class, denied child, missing expected ref, duplicate ref, and log-only child.
- [x] [serial] r[molten.testing.vm_evidence.child_receipt_semantic_validation] Run focused VM validation tests and update traceability coverage.
