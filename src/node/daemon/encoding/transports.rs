//! Receipt values for live listeners and transports.

#[derive(Debug, Clone, Copy)]
pub(super) struct ListenerReceiptValueInput<'a> {
    pub(super) decision: &'a str,
    pub(super) startup_receipt_ref: &'a str,
    pub(super) node_id: &'a str,
    pub(super) logical_endpoint_id: &'a str,
    pub(super) bound_endpoint_id: &'a str,
    pub(super) topic: &'a str,
    pub(super) max_events: u64,
    pub(super) observed_events: u64,
    pub(super) transport_receipt_refs: &'a [String],
    pub(super) neighbor_events: &'a [String],
    pub(super) service_receipt_ref: &'a str,
    pub(super) diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub(super) struct LiveTransportReceiptValueInput<'a> {
    pub(super) operation: &'a str,
    pub(super) decision: &'a str,
    pub(super) node_id: &'a str,
    pub(super) delivered_from: Option<&'a str>,
    pub(super) envelope: &'a super::transits::ControlIngressEnvelope,
    pub(super) ingress_receipt_ref: Option<&'a str>,
    pub(super) topology_profile_ref: Option<&'a str>,
    pub(super) transport_profile_ref: Option<&'a str>,
    pub(super) effective_max_attempts: Option<u64>,
    pub(super) effective_join_timeout_ms: Option<u64>,
    pub(super) diagnostics: &'a [String],
}

pub(super) fn live_listener_receipt_value(
    input: &ListenerReceiptValueInput<'_>,
) -> crate::error::Result<preserves::IOValue> {
    super::guards::validate_decision(input.decision)?;
    Ok(crate::preserves_rail::record("node-control-live-listener-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_LISTENER_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("startup", vec![crate::preserves_rail::string(input.startup_receipt_ref)]),
        crate::preserves_rail::record("node", vec![crate::preserves_rail::string(input.node_id)]),
        crate::preserves_rail::record("logical-endpoint", vec![crate::preserves_rail::string(
            input.logical_endpoint_id,
        )]),
        crate::preserves_rail::record("bound-endpoint", vec![crate::preserves_rail::string(input.bound_endpoint_id)]),
        crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(input.topic)]),
        crate::preserves_rail::record("max-events", vec![crate::preserves_rail::string(input.max_events.to_string())]),
        crate::preserves_rail::record("observed-events", vec![crate::preserves_rail::string(
            input.observed_events.to_string(),
        )]),
        crate::preserves_rail::record("transport-receipts", vec![crate::preserves_rail::sequence(
            input.transport_receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("neighbor-events", vec![crate::preserves_rail::sequence(
            input.neighbor_events.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("service-run", vec![crate::preserves_rail::string(input.service_receipt_ref)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("live-iroh-listener"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("receive-before-drain"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("session-evidence-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bounded-listener"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("durable-inbox-boundary"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
        super::tuning::live_profile_ref_records(None, None),
        super::tuning::live_effective_transport_optional_record(None, None),
    ]))
}

pub(super) fn live_transport_receipt_value(
    input: &LiveTransportReceiptValueInput<'_>,
) -> crate::error::Result<preserves::IOValue> {
    super::guards::validate_decision(input.decision)?;
    let has_peer_bootstrap = !input.envelope.peer_bootstrap_refs.is_empty();
    Ok(crate::preserves_rail::record("node-control-live-transport-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_TRANSPORT_RECEIPT_SCHEMA),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(input.operation)]),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("transport", vec![crate::preserves_rail::string(
            super::settings::LIVE_CONTROL_INGRESS_TRANSPORT,
        )]),
        crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(&input.envelope.topic)]),
        crate::preserves_rail::record("node", vec![crate::preserves_rail::string(input.node_id)]),
        crate::preserves_rail::record("delivered-from", vec![super::fields::optional_string(input.delivered_from)]),
        crate::preserves_rail::record("envelope", vec![crate::preserves_rail::string(&input.envelope.envelope_ref)]),
        crate::preserves_rail::record("ingress-receipt", vec![super::fields::optional_string(
            input.ingress_receipt_ref,
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("canonical-envelope-ref"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("live-iroh-gossip"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("peer-bootstrap-before-enqueue"),
                crate::preserves_rail::string(if has_peer_bootstrap { "pass" } else { "fail" }),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("transport-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("durable-inbox-boundary"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
        super::tuning::live_profile_ref_records(input.topology_profile_ref, input.transport_profile_ref),
        super::tuning::live_effective_transport_optional_record(
            input.effective_max_attempts,
            input.effective_join_timeout_ms,
        ),
    ]))
}
