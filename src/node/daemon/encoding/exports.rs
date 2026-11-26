//! Receipt values for bundle export and verify.

#[derive(Debug, Clone, Copy)]
pub(super) struct LiveWorkflowBundleValueInput<'a> {
    pub(super) ticket: &'a super::entitlements::ControlLiveTicket,
    pub(super) admission: &'a super::entitlements::ControlLivePeerAdmission,
    pub(super) authority: &'a super::entitlements::ControlAuthorityGrant,
    pub(super) ticket_value: &'a preserves::IOValue,
    pub(super) admission_value: &'a preserves::IOValue,
    pub(super) authority_value: &'a preserves::IOValue,
    pub(super) receipt_values: &'a [&'a preserves::IOValue],
    pub(super) diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub(super) struct LiveWorkflowBundleExportReceiptValueInput<'a> {
    pub(super) decision: &'a str,
    pub(super) bundle: &'a super::artifacts::ControlLiveWorkflowBundle,
    pub(super) diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub(super) struct LiveWorkflowBundleVerifyReceiptValueInput<'a> {
    pub(super) decision: &'a str,
    pub(super) bundle_ref: &'a str,
    pub(super) ticket_ref: Option<&'a str>,
    pub(super) peer_admission_ref: Option<&'a str>,
    pub(super) authority_grant_ref: Option<&'a str>,
    pub(super) receipt_refs: &'a [String],
    pub(super) expected: &'a super::intake::LiveWorkflowBundleExpectedInput<'a>,
    pub(super) diagnostics: &'a [String],
}

pub(super) fn live_workflow_bundle_value(
    input: &LiveWorkflowBundleValueInput<'_>,
) -> crate::error::Result<preserves::IOValue> {
    let binding_status = if input.diagnostics.is_empty() { "pass" } else { "fail" };
    let receipt_refs = super::exporting::live_workflow_bundle_receipt_refs(input.receipt_values)?;
    Ok(crate::preserves_rail::record("node-control-live-workflow-bundle-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_SCHEMA),
        crate::preserves_rail::record("ticket", vec![(*input.ticket_value).clone()]),
        crate::preserves_rail::record("peer-admission", vec![(*input.admission_value).clone()]),
        crate::preserves_rail::record("authority-grant", vec![(*input.authority_value).clone()]),
        crate::preserves_rail::record("receipts", vec![crate::preserves_rail::sequence(
            input.receipt_values.iter().map(|value| (**value).clone()).collect(),
        )]),
        crate::preserves_rail::record("ticket-ref", vec![crate::preserves_rail::string(&input.ticket.ticket_ref)]),
        crate::preserves_rail::record("peer-admission-ref", vec![crate::preserves_rail::string(
            &input.admission.admission_ref,
        )]),
        crate::preserves_rail::record("authority-grant-ref", vec![crate::preserves_rail::string(
            &input.authority.grant_ref,
        )]),
        crate::preserves_rail::record("receipt-refs", vec![crate::preserves_rail::sequence(
            receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("ticket-kind-version"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("peer-admission-kind-version"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("authority-grant-kind-version"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("ticket-admission-bound"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("authority-grant-bound"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("provenance-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

pub(super) fn live_workflow_bundle_export_receipt_value(
    input: &LiveWorkflowBundleExportReceiptValueInput<'_>,
) -> crate::error::Result<preserves::IOValue> {
    super::guards::validate_decision(input.decision)?;
    let binding_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(crate::preserves_rail::record("node-control-live-workflow-bundle-export-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_EXPORT_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("bundle", vec![crate::preserves_rail::string(&input.bundle.bundle_ref)]),
        crate::preserves_rail::record("ticket", vec![crate::preserves_rail::string(&input.bundle.ticket_ref)]),
        crate::preserves_rail::record("peer-admission", vec![crate::preserves_rail::string(
            &input.bundle.peer_admission_ref,
        )]),
        crate::preserves_rail::record("authority-grant", vec![crate::preserves_rail::string(
            &input.bundle.authority_grant_ref,
        )]),
        crate::preserves_rail::record("receipts", vec![crate::preserves_rail::sequence(
            input.bundle.receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-kind-version"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-member-bindings"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-receipt-kinds"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("provenance-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

pub(super) fn live_workflow_bundle_verify_receipt_value(
    input: &LiveWorkflowBundleVerifyReceiptValueInput<'_>,
) -> crate::error::Result<preserves::IOValue> {
    super::guards::validate_decision(input.decision)?;
    let binding_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(crate::preserves_rail::record("node-control-live-workflow-bundle-verify-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_VERIFY_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("bundle", vec![crate::preserves_rail::string(input.bundle_ref)]),
        crate::preserves_rail::record("ticket", vec![super::fields::optional_string(input.ticket_ref)]),
        crate::preserves_rail::record("peer-admission", vec![super::fields::optional_string(input.peer_admission_ref)]),
        crate::preserves_rail::record("authority-grant", vec![super::fields::optional_string(
            input.authority_grant_ref,
        )]),
        crate::preserves_rail::record("receipts", vec![crate::preserves_rail::sequence(
            input.receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("expected", vec![super::gates::live_workflow_bundle_expected_value(
            input.expected,
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-kind-version"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-member-bindings"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-receipt-kinds"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("expected-bindings"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("verify-receipt-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("provenance-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}
