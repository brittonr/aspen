# Tasks: Node Control Live Workflow Bundle Import/Export

- [x] [serial] r[molten.node_control_live_workflow_bundle.spec.bundle_artifact] Add canonical bundle/export receipt records and `live-workflow-bundle-export` CLI.
- [x] [serial] r[molten.node_control_live_workflow_bundle.spec.ticket_admission_import] Validate ticket/admission expectations during bundle import.
- [x] [serial] r[molten.node_control_live_workflow_bundle.spec.authority_import] Validate authority grant operation/scope/freshness/revocation during bundle import.
- [x] [serial] r[molten.node_control_live_workflow_bundle.spec.malformed_members] Reject missing or malformed bundle members before importing evidence.
- [x] [parallel] r[molten.node_control_live_workflow_bundle.spec.sender_preflight] Import bundle member artifacts into the sender ledger so live-send preflight can resolve refs.
- [x] [parallel] r[molten.node_control_live_workflow_bundle.spec.non_authority] Keep bundle receipts outside receiver-side authority/provenance gates.
- [x] [serial] r[molten.node_control_live_workflow_bundle.spec.cli_tests] Cover bundle export/import and follow-up live-send diagnostics in CLI tests.
