//! Reconcile export, gate, apply, and pass assertions.

fn export_reconcile_bundle(
    delivery: &super::seeds::ReconcileDelivery,
) -> super::super::artifacts::ControlLiveWorkflowBundleExport {
    let ticket =
        super::super::issuance::export_control_live_ticket(&super::super::offers::ControlLiveTicketExportInput {
            state_root: &delivery.root,
            topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
            policy_refs: &delivery.policy_refs,
            evidence_refs: &[],
        })
        .expect("export reconcile ticket");
    let admission = super::super::peering::admit_control_live_peer(&super::super::offers::ControlLivePeerAdmitInput {
        state_root: &delivery.root,
        ticket_value: &ticket.value,
        peer_id: "peer:reconcile",
        sequence: 1,
        expires_at: None,
        policy_refs: &delivery.policy_refs,
        evidence_refs: &[],
    })
    .expect("admit reconcile peer");
    let authority_value =
        super::super::issuance::control_authority_grant_value(&super::super::offers::ControlAuthorityGrantInput {
            peer_id: "peer:reconcile",
            node_id: "node:reconcile",
            operations: &delivery.operations,
            target_scope: "*",
            resource_scope: "*",
            epoch: 1,
            expires_at: None,
            policy_refs: &delivery.policy_refs,
            revocation_refs: &[],
            evidence_refs: &[],
        })
        .expect("reconcile authority value");
    let receipt_values: Vec<&preserves::IOValue> = Vec::new();
    let exported = super::super::exporting::export_control_live_workflow_bundle(
        &super::super::bundling::ControlLiveWorkflowBundleExportInput {
            receiver_ticket_value: &ticket.value,
            peer_admission_value: &admission.value,
            authority_grant_value: &authority_value,
            receipt_values: &receipt_values,
        },
    )
    .expect("export reconcile workflow bundle");
    assert_eq!(exported.decision, "pass");
    exported
}

fn gate_reconcile_bundle(
    exported: &super::super::artifacts::ControlLiveWorkflowBundleExport,
    expected: &super::super::intake::LiveWorkflowBundleExpectedInput<'_>,
) -> (
    super::super::artifacts::ControlLiveWorkflowBundleVerify,
    super::super::artifacts::ControlLiveWorkflowBundleGate,
) {
    let verified = super::super::verifying::verify_control_live_workflow_bundle(
        &super::super::bundling::ControlLiveWorkflowBundleVerifyInput {
            bundle_value: &exported.bundle.bundle_value,
            expected_node: expected.expected_node,
            expected_topic: expected.expected_topic,
            expected_endpoint: expected.expected_endpoint,
            expected_peer: expected.expected_peer,
            expected_operations: expected.expected_operations,
            expected_target_scope: expected.expected_target_scope,
            expected_resource_scope: expected.expected_resource_scope,
            as_of_sequence: expected.as_of_sequence,
            as_of_epoch: expected.as_of_epoch,
        },
    )
    .expect("verify reconcile workflow bundle");
    assert_eq!(verified.decision, "pass");
    let gated = super::super::verifying::gate_control_live_workflow_bundle(
        &super::super::bundling::ControlLiveWorkflowBundleGateInput {
            bundle_value: &exported.bundle.bundle_value,
            verify_receipt_value: Some(&verified.receipt_value),
            require_verify_receipt: true,
            expected_node: expected.expected_node,
            expected_topic: expected.expected_topic,
            expected_endpoint: expected.expected_endpoint,
            expected_peer: expected.expected_peer,
            expected_operations: expected.expected_operations,
            expected_target_scope: expected.expected_target_scope,
            expected_resource_scope: expected.expected_resource_scope,
            as_of_sequence: expected.as_of_sequence,
            as_of_epoch: expected.as_of_epoch,
        },
    )
    .expect("gate reconcile workflow bundle");
    assert_eq!(gated.decision, "pass");
    (verified, gated)
}

