
fn live_workflow_bundle_apply_receipt_value(input: &LiveWorkflowBundleApplyReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    let apply_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(crate::preserves_rail::record("node-control-live-workflow-bundle-apply-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_APPLY_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("state-root", vec![crate::preserves_rail::string(
            input.state_root.display().to_string(),
        )]),
        crate::preserves_rail::record("bundle", vec![crate::preserves_rail::string(input.bundle_ref)]),
        crate::preserves_rail::record("gate-receipt", vec![optional_string(input.gate_receipt_ref)]),
        crate::preserves_rail::record("recomputed-verify", vec![crate::preserves_rail::string(
            input.recomputed_verify_receipt_ref,
        )]),
        crate::preserves_rail::record("import-receipt", vec![optional_string(input.import_receipt_ref)]),
        crate::preserves_rail::record("imported", vec![crate::preserves_rail::sequence(
            input.imported_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("mode", vec![crate::preserves_rail::string(input.mode)]),
        crate::preserves_rail::record("envelope", vec![optional_string(input.envelope_ref)]),
        crate::preserves_rail::record("operation", vec![optional_string(input.operation_ref)]),
        crate::preserves_rail::record("send-receipt", vec![optional_string(input.send_receipt_ref)]),
        crate::preserves_rail::record("expected", vec![live_workflow_bundle_expected_value(input.expected)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-verification"),
                crate::preserves_rail::string(apply_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("gate-receipt-current"),
                crate::preserves_rail::string(apply_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-imported"),
                crate::preserves_rail::string(apply_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("send-preflight-or-dispatch"),
                crate::preserves_rail::string(apply_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("apply-receipt-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("provenance-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
        live_profile_ref_records(input.topology_profile_ref, input.transport_profile_ref),
        live_effective_transport_record(input.effective_max_attempts, input.effective_join_timeout_ms),
    ]))
}

fn live_workflow_bundle_reconcile_receipt_value(
    input: &LiveWorkflowBundleReconcileReceiptValueInput<'_>,
) -> Result<IoValue> {
    validate_decision(input.decision)?;
    let reconcile_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(crate::preserves_rail::record("node-control-live-workflow-bundle-reconcile-receipt-v1", vec![
        crate::preserves_rail::string(
            crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_RECONCILE_RECEIPT_SCHEMA,
        ),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("apply-receipt", vec![crate::preserves_rail::string(input.apply_receipt_ref)]),
        crate::preserves_rail::record("bundle", vec![crate::preserves_rail::string(input.bundle_ref)]),
        crate::preserves_rail::record("send-receipt", vec![optional_string(input.send_receipt_ref)]),
        crate::preserves_rail::record("ingress-receipt", vec![optional_string(input.ingress_receipt_ref)]),
        crate::preserves_rail::record("queue-receipt", vec![optional_string(input.queue_receipt_ref)]),
        crate::preserves_rail::record("control-receipt", vec![optional_string(input.control_receipt_ref)]),
        crate::preserves_rail::record("envelope", vec![optional_string(input.envelope_ref)]),
        crate::preserves_rail::record("operation", vec![optional_string(input.operation_ref)]),
        crate::preserves_rail::record("request", vec![optional_string(input.request_ref)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("apply-receipt-bound"),
                crate::preserves_rail::string(reconcile_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("send-receipt-current"),
                crate::preserves_rail::string(reconcile_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("receiver-ingress-bound"),
                crate::preserves_rail::string(reconcile_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("durable-enqueue-or-deny"),
                crate::preserves_rail::string(reconcile_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("control-dispatch-bound"),
                crate::preserves_rail::string(reconcile_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("reconcile-receipt-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("provenance-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn live_workflow_bundle_ack_value(input: &LiveWorkflowBundleAckValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.receiver_decision)?;
    Ok(crate::preserves_rail::record("node-control-live-workflow-bundle-ack-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_ACK_SCHEMA),
        crate::preserves_rail::record("apply-receipt", vec![input.apply_receipt_value.clone()]),
        crate::preserves_rail::record("send-receipt", vec![optional_value(input.send_receipt_value)]),
        crate::preserves_rail::record("ingress-receipt", vec![optional_value(input.ingress_receipt_value)]),
        crate::preserves_rail::record("queue-receipt", vec![optional_value(input.queue_receipt_value)]),
        crate::preserves_rail::record("control-receipt", vec![optional_value(input.control_receipt_value)]),
        crate::preserves_rail::record("reconcile-receipt", vec![input.reconcile_receipt_value.clone()]),
        crate::preserves_rail::record("apply-ref", vec![crate::preserves_rail::string(input.apply_receipt_ref)]),
        crate::preserves_rail::record("send-ref", vec![optional_string(input.send_receipt_ref)]),
        crate::preserves_rail::record("ingress-ref", vec![optional_string(input.ingress_receipt_ref)]),
        crate::preserves_rail::record("queue-ref", vec![optional_string(input.queue_receipt_ref)]),
        crate::preserves_rail::record("control-ref", vec![optional_string(input.control_receipt_ref)]),
        crate::preserves_rail::record("reconcile-ref", vec![crate::preserves_rail::string(
            input.reconcile_receipt_ref,
        )]),
        crate::preserves_rail::record("bundle", vec![crate::preserves_rail::string(input.bundle_ref)]),
        crate::preserves_rail::record("envelope", vec![optional_string(input.envelope_ref)]),
        crate::preserves_rail::record("operation", vec![optional_string(input.operation_ref)]),
        crate::preserves_rail::record("request", vec![optional_string(input.request_ref)]),
        crate::preserves_rail::record("receiver-decision", vec![crate::preserves_rail::string(
            input.receiver_decision,
        )]),
        crate::preserves_rail::record("receiver-diagnostics", vec![crate::preserves_rail::sequence(
            input.receiver_diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("ack-member-refs-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("receiver-outcome-recorded"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("ack-bundle-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("provenance-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
        crate::preserves_rail::record("member-refs", vec![crate::preserves_rail::sequence(
            [
                Some(input.apply_receipt_ref),
                input.send_receipt_ref,
                input.ingress_receipt_ref,
                input.queue_receipt_ref,
                input.control_receipt_ref,
                Some(input.reconcile_receipt_ref),
            ]
            .into_iter()
            .flatten()
            .map(crate::preserves_rail::string)
            .collect(),
        )]),
    ]))
}

fn live_workflow_bundle_ack_export_receipt_value(
    input: &LiveWorkflowBundleAckExportReceiptValueInput<'_>,
) -> Result<IoValue> {
    validate_decision(input.decision)?;
    let ack_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(crate::preserves_rail::record("node-control-live-workflow-bundle-ack-export-receipt-v1", vec![
        crate::preserves_rail::string(
            crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_ACK_EXPORT_RECEIPT_SCHEMA,
        ),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("ack", vec![crate::preserves_rail::string(&input.ack.ack_ref)]),
        crate::preserves_rail::record("bundle", vec![crate::preserves_rail::string(&input.ack.bundle_ref)]),
        crate::preserves_rail::record("apply-receipt", vec![crate::preserves_rail::string(
            &input.ack.apply_receipt_ref,
        )]),
        crate::preserves_rail::record("send-receipt", vec![optional_string(input.ack.send_receipt_ref.as_deref())]),
        crate::preserves_rail::record("ingress-receipt", vec![optional_string(
            input.ack.ingress_receipt_ref.as_deref(),
        )]),
        crate::preserves_rail::record("queue-receipt", vec![optional_string(input.ack.queue_receipt_ref.as_deref())]),
        crate::preserves_rail::record("control-receipt", vec![optional_string(
            input.ack.control_receipt_ref.as_deref(),
        )]),
        crate::preserves_rail::record("reconcile-receipt", vec![crate::preserves_rail::string(
            &input.ack.reconcile_receipt_ref,
        )]),
        crate::preserves_rail::record("envelope", vec![optional_string(input.ack.envelope_ref.as_deref())]),
        crate::preserves_rail::record("operation", vec![optional_string(input.ack.operation_ref.as_deref())]),
        crate::preserves_rail::record("request", vec![optional_string(input.ack.request_ref.as_deref())]),
        crate::preserves_rail::record("receiver-decision", vec![crate::preserves_rail::string(
            &input.ack.receiver_decision,
        )]),
        crate::preserves_rail::record("receiver-diagnostics", vec![crate::preserves_rail::sequence(
            input.ack.receiver_diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("ack-bundle-kind-version"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("receiver-evidence-packaged"),
                crate::preserves_rail::string(ack_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("reconcile-receipt-current"),
                crate::preserves_rail::string(ack_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("ack-export-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("provenance-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn live_workflow_bundle_ack_import_receipt_value(
    input: &LiveWorkflowBundleAckImportReceiptValueInput<'_>,
) -> Result<IoValue> {
    validate_decision(input.decision)?;
    let ack_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(crate::preserves_rail::record("node-control-live-workflow-bundle-ack-import-receipt-v1", vec![
        crate::preserves_rail::string(
            crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_ACK_IMPORT_RECEIPT_SCHEMA,
        ),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("state-root", vec![crate::preserves_rail::string(
            input.state_root.display().to_string(),
        )]),
        crate::preserves_rail::record("ack", vec![crate::preserves_rail::string(&input.ack.ack_ref)]),
        crate::preserves_rail::record("bundle", vec![crate::preserves_rail::string(&input.ack.bundle_ref)]),
        crate::preserves_rail::record("imported", vec![crate::preserves_rail::sequence(
            input.imported_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("receiver-decision", vec![crate::preserves_rail::string(
            &input.ack.receiver_decision,
        )]),
        crate::preserves_rail::record("receiver-diagnostics", vec![crate::preserves_rail::sequence(
            input.ack.receiver_diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("ack-bundle-kind-version"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("ack-member-bindings"),
                crate::preserves_rail::string(ack_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("sender-ledger-imported"),
                crate::preserves_rail::string(ack_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("ack-import-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("provenance-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}
