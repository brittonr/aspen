//! Shared receipt value helpers and live workflow receipt values.

#[derive(Debug, Clone, Copy)]
pub(super) struct AuthorityReceiptValueInput<'a> {
    pub(super) decision: &'a str,
    pub(super) envelope: &'a super::transits::ControlIngressEnvelope,
    pub(super) grant_ref: Option<&'a str>,
    pub(super) diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub(super) struct LiveWorkflowReceiptValueInput<'a> {
    pub(super) decision: &'a str,
    pub(super) ticket: &'a super::entitlements::ControlLiveTicket,
    pub(super) admission: &'a super::entitlements::ControlLivePeerAdmission,
    pub(super) authority: &'a super::entitlements::ControlAuthorityGrant,
    pub(super) send: &'a super::transits::ControlLiveSendReceipt,
    pub(super) receive_receipt_refs: &'a [String],
    pub(super) listener_receipt_ref: Option<&'a str>,
    pub(super) service_receipt_ref: &'a str,
    pub(super) diagnostics: &'a [String],
}

pub(super) fn receipt_value(input: &AuthorityReceiptValueInput<'_>) -> crate::error::Result<preserves::IOValue> {
    super::guards::validate_decision(input.decision)?;
    Ok(crate::preserves_rail::record("node-control-authority-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_AUTHORITY_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("envelope", vec![crate::preserves_rail::string(&input.envelope.envelope_ref)]),
        crate::preserves_rail::record("request", vec![crate::preserves_rail::string(
            &input.envelope.request.request_ref,
        )]),
        crate::preserves_rail::record("from-peer", vec![crate::preserves_rail::string(&input.envelope.from_peer)]),
        crate::preserves_rail::record("to-node", vec![crate::preserves_rail::string(&input.envelope.to_node)]),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(
            &input.envelope.request.operation,
        )]),
        crate::preserves_rail::record("grant", vec![super::fields::optional_string(input.grant_ref)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("peer-node-bound"),
                crate::preserves_rail::string(if input.grant_ref.is_some() { "pass" } else { "fail" }),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("operation-scope-bound"),
                crate::preserves_rail::string(if input.grant_ref.is_some() { "pass" } else { "fail" }),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("revocation-checked-at-ingress"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("transport-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

pub(super) fn live_workflow_receipt_value(
    input: &LiveWorkflowReceiptValueInput<'_>,
) -> crate::error::Result<preserves::IOValue> {
    super::guards::validate_decision(input.decision)?;
    Ok(crate::preserves_rail::record("node-control-live-workflow-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(&input.ticket.topic)]),
        crate::preserves_rail::record("peer", vec![crate::preserves_rail::string(&input.admission.peer_id)]),
        crate::preserves_rail::record("node", vec![crate::preserves_rail::string(&input.ticket.node_id)]),
        crate::preserves_rail::record("receiver-ticket", vec![crate::preserves_rail::string(&input.ticket.ticket_ref)]),
        crate::preserves_rail::record("peer-admission", vec![crate::preserves_rail::string(
            &input.admission.admission_ref,
        )]),
        crate::preserves_rail::record("authority-grant", vec![crate::preserves_rail::string(
            &input.authority.grant_ref,
        )]),
        crate::preserves_rail::record("send-receipt", vec![crate::preserves_rail::string(&input.send.receipt_ref)]),
        crate::preserves_rail::record("receive-receipts", vec![crate::preserves_rail::sequence(
            input.receive_receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("listener-receipt", vec![super::fields::optional_string(
            input.listener_receipt_ref,
        )]),
        crate::preserves_rail::record("service-run", vec![crate::preserves_rail::string(input.service_receipt_ref)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![live_workflow_check_sequence(input)]),
    ]))
}

fn live_workflow_check_sequence(input: &LiveWorkflowReceiptValueInput<'_>) -> preserves::IOValue {
    crate::preserves_rail::sequence(vec![
        receipt_check_value("ticket-admission-bound", pass_if(input.admission.ticket_ref == input.ticket.ticket_ref)),
        receipt_check_value(
            "authority-grant-bound",
            pass_if(
                input.authority.peer_id == input.admission.peer_id && input.authority.node_id == input.ticket.node_id,
            ),
        ),
        receipt_check_value("send-ticket-bound", pass_if(input.send.receiver_ticket_ref == input.ticket.ticket_ref)),
        receipt_check_value("receive-before-service", fail_if(input.receive_receipt_refs.is_empty())),
        receipt_check_value("transport-is-not-authority", "pass"),
        receipt_check_value("durable-inbox-boundary", "pass"),
    ])
}

pub(super) fn pass_if(condition: bool) -> &'static str {
    if condition { "pass" } else { "fail" }
}

pub(super) fn fail_if(condition: bool) -> &'static str {
    if condition { "fail" } else { "pass" }
}

pub(super) fn receipt_check_value(name: &str, status: &str) -> preserves::IOValue {
    crate::preserves_rail::record("check", vec![
        crate::preserves_rail::string(name),
        crate::preserves_rail::string(status),
    ])
}
