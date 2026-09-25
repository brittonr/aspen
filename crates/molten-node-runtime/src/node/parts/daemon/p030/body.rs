
fn receipt_value(input: &AuthorityReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
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
        crate::preserves_rail::record("grant", vec![optional_string(input.grant_ref)]),
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

fn live_listener_receipt_value(input: &ListenerReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
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
        live_profile_ref_records(None, None),
        live_effective_transport_optional_record(None, None),
    ]))
}

fn live_transport_receipt_value(input: &LiveTransportReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    let has_peer_bootstrap = !input.envelope.peer_bootstrap_refs.is_empty();
    Ok(crate::preserves_rail::record("node-control-live-transport-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_TRANSPORT_RECEIPT_SCHEMA),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(input.operation)]),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("transport", vec![crate::preserves_rail::string(LIVE_CONTROL_INGRESS_TRANSPORT)]),
        crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(&input.envelope.topic)]),
        crate::preserves_rail::record("node", vec![crate::preserves_rail::string(input.node_id)]),
        crate::preserves_rail::record("delivered-from", vec![optional_string(input.delivered_from)]),
        crate::preserves_rail::record("envelope", vec![crate::preserves_rail::string(&input.envelope.envelope_ref)]),
        crate::preserves_rail::record("ingress-receipt", vec![optional_string(input.ingress_receipt_ref)]),
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
        live_profile_ref_records(input.topology_profile_ref, input.transport_profile_ref),
        live_effective_transport_optional_record(input.effective_max_attempts, input.effective_join_timeout_ms),
    ]))
}

fn live_workflow_receipt_value(input: &LiveWorkflowReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
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
        crate::preserves_rail::record("listener-receipt", vec![optional_string(input.listener_receipt_ref)]),
        crate::preserves_rail::record("service-run", vec![crate::preserves_rail::string(input.service_receipt_ref)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![live_workflow_check_sequence(input)]),
    ]))
}

fn live_workflow_check_sequence(input: &LiveWorkflowReceiptValueInput<'_>) -> IoValue {
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

struct LiveSendReceiptChecks {
    has_addresses: bool,
    has_supported_addresses: bool,
    has_expected_ticket_binding: bool,
    has_operation_mismatch: bool,
    has_state_root_evidence: bool,
    has_transport_success: bool,
}

fn pass_if(condition: bool) -> &'static str {
    if condition { "pass" } else { "fail" }
}

fn fail_if(condition: bool) -> &'static str {
    if condition { "fail" } else { "pass" }
}

fn receipt_check_value(name: &str, status: &str) -> IoValue {
    crate::preserves_rail::record("check", vec![
        crate::preserves_rail::string(name),
        crate::preserves_rail::string(status),
    ])
}

fn live_send_receipt_checks(input: &LiveSendReceiptValueInput<'_>) -> LiveSendReceiptChecks {
    let has_addresses = !input.ticket.address_refs.is_empty();
    LiveSendReceiptChecks {
        has_addresses,
        has_operation_mismatch: diagnostics_include(input.diagnostics, "operation-id"),
        has_supported_addresses: has_addresses
            && !diagnostics_include(input.diagnostics, "unsupported transport address")
            && !diagnostics_include(input.diagnostics, "address unsupported or malformed")
            && !diagnostics_include(input.diagnostics, "address parse failed")
            && !diagnostics_include(input.diagnostics, "endpoint parse failed"),
        has_expected_ticket_binding: !diagnostics_include(input.diagnostics, "ticket node")
            && !diagnostics_include(input.diagnostics, "ticket topic")
            && !diagnostics_include(input.diagnostics, "ticket endpoint"),
        has_state_root_evidence: !diagnostics_include(input.diagnostics, "sender state root")
            && !diagnostics_include(input.diagnostics, "peer admission refs missing")
            && !diagnostics_include(input.diagnostics, "authority grant refs missing"),
        has_transport_success: input.transport_receipt_ref.is_some(),
    }
}

fn live_send_check_sequence(checks: &LiveSendReceiptChecks) -> IoValue {
    crate::preserves_rail::sequence(vec![
        receipt_check_value("receiver-ticket-bound", "pass"),
        receipt_check_value("receiver-address-bound", pass_if(checks.has_addresses)),
        receipt_check_value("receiver-address-supported", pass_if(checks.has_supported_addresses)),
        receipt_check_value("receiver-ticket-expected", pass_if(checks.has_expected_ticket_binding)),
        receipt_check_value("operation-id-bound", fail_if(checks.has_operation_mismatch)),
        receipt_check_value("sender-state-root-evidence", pass_if(checks.has_state_root_evidence)),
        receipt_check_value("join-or-publish-succeeded", pass_if(checks.has_transport_success)),
        receipt_check_value("canonical-envelope-ref", "pass"),
        receipt_check_value("live-iroh-gossip", "pass"),
        receipt_check_value("live-profile-is-not-authority", "pass"),
        receipt_check_value("transport-is-not-authority", "pass"),
        receipt_check_value("durable-inbox-boundary", "pass"),
    ])
}

fn live_send_receipt_value(input: &LiveSendReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    let checks = live_send_receipt_checks(input);
    Ok(crate::preserves_rail::record("node-control-live-send-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_SEND_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("transport", vec![crate::preserves_rail::string(LIVE_CONTROL_INGRESS_TRANSPORT)]),
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
        crate::preserves_rail::record("transport-receipt", vec![optional_string(input.transport_receipt_ref)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![live_send_check_sequence(&checks)]),
        live_profile_ref_records(input.topology_profile_ref, input.transport_profile_ref),
        live_effective_transport_record(input.effective_max_attempts, input.effective_join_timeout_ms),
    ]))
}

fn live_send_retry_receipt_value(input: &LiveSendRetryReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    Ok(crate::preserves_rail::record("node-control-live-send-retry-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_SEND_RETRY_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("attempt", vec![crate::preserves_rail::string(input.attempt.to_string())]),
        crate::preserves_rail::record("max-attempts", vec![crate::preserves_rail::string(
            input.max_attempts.to_string(),
        )]),
        crate::preserves_rail::record("transport", vec![crate::preserves_rail::string(LIVE_CONTROL_INGRESS_TRANSPORT)]),
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
