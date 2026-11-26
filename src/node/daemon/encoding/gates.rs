//! Receipt values for bundle gate and apply.

#[derive(Debug, Clone, Copy)]
pub(super) struct LiveWorkflowBundleGateReceiptValueInput<'a> {
    pub(super) decision: &'a str,
    pub(super) bundle_ref: &'a str,
    pub(super) verify_receipt_ref: Option<&'a str>,
    pub(super) recomputed_verify_receipt_ref: &'a str,
    pub(super) ticket_ref: Option<&'a str>,
    pub(super) peer_admission_ref: Option<&'a str>,
    pub(super) authority_grant_ref: Option<&'a str>,
    pub(super) receipt_refs: &'a [String],
    pub(super) expected: &'a super::intake::LiveWorkflowBundleExpectedInput<'a>,
    pub(super) diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub(super) struct LiveWorkflowBundleApplyReceiptValueInput<'a> {
    pub(super) decision: &'a str,
    pub(super) bundle_ref: &'a str,
    pub(super) gate_receipt_ref: Option<&'a str>,
    pub(super) recomputed_verify_receipt_ref: &'a str,
    pub(super) import_receipt_ref: Option<&'a str>,
    pub(super) imported_refs: &'a [String],
    pub(super) mode: &'a str,
    pub(super) envelope_ref: Option<&'a str>,
    pub(super) operation_ref: Option<&'a str>,
    pub(super) send_receipt_ref: Option<&'a str>,
    pub(super) topology_profile_ref: Option<&'a str>,
    pub(super) transport_profile_ref: Option<&'a str>,
    pub(super) effective_max_attempts: u64,
    pub(super) effective_join_timeout_ms: u64,
    pub(super) expected: &'a super::intake::LiveWorkflowBundleExpectedInput<'a>,
    pub(super) diagnostics: &'a [String],
}

pub(super) fn live_workflow_bundle_gate_receipt_value(
    input: &LiveWorkflowBundleGateReceiptValueInput<'_>,
) -> crate::error::Result<preserves::IOValue> {
    super::guards::validate_decision(input.decision)?;
    let gate_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(crate::preserves_rail::record("node-control-live-workflow-bundle-gate-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_GATE_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("bundle", vec![crate::preserves_rail::string(input.bundle_ref)]),
        crate::preserves_rail::record("verify-receipt", vec![super::fields::optional_string(input.verify_receipt_ref)]),
        crate::preserves_rail::record("recomputed-verify", vec![crate::preserves_rail::string(
            input.recomputed_verify_receipt_ref,
        )]),
        crate::preserves_rail::record("ticket", vec![super::fields::optional_string(input.ticket_ref)]),
        crate::preserves_rail::record("peer-admission", vec![super::fields::optional_string(input.peer_admission_ref)]),
        crate::preserves_rail::record("authority-grant", vec![super::fields::optional_string(
            input.authority_grant_ref,
        )]),
        crate::preserves_rail::record("receipts", vec![crate::preserves_rail::sequence(
            input.receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("expected", vec![live_workflow_bundle_expected_value(input.expected)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-verification"),
                crate::preserves_rail::string(gate_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("verify-receipt-current"),
                crate::preserves_rail::string(gate_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("expected-bindings"),
                crate::preserves_rail::string(gate_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("gate-receipt-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-import-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("provenance-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

pub(super) fn live_workflow_bundle_apply_receipt_value(
    input: &LiveWorkflowBundleApplyReceiptValueInput<'_>,
) -> crate::error::Result<preserves::IOValue> {
    super::guards::validate_decision(input.decision)?;
    let apply_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(crate::preserves_rail::record("node-control-live-workflow-bundle-apply-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_APPLY_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("state-root", vec![crate::preserves_rail::string(
            &super::storage::state_root_profile_ref(&())?,
        )]),
        crate::preserves_rail::record("bundle", vec![crate::preserves_rail::string(input.bundle_ref)]),
        crate::preserves_rail::record("gate-receipt", vec![super::fields::optional_string(input.gate_receipt_ref)]),
        crate::preserves_rail::record("recomputed-verify", vec![crate::preserves_rail::string(
            input.recomputed_verify_receipt_ref,
        )]),
        crate::preserves_rail::record("import-receipt", vec![super::fields::optional_string(input.import_receipt_ref)]),
        crate::preserves_rail::record("imported", vec![crate::preserves_rail::sequence(
            input.imported_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("mode", vec![crate::preserves_rail::string(input.mode)]),
        crate::preserves_rail::record("envelope", vec![super::fields::optional_string(input.envelope_ref)]),
        crate::preserves_rail::record("operation", vec![super::fields::optional_string(input.operation_ref)]),
        crate::preserves_rail::record("send-receipt", vec![super::fields::optional_string(input.send_receipt_ref)]),
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
        super::tuning::live_profile_ref_records(input.topology_profile_ref, input.transport_profile_ref),
        super::tuning::live_effective_transport_record(input.effective_max_attempts, input.effective_join_timeout_ms),
    ]))
}

pub(super) fn live_workflow_bundle_expected_value(
    input: &super::intake::LiveWorkflowBundleExpectedInput<'_>,
) -> preserves::IOValue {
    crate::preserves_rail::record("expected", vec![crate::preserves_rail::sequence(vec![
        crate::preserves_rail::record("node", vec![super::fields::optional_string(input.expected_node)]),
        crate::preserves_rail::record("topic", vec![super::fields::optional_string(input.expected_topic)]),
        crate::preserves_rail::record("endpoint", vec![super::fields::optional_string(input.expected_endpoint)]),
        crate::preserves_rail::record("peer", vec![super::fields::optional_string(input.expected_peer)]),
        crate::preserves_rail::record("operations", vec![crate::preserves_rail::sequence(
            input.expected_operations.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("target-scope", vec![super::fields::optional_string(
            input.expected_target_scope,
        )]),
        crate::preserves_rail::record("resource-scope", vec![super::fields::optional_string(
            input.expected_resource_scope,
        )]),
        crate::preserves_rail::record("as-of-sequence", vec![crate::preserves_rail::string(
            input.as_of_sequence.to_string(),
        )]),
        crate::preserves_rail::record("as-of-epoch", vec![crate::preserves_rail::string(
            input.as_of_epoch.to_string(),
        )]),
    ])])
}
