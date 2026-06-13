# Tasks: Node Control Live Workflow Bundle Reconcile UX

- [x] [serial] r[molten.node_control_live_workflow_bundle_reconcile.spec.reconcile_receipt] Add canonical reconcile receipt and `live-workflow-bundle-reconcile` CLI.
- [x] [serial] r[molten.node_control_live_workflow_bundle_reconcile.spec.apply_send_binding] Validate apply/send receipt binding before accepting receiver evidence.
- [x] [serial] r[molten.node_control_live_workflow_bundle_reconcile.spec.receiver_ingress_binding] Deny missing or mismatched receiver ingress receipts.
- [x] [serial] r[molten.node_control_live_workflow_bundle_reconcile.spec.queue_control_binding] Bind queue/control receipts to the same receiver request and propagate denials.
- [x] [serial] r[molten.node_control_live_workflow_bundle_reconcile.spec.non_authority] Keep reconcile receipts outside authority/provenance gates.
- [x] [serial] r[molten.node_control_live_workflow_bundle_reconcile.spec.cli_tests] Add CLI coverage for reconcile receipt output and artifact kind recognition.
- [x] [serial] r[molten.node_control_live_workflow_bundle_reconcile.spec.next_steps] Print deterministic next-step guidance for missing ingress, receiver denial, and passing control outcomes.
