//! Lifecycle wrong-operation and ack denial assertions.

pub(super) fn assert_lifecycle_wrong_operation_denied(
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
    let mut wrong_ack = ack_export.ack.clone();
    wrong_ack.operation_ref =
        Some(super::super::storage::local_ref("node-control-live-workflow-wrong-operation", "ack").expect("ref"));
    let lifecycle = super::super::sequencing::evaluate_live_workflow_lifecycle(
        super::super::entitlements::LiveWorkflowLifecycleInput {
            bundle: Some(&case.exported.bundle),
            gate: Some(&gate),
            apply: Some(&apply),
            reconcile: Some(&reconcile),
            ack: Some(&wrong_ack),
            ack_import: Some(ack_import),
            protocol_gate: Some(protocol_gate),
            expected_bundle_ref: Some(&case.exported.bundle.bundle_ref),
            expected_envelope_ref: Some(&delivery.envelope.envelope_ref),
            expected_operation_ref: Some(&delivery.envelope.operation_ref),
            expected_request_ref: Some(&delivery.delivered.request_ref),
        },
    );
    assert_eq!(lifecycle.decision, "deny");
    assert!(
        lifecycle
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic == "node-control-live-workflow-ack-operation-mismatch")
    );
    assert!(lifecycle.diagnostics.iter().any(|diagnostic| diagnostic.contains("expected-operation-mismatch")));
}

fn assert_missing_ack_export_denied(case: &super::seeds::ReconcileCase, denials: &super::seeds::ReconcileDenials) {
    let missing_ack_export = super::super::acking::export_control_live_workflow_bundle_ack(
        &super::super::bundling::ControlLiveWorkflowBundleAckExportInput {
            apply_receipt_value: &case.apply_receipt_value,
            send_receipt_value: None,
            ingress_receipt_value: None,
            queue_receipt_value: None,
            control_receipt_value: None,
            reconcile_receipt_value: &denials.missing_receiver.receipt_value,
        },
    )
    .expect("missing ack export");
    assert_eq!(missing_ack_export.decision, "deny");
    assert!(
        missing_ack_export
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("requires receiver ingress receipt"))
    );
}

pub(super) fn assert_ack_denials(
    case: &super::seeds::ReconcileCase,
    denials: &super::seeds::ReconcileDenials,
    ack: &super::seeds::AckPass,
) {
    let delivery = &case.delivery;
    assert_missing_ack_export_denied(case, denials);

    let denied_ack_export = super::super::acking::export_control_live_workflow_bundle_ack(
        &super::super::bundling::ControlLiveWorkflowBundleAckExportInput {
            apply_receipt_value: &case.apply_receipt_value,
            send_receipt_value: None,
            ingress_receipt_value: Some(&delivery.delivered.ingress_receipt_value),
            queue_receipt_value: Some(&delivery.queue_value),
            control_receipt_value: Some(&denials.denied_control),
            reconcile_receipt_value: &denials.denied_reconcile.receipt_value,
        },
    )
    .expect("denied ack export");
    assert_eq!(denied_ack_export.decision, "pass");
    assert_eq!(denied_ack_export.receiver_decision, "deny");
    assert!(
        denied_ack_export
            .ack
            .receiver_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("receiver denial propagated"))
    );
    let denied_ack_import = super::super::acking::import_control_live_workflow_bundle_ack(
        &super::super::bundling::ControlLiveWorkflowBundleAckImportInput {
            state_root: &ack.import_root,
            ack_value: &denied_ack_export.ack.ack_value,
            expected_bundle_ref: Some(&case.exported.bundle.bundle_ref),
            expected_envelope_ref: Some(&delivery.envelope.envelope_ref),
            expected_operation_ref: Some(&delivery.envelope.operation_ref),
            expected_request_ref: Some(&delivery.delivered.request_ref),
        },
    )
    .expect("denied ack import");
    assert_eq!(denied_ack_import.decision, "pass");
    assert_eq!(denied_ack_import.receiver_decision, "deny");
    let denied_protocol_gate = super::super::handshake::gate_control_live_workflow_protocol(
        &super::super::bundling::ControlLiveWorkflowProtocolGateInput {
            bundle_value: &case.exported.bundle.bundle_value,
            receipt_value: &case.gated.receipt_value,
            apply_receipt_value: &case.apply_receipt_value,
            reconcile_receipt_value: &denials.denied_reconcile.receipt_value,
            ack_value: &denied_ack_export.ack.ack_value,
            expected_envelope_ref: Some(&delivery.envelope.envelope_ref),
            expected_operation_ref: Some(&delivery.envelope.operation_ref),
            expected_request_ref: Some(&delivery.delivered.request_ref),
        },
    )
    .expect("denied workflow protocol gate");
    assert_eq!(denied_protocol_gate.decision, "deny");
    assert!(
        denied_protocol_gate
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("ack receiver decision deny"))
    );
}
