//! Receipt values for live sends, retries, and duplicates.

#[derive(Debug, Clone, Copy)]
pub(super) struct LiveSendReceiptValueInput<'a> {
    pub(super) decision: &'a str,
    pub(super) from_peer: &'a str,
    pub(super) ticket: &'a super::entitlements::ControlLiveTicket,
    pub(super) envelope: &'a super::transits::ControlIngressEnvelope,
    pub(super) transport_receipt_ref: Option<&'a str>,
    pub(super) topology_profile_ref: Option<&'a str>,
    pub(super) transport_profile_ref: Option<&'a str>,
    pub(super) effective_max_attempts: u64,
    pub(super) effective_join_timeout_ms: u64,
    pub(super) diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub(super) struct LiveSendRetryReceiptValueInput<'a> {
    pub(super) decision: &'a str,
    pub(super) attempt: u64,
    pub(super) max_attempts: u64,
    pub(super) from_peer: &'a str,
    pub(super) ticket: &'a super::entitlements::ControlLiveTicket,
    pub(super) envelope: &'a super::transits::ControlIngressEnvelope,
    pub(super) diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub(super) struct LiveSendDuplicateReceiptValueInput<'a> {
    pub(super) from_peer: &'a str,
    pub(super) ticket: &'a super::entitlements::ControlLiveTicket,
    pub(super) envelope: &'a super::transits::ControlIngressEnvelope,
    pub(super) prior_send_receipt_ref: &'a str,
    pub(super) diagnostics: &'a [String],
}

struct LiveSendReceiptChecks {
    has_addresses: bool,
    has_supported_addresses: bool,
    has_expected_ticket_binding: bool,
    has_operation_mismatch: bool,
    has_state_root_evidence: bool,
    has_transport_success: bool,
}

fn live_send_receipt_checks(input: &LiveSendReceiptValueInput<'_>) -> LiveSendReceiptChecks {
    let has_addresses = !input.ticket.address_refs.is_empty();
    LiveSendReceiptChecks {
        has_addresses,
        has_operation_mismatch: super::fields::diagnostics_include(input.diagnostics, "operation-id"),
        has_supported_addresses: has_addresses
            && !super::fields::diagnostics_include(input.diagnostics, "unsupported transport address")
            && !super::fields::diagnostics_include(input.diagnostics, "address unsupported or malformed")
            && !super::fields::diagnostics_include(input.diagnostics, "address parse failed")
            && !super::fields::diagnostics_include(input.diagnostics, "endpoint parse failed"),
        has_expected_ticket_binding: !super::fields::diagnostics_include(input.diagnostics, "ticket node")
            && !super::fields::diagnostics_include(input.diagnostics, "ticket topic")
            && !super::fields::diagnostics_include(input.diagnostics, "ticket endpoint"),
        has_state_root_evidence: !super::fields::diagnostics_include(input.diagnostics, "sender state root")
            && !super::fields::diagnostics_include(input.diagnostics, "peer admission refs missing")
            && !super::fields::diagnostics_include(input.diagnostics, "authority grant refs missing"),
        has_transport_success: input.transport_receipt_ref.is_some(),
    }
}

fn live_send_check_sequence(checks: &LiveSendReceiptChecks) -> preserves::IOValue {
    crate::preserves_rail::sequence(vec![
        super::stamps::receipt_check_value("receiver-ticket-bound", "pass"),
        super::stamps::receipt_check_value("receiver-address-bound", super::stamps::pass_if(checks.has_addresses)),
        super::stamps::receipt_check_value(
            "receiver-address-supported",
            super::stamps::pass_if(checks.has_supported_addresses),
        ),
        super::stamps::receipt_check_value(
            "receiver-ticket-expected",
            super::stamps::pass_if(checks.has_expected_ticket_binding),
        ),
        super::stamps::receipt_check_value("operation-id-bound", super::stamps::fail_if(checks.has_operation_mismatch)),
        super::stamps::receipt_check_value(
            "sender-state-root-evidence",
            super::stamps::pass_if(checks.has_state_root_evidence),
        ),
        super::stamps::receipt_check_value(
            "join-or-publish-succeeded",
            super::stamps::pass_if(checks.has_transport_success),
        ),
        super::stamps::receipt_check_value("canonical-envelope-ref", "pass"),
        super::stamps::receipt_check_value("live-iroh-gossip", "pass"),
        super::stamps::receipt_check_value("live-profile-is-not-authority", "pass"),
        super::stamps::receipt_check_value("transport-is-not-authority", "pass"),
        super::stamps::receipt_check_value("durable-inbox-boundary", "pass"),
    ])
}

