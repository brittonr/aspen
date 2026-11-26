//! Ack, protocol, and lifecycle pass assertions.

pub(super) fn assert_ack_pass(
    case: &super::seeds::ReconcileCase,
    reconciled: &super::super::artifacts::ControlLiveWorkflowBundleReconcile,
    wrong_envelope: &str,
) -> super::seeds::AckPass {
    let delivery = &case.delivery;
    let ack_export = assert_ack_export_pass(case, reconciled);
    let import_root = super::fixtures::temp_dir("node-control-live-workflow-ack-import");
    super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &import_root,
        node_id: "node:ack-import",
    })
    .expect("init ack import root");
    let ack_import = super::super::acking::import_control_live_workflow_bundle_ack(
        &super::super::bundling::ControlLiveWorkflowBundleAckImportInput {
            state_root: &import_root,
            ack_value: &ack_export.ack.ack_value,
            expected_bundle_ref: Some(&case.exported.bundle.bundle_ref),
            expected_envelope_ref: Some(&delivery.envelope.envelope_ref),
            expected_operation_ref: Some(&delivery.envelope.operation_ref),
            expected_request_ref: Some(&delivery.delivered.request_ref),
        },
    )
    .expect("ack import");
    assert_eq!(ack_import.decision, "pass");
    assert!(ack_import.imported_refs.iter().any(|reference| reference == &ack_export.ack.ack_ref));
    assert_eq!(
        crate::ledger::artifact_kind(&ack_import.receipt_value),
        "node-control-live-workflow-bundle-ack-import-receipt"
    );
    assert!(
        crate::preserves_rail::to_text(&ack_import.receipt_value)
            .expect("ack import text")
            .contains("ack-import-is-not-authority")
    );
    super::super::storage::read_ledger_artifact(&import_root, &ack_export.ack.ack_ref).expect("ack imported");
    super::super::storage::read_ledger_artifact(&import_root, &reconciled.receipt_ref).expect("reconcile imported");
    let protocol_gate = assert_protocol_pass(case, reconciled, &ack_export.ack.ack_value);
    assert_lifecycle_pass(case, reconciled, &ack_export, &ack_import, &protocol_gate);
    super::refusals::assert_lifecycle_wrong_operation_denied(
        case,
        reconciled,
        &ack_export,
        &ack_import,
        &protocol_gate,
    );
    let wrong_ack_import = super::super::acking::import_control_live_workflow_bundle_ack(
        &super::super::bundling::ControlLiveWorkflowBundleAckImportInput {
            state_root: &import_root,
            ack_value: &ack_export.ack.ack_value,
            expected_bundle_ref: Some(&case.exported.bundle.bundle_ref),
            expected_envelope_ref: Some(wrong_envelope),
            expected_operation_ref: Some(&delivery.envelope.operation_ref),
            expected_request_ref: Some(&delivery.delivered.request_ref),
        },
    )
    .expect("wrong ack import");
    assert_eq!(wrong_ack_import.decision, "deny");
    assert!(wrong_ack_import.diagnostics.iter().any(|value| value.contains("does not match expected")));
    super::seeds::AckPass { import_root }
}

fn assert_ack_export_pass(
    case: &super::seeds::ReconcileCase,
    reconciled: &super::super::artifacts::ControlLiveWorkflowBundleReconcile,
) -> super::super::artifacts::ControlLiveWorkflowBundleAckExport {
    let delivery = &case.delivery;
    let ack_export = super::super::acking::export_control_live_workflow_bundle_ack(
        &super::super::bundling::ControlLiveWorkflowBundleAckExportInput {
            apply_receipt_value: &case.apply_receipt_value,
            send_receipt_value: None,
            ingress_receipt_value: Some(&delivery.delivered.ingress_receipt_value),
            queue_receipt_value: Some(&delivery.queue_value),
            control_receipt_value: Some(&delivery.control_value),
            reconcile_receipt_value: &reconciled.receipt_value,
        },
    )
    .expect("ack export");
    assert_eq!(ack_export.decision, "pass");
    assert_eq!(ack_export.receiver_decision, "pass");
    assert_eq!(crate::ledger::artifact_kind(&ack_export.ack.ack_value), "node-control-live-workflow-bundle-ack");
    assert_eq!(
        crate::ledger::artifact_kind(&ack_export.receipt_value),
        "node-control-live-workflow-bundle-ack-export-receipt"
    );
    assert!(super::super::issuance::parse_control_authority_grant(&ack_export.ack.ack_value).is_err());
    assert!(
        crate::preserves_rail::to_text(&ack_export.ack.ack_value)
            .expect("ack text")
            .contains("ack-bundle-is-not-authority")
    );
    ack_export
}

