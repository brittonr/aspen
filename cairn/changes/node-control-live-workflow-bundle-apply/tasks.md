# Tasks: Node Control Live Workflow Bundle Apply UX

- [x] [serial] r[molten.node_control_live_workflow_bundle_apply.spec.apply_receipt] Add canonical apply receipt and `live-workflow-bundle-apply` CLI.
- [x] [serial] r[molten.node_control_live_workflow_bundle_apply.spec.gate_required] Deny missing, malformed, stale, or non-passing gate receipts when required.
- [x] [serial] r[molten.node_control_live_workflow_bundle_apply.spec.import_after_validation] Import bundle members only after bundle and gate validation pass.
- [x] [serial] r[molten.node_control_live_workflow_bundle_apply.spec.dry_run_default] Keep apply dry-run by default and avoid live Iroh sends unless `--send` is explicit.
- [x] [serial] r[molten.node_control_live_workflow_bundle_apply.spec.send_explicit] Route explicit send mode through the existing bounded live-send path and record nested send receipts.
- [x] [serial] r[molten.node_control_live_workflow_bundle_apply.spec.non_authority] Keep apply receipts outside authority/provenance gates.
- [x] [serial] r[molten.node_control_live_workflow_bundle_apply.spec.cli_tests] Add CLI coverage for apply receipt output and artifact kind recognition.
