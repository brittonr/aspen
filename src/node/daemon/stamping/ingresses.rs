//! Ingress envelope and ingress receipt values.

#[derive(Debug, Clone, Copy)]
pub(super) struct IngressReceiptValueInput<'a> {
    pub(super) decision: &'a str,
    pub(super) phase: &'a str,
    pub(super) transport: &'a str,
    pub(super) envelope: &'a super::transits::ControlIngressEnvelope,
    pub(super) idempotency_receipt_ref: Option<&'a str>,
    pub(super) queue_receipt_ref: Option<&'a str>,
    pub(super) diagnostics: &'a [String],
}

pub(super) fn ingress_envelope_value(
    input: &super::requests::ControlIngressEnvelopeInput<'_>,
    request: &crate::node_runtime::ControlRequest,
    operation_ref: &str,
    transport: &str,
    transport_check: &str,
) -> crate::error::Result<preserves::IOValue> {
    Ok(crate::preserves_rail::record("node-control-ingress-envelope-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_INGRESS_ENVELOPE_SCHEMA),
        crate::preserves_rail::record("transport", vec![crate::preserves_rail::string(transport)]),
        crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(input.topic)]),
        crate::preserves_rail::record("from-peer", vec![crate::preserves_rail::string(input.from_peer)]),
        crate::preserves_rail::record("to-node", vec![crate::preserves_rail::string(input.to_node)]),
        crate::preserves_rail::record("sequence", vec![crate::preserves_rail::string(input.sequence.to_string())]),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(operation_ref)]),
        crate::preserves_rail::record("request-ref", vec![crate::preserves_rail::string(&request.request_ref)]),
        crate::preserves_rail::record("request", vec![request.value.clone()]),
        crate::preserves_rail::record("peer-bootstrap", vec![crate::preserves_rail::sequence(
            input.peer_bootstrap_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("authority", vec![crate::preserves_rail::sequence(
            input.authority_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("policy", vec![crate::preserves_rail::sequence(
            input.policy_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("resource", vec![crate::preserves_rail::sequence(
            input.resource_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("evidence", vec![crate::preserves_rail::sequence(
            input.evidence_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("canonical-request-ref"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("operation-id-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string(transport_check),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("transport-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

pub(super) fn ingress_receipt_value(input: &IngressReceiptValueInput<'_>) -> crate::error::Result<preserves::IOValue> {
    super::guards::validate_decision(input.decision)?;
    Ok(crate::preserves_rail::record("node-control-ingress-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_INGRESS_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("phase", vec![crate::preserves_rail::string(input.phase)]),
        crate::preserves_rail::record("transport", vec![crate::preserves_rail::string(input.transport)]),
        crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(&input.envelope.topic)]),
        crate::preserves_rail::record("from-peer", vec![crate::preserves_rail::string(&input.envelope.from_peer)]),
        crate::preserves_rail::record("to-node", vec![crate::preserves_rail::string(&input.envelope.to_node)]),
        crate::preserves_rail::record("sequence", vec![crate::preserves_rail::string(
            input.envelope.sequence.to_string(),
        )]),
        crate::preserves_rail::record("envelope", vec![crate::preserves_rail::string(&input.envelope.envelope_ref)]),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(&input.envelope.operation_ref)]),
        crate::preserves_rail::record("request", vec![crate::preserves_rail::string(
            &input.envelope.request.request_ref,
        )]),
        crate::preserves_rail::record("idempotency", vec![super::fields::optional_string(
            input.idempotency_receipt_ref,
        )]),
        crate::preserves_rail::record("queue", vec![super::fields::optional_string(input.queue_receipt_ref)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![ingress_check_sequence(input)]),
    ]))
}

fn ingress_check_sequence(input: &IngressReceiptValueInput<'_>) -> preserves::IOValue {
    let has_peer_bootstrap = !input.envelope.peer_bootstrap_refs.is_empty();
    let has_authority = !input.envelope.authority_refs.is_empty() && !input.envelope.request.authority_refs.is_empty();
    let has_policy = !input.envelope.policy_refs.is_empty() && !input.envelope.request.policy_refs.is_empty();
    let has_resource = !input.envelope.resource_refs.is_empty() && !input.envelope.request.resource_refs.is_empty();
    crate::preserves_rail::sequence(vec![
        super::stamps::receipt_check_value("peer-bootstrap-bound", super::stamps::pass_if(has_peer_bootstrap)),
        super::stamps::receipt_check_value("authority-before-enqueue", super::stamps::pass_if(has_authority)),
        super::stamps::receipt_check_value(
            "authority-delegation-before-enqueue",
            super::stamps::pass_if(
                input.envelope.transport != super::settings::LIVE_CONTROL_INGRESS_TRANSPORT || input.decision == "pass",
            ),
        ),
        super::stamps::receipt_check_value("policy-before-enqueue", super::stamps::pass_if(has_policy)),
        super::stamps::receipt_check_value("resource-before-enqueue", super::stamps::pass_if(has_resource)),
        super::stamps::receipt_check_value(
            "delivery-idempotency-before-enqueue",
            super::stamps::pass_if(
                input.phase == "publish" || input.idempotency_receipt_ref.is_some() || input.decision == "deny",
            ),
        ),
        super::stamps::receipt_check_value("durable-inbox-boundary", "pass"),
    ])
}

pub fn control_ingress_envelope(
    input: &super::requests::ControlIngressEnvelopeInput<'_>,
) -> crate::error::Result<super::transits::ControlIngressEnvelope> {
    control_ingress_envelope_for_transport(
        input,
        super::settings::LOCAL_CONTROL_INGRESS_TRANSPORT,
        "iroh-local-ingress",
    )
}

pub fn control_live_ingress_envelope(
    input: &super::requests::ControlIngressEnvelopeInput<'_>,
) -> crate::error::Result<super::transits::ControlIngressEnvelope> {
    control_ingress_envelope_for_transport(input, super::settings::LIVE_CONTROL_INGRESS_TRANSPORT, "live-iroh-gossip")
}

fn control_ingress_envelope_for_transport(
    input: &super::requests::ControlIngressEnvelopeInput<'_>,
    transport: &str,
    transport_check: &str,
) -> crate::error::Result<super::transits::ControlIngressEnvelope> {
    let request = crate::node_runtime::parse_control_request(input.request_value)?;
    super::storage::validate_node_id(input.from_peer)?;
    super::storage::validate_node_id(input.to_node)?;
    super::storage::validate_node_id(input.topic)?;
    super::storage::validate_node_id(transport)?;
    super::guards::validate_ingress_refs(input.peer_bootstrap_refs, "node control ingress peer bootstrap ref")?;
    super::guards::validate_ingress_refs(input.authority_refs, "node control ingress authority ref")?;
    super::guards::validate_ingress_refs(input.policy_refs, "node control ingress policy ref")?;
    super::guards::validate_ingress_refs(input.resource_refs, "node control ingress resource ref")?;
    super::guards::validate_ingress_refs(input.evidence_refs, "node control ingress evidence ref")?;
    let scope_ref = crate::delivery_idempotency::remote_topic_scope_ref(input.topic, input.to_node)?;
    let operation = crate::delivery_idempotency::derive_operation_id(crate::delivery_idempotency::OperationIdInput {
        scope_ref,
        producer: input.from_peer.to_string(),
        consumer: input.to_node.to_string(),
        sequence: input.sequence,
        intent: "node-control-ingress".to_string(),
        payload_ref: request.request_ref.clone(),
        policy_refs: input.policy_refs.to_vec(),
    })?;
    let value = super::ingresses::ingress_envelope_value(
        input,
        &request,
        &operation.operation_ref,
        transport,
        transport_check,
    )?;
    super::delivery::parse_control_ingress_envelope(&value)
}