fn apply_reconcile_value(
    delivery: &super::seeds::ReconcileDelivery,
    exported: &super::super::artifacts::ControlLiveWorkflowBundleExport,
    verified: &super::super::artifacts::ControlLiveWorkflowBundleVerify,
    gated: &super::super::artifacts::ControlLiveWorkflowBundleGate,
    expected: &super::super::intake::LiveWorkflowBundleExpectedInput<'_>,
) -> preserves::IOValue {
    let imported_refs = Vec::new();
    let diagnostics = Vec::new();
    super::super::gates::live_workflow_bundle_apply_receipt_value(
        &super::super::gates::LiveWorkflowBundleApplyReceiptValueInput {
            decision: "pass",
            bundle_ref: &exported.bundle.bundle_ref,
            gate_receipt_ref: Some(&gated.receipt_ref),
            recomputed_verify_receipt_ref: &verified.receipt_ref,
            import_receipt_ref: None,
            imported_refs: &imported_refs,
            mode: "dry-run",
            envelope_ref: Some(&delivery.envelope.envelope_ref),
            operation_ref: Some(&delivery.envelope.operation_ref),
            send_receipt_ref: None,
            topology_profile_ref: None,
            transport_profile_ref: None,
            effective_max_attempts: super::super::settings::DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS,
            effective_join_timeout_ms: super::super::settings::MAX_CONTROL_LIVE_SEND_TIMEOUT_MS,
            expected,
            diagnostics: &diagnostics,
        },
    )
    .expect("apply receipt")
}

pub(super) fn reconcile_case() -> super::seeds::ReconcileCase {
    let delivery = super::seeds::reconcile_delivery();
    let expected = super::seeds::reconcile_expected(&delivery.operations);
    let exported = export_reconcile_bundle(&delivery);
    let (verified, gated) = gate_reconcile_bundle(&exported, &expected);
    let apply_receipt_value = apply_reconcile_value(&delivery, &exported, &verified, &gated, &expected);
    super::seeds::ReconcileCase {
        delivery,
        exported,
        gated,
        apply_receipt_value,
    }
}

pub(super) fn assert_reconcile_pass(
    case: &super::seeds::ReconcileCase,
) -> super::super::artifacts::ControlLiveWorkflowBundleReconcile {
    let delivery = &case.delivery;
    let reconciled = super::super::reconciling::reconcile_control_live_workflow_bundle(
        &super::super::bundling::ControlLiveWorkflowBundleReconcileInput {
            apply_receipt_value: &case.apply_receipt_value,
            send_receipt_value: None,
            ingress_receipt_value: Some(&delivery.delivered.ingress_receipt_value),
            queue_receipt_value: Some(&delivery.queue_value),
            control_receipt_value: Some(&delivery.control_value),
            expected_envelope_ref: Some(&delivery.envelope.envelope_ref),
            expected_operation_ref: Some(&delivery.envelope.operation_ref),
            expected_request_ref: Some(&delivery.delivered.request_ref),
        },
    )
    .expect("reconcile");
    assert_eq!(reconciled.decision, "pass");
    assert_eq!(
        crate::ledger::artifact_kind(&reconciled.receipt_value),
        "node-control-live-workflow-bundle-reconcile-receipt"
    );
    assert_eq!(reconciled.ingress_receipt_ref.as_deref(), Some(delivery.delivered.ingress_receipt_ref.as_str()));
    assert_eq!(reconciled.control_receipt_ref.as_deref(), Some(delivery.control_receipt_ref.as_str()));
    assert!(super::super::issuance::parse_control_authority_grant(&reconciled.receipt_value).is_err());
    assert!(
        crate::preserves_rail::to_text(&reconciled.receipt_value)
            .expect("reconcile text")
            .contains("reconcile-receipt-is-not-authority")
    );
    super::super::storage::import_artifact(&delivery.root, &reconciled.receipt_value)
        .expect("import reconcile receipt");
    super::rejections::assert_reconcile_not_authority(case, &reconciled);
    reconciled
}
