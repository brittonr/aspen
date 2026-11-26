//! Reconcile authority and denial assertions.

pub(super) fn assert_reconcile_not_authority(
    case: &super::seeds::ReconcileCase,
    reconciled: &super::super::artifacts::ControlLiveWorkflowBundleReconcile,
) {
    let refs = vec![reconciled.receipt_ref.clone()];
    let request_value = crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
        operation: "status",
        target_ref: None,
        payload_ref: None,
        authority_refs: &refs,
        policy_refs: &[],
        resource_refs: &[],
        evidence_refs: &[],
    })
    .expect("reconcile authority request");
    let envelope =
        super::super::ingresses::control_live_ingress_envelope(&super::super::requests::ControlIngressEnvelopeInput {
            request_value: &request_value,
            from_peer: "peer:reconcile",
            to_node: "node:reconcile",
            topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 2,
            peer_bootstrap_refs: &[],
            authority_refs: &refs,
            policy_refs: &[],
            resource_refs: &[],
            evidence_refs: &[],
        })
        .expect("reconcile authority envelope");
    let state_root = crate::node_state::DirectoryRoot::open(&case.delivery.root).expect("open node state root");
    let diagnostics = super::super::admissibility::live_send_authority_grant_diagnostics(&state_root, &envelope)
        .expect("reconcile authority diagnostics");
    assert!(diagnostics.iter().any(|value| value.contains("is not a grant")));
    assert!(diagnostics.iter().any(|value| value.contains("authority delegation missing admitted grant")));
}

fn assert_missing_receiver_denied(
    case: &super::seeds::ReconcileCase,
) -> super::super::artifacts::ControlLiveWorkflowBundleReconcile {
    let delivery = &case.delivery;
    let missing_receiver = super::super::reconciling::reconcile_control_live_workflow_bundle(
        &super::super::bundling::ControlLiveWorkflowBundleReconcileInput {
            apply_receipt_value: &case.apply_receipt_value,
            send_receipt_value: None,
            ingress_receipt_value: None,
            queue_receipt_value: None,
            control_receipt_value: None,
            expected_envelope_ref: Some(&delivery.envelope.envelope_ref),
            expected_operation_ref: Some(&delivery.envelope.operation_ref),
            expected_request_ref: Some(&delivery.delivered.request_ref),
        },
    )
    .expect("missing receiver reconcile");
    assert_eq!(missing_receiver.decision, "deny");
    assert!(
        missing_receiver
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("requires receiver ingress receipt"))
    );
    missing_receiver
}

pub(super) fn assert_reconcile_denials(case: &super::seeds::ReconcileCase) -> super::seeds::ReconcileDenials {
    let delivery = &case.delivery;
    let missing_receiver = assert_missing_receiver_denied(case);

    let wrong_envelope =
        super::super::storage::local_ref("node-control-envelope", "wrong-reconcile").expect("wrong envelope");
    let wrong_reconcile = super::super::reconciling::reconcile_control_live_workflow_bundle(
        &super::super::bundling::ControlLiveWorkflowBundleReconcileInput {
            apply_receipt_value: &case.apply_receipt_value,
            send_receipt_value: None,
            ingress_receipt_value: Some(&delivery.delivered.ingress_receipt_value),
            queue_receipt_value: Some(&delivery.queue_value),
            control_receipt_value: Some(&delivery.control_value),
            expected_envelope_ref: Some(&wrong_envelope),
            expected_operation_ref: Some(&delivery.envelope.operation_ref),
            expected_request_ref: Some(&delivery.delivered.request_ref),
        },
    )
    .expect("wrong envelope reconcile");
    assert_eq!(wrong_reconcile.decision, "deny");
    assert!(wrong_reconcile.diagnostics.iter().any(|diagnostic| diagnostic.contains("does not match expected")));

    let denied_control = crate::node_runtime::control_deny_receipt_value(
        &delivery.request,
        &super::super::storage::local_ref("node-startup", "reconcile-deny").expect("startup ref"),
        "receiver denial propagated",
    )
    .expect("denied control");
    let denied_reconcile = super::super::reconciling::reconcile_control_live_workflow_bundle(
        &super::super::bundling::ControlLiveWorkflowBundleReconcileInput {
            apply_receipt_value: &case.apply_receipt_value,
            send_receipt_value: None,
            ingress_receipt_value: Some(&delivery.delivered.ingress_receipt_value),
            queue_receipt_value: Some(&delivery.queue_value),
            control_receipt_value: Some(&denied_control),
            expected_envelope_ref: Some(&delivery.envelope.envelope_ref),
            expected_operation_ref: Some(&delivery.envelope.operation_ref),
            expected_request_ref: Some(&delivery.delivered.request_ref),
        },
    )
    .expect("denied reconcile");
    assert_eq!(denied_reconcile.decision, "deny");
    assert!(
        denied_reconcile
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("receiver denial propagated"))
    );

    super::seeds::ReconcileDenials {
        missing_receiver,
        denied_control,
        denied_reconcile,
        wrong_envelope,
    }
}
