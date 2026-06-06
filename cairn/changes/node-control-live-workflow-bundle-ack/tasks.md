# Tasks: Node Control Live Workflow Bundle Ack UX

- [x] [serial] r[molten.node_control_live_workflow_bundle_ack.spec.ack_artifact] Add canonical ack artifact and ack export/import receipt schemas.
- [x] [serial] r[molten.node_control_live_workflow_bundle_ack.spec.ack_export_binding] Add `live-workflow-bundle-ack-export` with reconcile recomputation and receiver evidence completeness checks.
- [x] [serial] r[molten.node_control_live_workflow_bundle_ack.spec.ack_import_binding] Add `live-workflow-bundle-ack-import` with expected bundle/envelope/operation/request guards and sender-ledger materialization.
- [x] [serial] r[molten.node_control_live_workflow_bundle_ack.spec.receiver_denial] Preserve receiver denial diagnostics without treating denial as ack package invalidity.
- [x] [serial] r[molten.node_control_live_workflow_bundle_ack.spec.non_authority] Keep ack artifacts and receipts outside authority/provenance gates.
- [x] [serial] r[molten.node_control_live_workflow_bundle_ack.spec.cli_tests] Add CLI coverage for ack export/import receipt output and artifact kind recognition.
- [x] [serial] r[molten.node_control_live_workflow_bundle_ack.spec.next_steps] Print deterministic next-step guidance for incomplete ack evidence, ack import, and receiver denial outcomes.