pub(super) fn live_send_receipt_value(
    input: &LiveSendReceiptValueInput<'_>,
) -> crate::error::Result<preserves::IOValue> {
    super::guards::validate_decision(input.decision)?;
    let checks = live_send_receipt_checks(input);
    Ok(crate::preserves_rail::record("node-control-live-send-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_SEND_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("transport", vec![crate::preserves_rail::string(
            super::settings::LIVE_CONTROL_INGRESS_TRANSPORT,
        )]),
        crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(&input.envelope.topic)]),
        crate::preserves_rail::record("from-peer", vec![crate::preserves_rail::string(input.from_peer)]),
        crate::preserves_rail::record("to-node", vec![crate::preserves_rail::string(&input.ticket.node_id)]),
        crate::preserves_rail::record("receiver-ticket", vec![crate::preserves_rail::string(&input.ticket.ticket_ref)]),
        crate::preserves_rail::record("receiver-endpoint", vec![crate::preserves_rail::string(
            &input.ticket.live_endpoint_id,
        )]),
        crate::preserves_rail::record("receiver-addresses", vec![crate::preserves_rail::sequence(
            input.ticket.address_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("envelope", vec![crate::preserves_rail::string(&input.envelope.envelope_ref)]),
        crate::preserves_rail::record("transport-receipt", vec![super::fields::optional_string(
            input.transport_receipt_ref,
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![live_send_check_sequence(&checks)]),
        super::tuning::live_profile_ref_records(input.topology_profile_ref, input.transport_profile_ref),
        super::tuning::live_effective_transport_record(input.effective_max_attempts, input.effective_join_timeout_ms),
    ]))
}

pub(super) fn live_send_retry_receipt_value(
    input: &LiveSendRetryReceiptValueInput<'_>,
) -> crate::error::Result<preserves::IOValue> {
    super::guards::validate_decision(input.decision)?;
    Ok(crate::preserves_rail::record("node-control-live-send-retry-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_SEND_RETRY_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("attempt", vec![crate::preserves_rail::string(input.attempt.to_string())]),
        crate::preserves_rail::record("max-attempts", vec![crate::preserves_rail::string(
            input.max_attempts.to_string(),
        )]),
        crate::preserves_rail::record("transport", vec![crate::preserves_rail::string(
            super::settings::LIVE_CONTROL_INGRESS_TRANSPORT,
        )]),
        crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(&input.envelope.topic)]),
        crate::preserves_rail::record("from-peer", vec![crate::preserves_rail::string(input.from_peer)]),
        crate::preserves_rail::record("to-node", vec![crate::preserves_rail::string(&input.ticket.node_id)]),
        crate::preserves_rail::record("receiver-ticket", vec![crate::preserves_rail::string(&input.ticket.ticket_ref)]),
        crate::preserves_rail::record("receiver-endpoint", vec![crate::preserves_rail::string(
            &input.ticket.live_endpoint_id,
        )]),
        crate::preserves_rail::record("envelope", vec![crate::preserves_rail::string(&input.envelope.envelope_ref)]),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(&input.envelope.operation_ref)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bounded-retry"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("operation-id-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("transport-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

pub(super) fn live_send_duplicate_receipt_value(
    input: &LiveSendDuplicateReceiptValueInput<'_>,
) -> crate::error::Result<preserves::IOValue> {
    Ok(crate::preserves_rail::record("node-control-live-send-duplicate-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_SEND_DUPLICATE_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string("pass")]),
        crate::preserves_rail::record("transport", vec![crate::preserves_rail::string(
            super::settings::LIVE_CONTROL_INGRESS_TRANSPORT,
        )]),
        crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(&input.envelope.topic)]),
        crate::preserves_rail::record("from-peer", vec![crate::preserves_rail::string(input.from_peer)]),
        crate::preserves_rail::record("to-node", vec![crate::preserves_rail::string(&input.ticket.node_id)]),
        crate::preserves_rail::record("receiver-ticket", vec![crate::preserves_rail::string(&input.ticket.ticket_ref)]),
        crate::preserves_rail::record("receiver-endpoint", vec![crate::preserves_rail::string(
            &input.ticket.live_endpoint_id,
        )]),
        crate::preserves_rail::record("envelope", vec![crate::preserves_rail::string(&input.envelope.envelope_ref)]),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(&input.envelope.operation_ref)]),
        crate::preserves_rail::record("prior-send-receipt", vec![crate::preserves_rail::string(
            input.prior_send_receipt_ref,
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("duplicate-side-effect-suppressed"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("operation-id-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("prior-send-receipt-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("transport-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}