fn assert_protocol_pass(
    case: &super::seeds::ReconcileCase,
    reconciled: &super::super::artifacts::ControlLiveWorkflowBundleReconcile,
    ack_value: &preserves::IOValue,
) -> super::super::artifacts::ControlLiveWorkflowProtocolGate {
    let delivery = &case.delivery;
    let protocol_gate = super::super::handshake::gate_control_live_workflow_protocol(
        &super::super::bundling::ControlLiveWorkflowProtocolGateInput {
            bundle_value: &case.exported.bundle.bundle_value,
            receipt_value: &case.gated.receipt_value,
            apply_receipt_value: &case.apply_receipt_value,
            reconcile_receipt_value: &reconciled.receipt_value,
            ack_value,
            expected_envelope_ref: Some(&delivery.envelope.envelope_ref),
            expected_operation_ref: Some(&delivery.envelope.operation_ref),
            expected_request_ref: Some(&delivery.delivered.request_ref),
        },
    )
    .expect("workflow protocol gate");
    assert_eq!(protocol_gate.decision, "pass");
    assert_eq!(protocol_gate.operation_count, 6);
    assert_eq!(protocol_gate.message_count, 3);
    assert_eq!(crate::ledger::artifact_kind(&protocol_gate.receipt_value), "protocol-session-gate-receipt");
    assert!(super::super::issuance::parse_control_authority_grant(&protocol_gate.receipt_value).is_err());
    protocol_gate
}

fn assert_lifecycle_pass(
    case: &super::seeds::ReconcileCase,
    reconciled: &super::super::artifacts::ControlLiveWorkflowBundleReconcile,
    ack_export: &super::super::artifacts::ControlLiveWorkflowBundleAckExport,
    ack_import: &super::super::artifacts::ControlLiveWorkflowBundleAckImport,
    protocol_gate: &super::super::artifacts::ControlLiveWorkflowProtocolGate,
) {
    let delivery = &case.delivery;
    let gate = super::super::manifests::parse_control_live_workflow_bundle_gate_receipt(&case.gated.receipt_value)
        .expect("gate receipt");
    let apply = super::super::receipts::parse_control_live_workflow_bundle_apply_receipt(&case.apply_receipt_value)
        .expect("apply receipt");
    let reconcile =
        super::super::receipts::parse_control_live_workflow_bundle_reconcile_receipt(&reconciled.receipt_value)
            .expect("reconcile receipt");
    let lifecycle = super::super::sequencing::evaluate_live_workflow_lifecycle(
        super::super::entitlements::LiveWorkflowLifecycleInput {
            bundle: Some(&case.exported.bundle),
            gate: Some(&gate),
            apply: Some(&apply),
            reconcile: Some(&reconcile),
            ack: Some(&ack_export.ack),
            ack_import: Some(ack_import),
            protocol_gate: Some(protocol_gate),
            expected_bundle_ref: Some(&case.exported.bundle.bundle_ref),
            expected_envelope_ref: Some(&delivery.envelope.envelope_ref),
            expected_operation_ref: Some(&delivery.envelope.operation_ref),
            expected_request_ref: Some(&delivery.delivered.request_ref),
        },
    );
    assert_eq!(lifecycle.decision, "pass");
}
