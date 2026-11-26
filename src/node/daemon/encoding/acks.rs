//! Receipt values for acknowledgement export and import.

#[derive(Debug, Clone, Copy)]
pub(super) struct LiveWorkflowBundleAckExportReceiptValueInput<'a> {
    pub(super) decision: &'a str,
    pub(super) ack: &'a super::artifacts::ControlLiveWorkflowBundleAck,
    pub(super) diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub(super) struct LiveWorkflowBundleAckImportReceiptValueInput<'a> {
    pub(super) decision: &'a str,
    pub(super) ack: &'a super::artifacts::ControlLiveWorkflowBundleAck,
    pub(super) imported_refs: &'a [String],
    pub(super) diagnostics: &'a [String],
}

pub(super) fn live_workflow_bundle_ack_export_receipt_value(
    input: &LiveWorkflowBundleAckExportReceiptValueInput<'_>,
) -> crate::error::Result<preserves::IOValue> {
    super::guards::validate_decision(input.decision)?;
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
        crate::preserves_rail::record("send-receipt", vec![super::fields::optional_string(
            input.ack.send_receipt_ref.as_deref(),
        )]),
        crate::preserves_rail::record("ingress-receipt", vec![super::fields::optional_string(
            input.ack.ingress_receipt_ref.as_deref(),
        )]),
        crate::preserves_rail::record("queue-receipt", vec![super::fields::optional_string(
            input.ack.queue_receipt_ref.as_deref(),
        )]),
        crate::preserves_rail::record("control-receipt", vec![super::fields::optional_string(
            input.ack.control_receipt_ref.as_deref(),
        )]),
        crate::preserves_rail::record("reconcile-receipt", vec![crate::preserves_rail::string(
            &input.ack.reconcile_receipt_ref,
        )]),
        crate::preserves_rail::record("envelope", vec![super::fields::optional_string(
            input.ack.envelope_ref.as_deref(),
        )]),
        crate::preserves_rail::record("operation", vec![super::fields::optional_string(
            input.ack.operation_ref.as_deref(),
        )]),
        crate::preserves_rail::record("request", vec![super::fields::optional_string(
            input.ack.request_ref.as_deref(),
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

pub(super) fn live_workflow_bundle_ack_import_receipt_value(
    input: &LiveWorkflowBundleAckImportReceiptValueInput<'_>,
) -> crate::error::Result<preserves::IOValue> {
    super::guards::validate_decision(input.decision)?;
    let ack_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(crate::preserves_rail::record("node-control-live-workflow-bundle-ack-import-receipt-v1", vec![
        crate::preserves_rail::string(
            crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_ACK_IMPORT_RECEIPT_SCHEMA,
        ),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("state-root", vec![crate::preserves_rail::string(
            &super::storage::state_root_profile_ref(&())?,
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
