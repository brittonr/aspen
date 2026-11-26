//! Receipt values for bundle reconcile and acknowledgements.

#[derive(Debug, Clone, Copy)]
pub(super) struct LiveWorkflowBundleReconcileReceiptValueInput<'a> {
    pub(super) decision: &'a str,
    pub(super) apply_receipt_ref: &'a str,
    pub(super) bundle_ref: &'a str,
    pub(super) send_receipt_ref: Option<&'a str>,
    pub(super) ingress_receipt_ref: Option<&'a str>,
    pub(super) queue_receipt_ref: Option<&'a str>,
    pub(super) control_receipt_ref: Option<&'a str>,
    pub(super) envelope_ref: Option<&'a str>,
    pub(super) operation_ref: Option<&'a str>,
    pub(super) request_ref: Option<&'a str>,
    pub(super) diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub(super) struct LiveWorkflowBundleAckValueInput<'a> {
    pub(super) apply_receipt_value: &'a preserves::IOValue,
    pub(super) send_receipt_value: Option<&'a preserves::IOValue>,
    pub(super) ingress_receipt_value: Option<&'a preserves::IOValue>,
    pub(super) queue_receipt_value: Option<&'a preserves::IOValue>,
    pub(super) control_receipt_value: Option<&'a preserves::IOValue>,
    pub(super) reconcile_receipt_value: &'a preserves::IOValue,
    pub(super) apply_receipt_ref: &'a str,
    pub(super) send_receipt_ref: Option<&'a str>,
    pub(super) ingress_receipt_ref: Option<&'a str>,
    pub(super) queue_receipt_ref: Option<&'a str>,
    pub(super) control_receipt_ref: Option<&'a str>,
    pub(super) reconcile_receipt_ref: &'a str,
    pub(super) bundle_ref: &'a str,
    pub(super) envelope_ref: Option<&'a str>,
    pub(super) operation_ref: Option<&'a str>,
    pub(super) request_ref: Option<&'a str>,
    pub(super) receiver_decision: &'a str,
    pub(super) receiver_diagnostics: &'a [String],
    pub(super) diagnostics: &'a [String],
}

pub(super) fn live_workflow_bundle_reconcile_receipt_value(
    input: &LiveWorkflowBundleReconcileReceiptValueInput<'_>,
) -> crate::error::Result<preserves::IOValue> {
    super::guards::validate_decision(input.decision)?;
    let reconcile_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(crate::preserves_rail::record("node-control-live-workflow-bundle-reconcile-receipt-v1", vec![
        crate::preserves_rail::string(
            crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_RECONCILE_RECEIPT_SCHEMA,
        ),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("apply-receipt", vec![crate::preserves_rail::string(input.apply_receipt_ref)]),
        crate::preserves_rail::record("bundle", vec![crate::preserves_rail::string(input.bundle_ref)]),
        crate::preserves_rail::record("send-receipt", vec![super::fields::optional_string(input.send_receipt_ref)]),
        crate::preserves_rail::record("ingress-receipt", vec![super::fields::optional_string(
            input.ingress_receipt_ref,
        )]),
        crate::preserves_rail::record("queue-receipt", vec![super::fields::optional_string(input.queue_receipt_ref)]),
        crate::preserves_rail::record("control-receipt", vec![super::fields::optional_string(
            input.control_receipt_ref,
        )]),
        crate::preserves_rail::record("envelope", vec![super::fields::optional_string(input.envelope_ref)]),
        crate::preserves_rail::record("operation", vec![super::fields::optional_string(input.operation_ref)]),
        crate::preserves_rail::record("request", vec![super::fields::optional_string(input.request_ref)]),
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

pub(super) fn live_workflow_bundle_ack_value(
    input: &LiveWorkflowBundleAckValueInput<'_>,
) -> crate::error::Result<preserves::IOValue> {
    super::guards::validate_decision(input.receiver_decision)?;
    Ok(crate::preserves_rail::record("node-control-live-workflow-bundle-ack-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_ACK_SCHEMA),
        crate::preserves_rail::record("apply-receipt", vec![input.apply_receipt_value.clone()]),
        crate::preserves_rail::record("send-receipt", vec![super::fields::optional_value(input.send_receipt_value)]),
        crate::preserves_rail::record("ingress-receipt", vec![super::fields::optional_value(
            input.ingress_receipt_value,
        )]),
        crate::preserves_rail::record("queue-receipt", vec![super::fields::optional_value(input.queue_receipt_value)]),
        crate::preserves_rail::record("control-receipt", vec![super::fields::optional_value(
            input.control_receipt_value,
        )]),
        crate::preserves_rail::record("reconcile-receipt", vec![input.reconcile_receipt_value.clone()]),
        crate::preserves_rail::record("apply-ref", vec![crate::preserves_rail::string(input.apply_receipt_ref)]),
        crate::preserves_rail::record("send-ref", vec![super::fields::optional_string(input.send_receipt_ref)]),
        crate::preserves_rail::record("ingress-ref", vec![super::fields::optional_string(input.ingress_receipt_ref)]),
        crate::preserves_rail::record("queue-ref", vec![super::fields::optional_string(input.queue_receipt_ref)]),
        crate::preserves_rail::record("control-ref", vec![super::fields::optional_string(input.control_receipt_ref)]),
        crate::preserves_rail::record("reconcile-ref", vec![crate::preserves_rail::string(
            input.reconcile_receipt_ref,
        )]),
        crate::preserves_rail::record("bundle", vec![crate::preserves_rail::string(input.bundle_ref)]),
        crate::preserves_rail::record("envelope", vec![super::fields::optional_string(input.envelope_ref)]),
        crate::preserves_rail::record("operation", vec![super::fields::optional_string(input.operation_ref)]),
        crate::preserves_rail::record("request", vec![super::fields::optional_string(input.request_ref)]),
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
