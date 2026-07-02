
fn live_send_duplicate_receipt_value(input: &LiveSendDuplicateReceiptValueInput<'_>) -> Result<IoValue> {
    Ok(crate::preserves_rail::record("node-control-live-send-duplicate-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_SEND_DUPLICATE_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string("pass")]),
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

fn service_lock_value(input: &ServiceLockValueInput<'_>) -> Result<IoValue> {
    Ok(crate::preserves_rail::record("node-control-service-lock-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_SERVICE_LOCK_SCHEMA),
        crate::preserves_rail::record("state-root", vec![crate::preserves_rail::string(&state_root_profile_ref(
            input.state_root,
        )?)]),
        crate::preserves_rail::record("startup", vec![crate::preserves_rail::string(input.startup_receipt_ref)]),
        crate::preserves_rail::record("node", vec![crate::preserves_rail::string(input.node_id)]),
        crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(input.topic)]),
        crate::preserves_rail::record("max-ticks", vec![crate::preserves_rail::string(input.max_ticks.to_string())]),
        crate::preserves_rail::record("max-requests-per-tick", vec![crate::preserves_rail::string(
            input.max_requests_per_tick.to_string(),
        )]),
        crate::preserves_rail::record("service-run", vec![crate::preserves_rail::string(input.service_run_ref)]),
        crate::preserves_rail::record("profile", vec![crate::preserves_rail::string(
            "local-supervised-node-control-v1",
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("startup-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("single-active-service"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bounded-ticks"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("not-authority-token"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn service_heartbeat_receipt_value(input: &ServiceHeartbeatValueInput<'_>) -> Result<IoValue> {
    Ok(crate::preserves_rail::record("node-control-service-heartbeat-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_SERVICE_HEARTBEAT_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(
            if input.diagnostics.is_empty() { "pass" } else { "deny" },
        )]),
        crate::preserves_rail::record("startup", vec![crate::preserves_rail::string(input.startup_receipt_ref)]),
        crate::preserves_rail::record("service-lock", vec![crate::preserves_rail::string(input.service_lock_ref)]),
        crate::preserves_rail::record("tick", vec![crate::preserves_rail::string(input.tick.to_string())]),
        crate::preserves_rail::record("delivered-count", vec![crate::preserves_rail::string(
            input.delivered_count.to_string(),
        )]),
        crate::preserves_rail::record("processed-count", vec![crate::preserves_rail::string(
            input.processed_count.to_string(),
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("service-lock-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("startup-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("monotonic-tick"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn supervisor_receipt_value(input: &SupervisorReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    Ok(crate::preserves_rail::record("node-control-supervisor-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_SUPERVISOR_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(input.operation)]),
        crate::preserves_rail::record("startup", vec![crate::preserves_rail::string(input.startup_receipt_ref)]),
        crate::preserves_rail::record("service-lock", vec![optional_string(input.service_lock_ref)]),
        crate::preserves_rail::record("policy", vec![optional_string(input.supervisor_policy_ref)]),
        crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(input.topic)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("supervisor-policy-bound"),
                crate::preserves_rail::string(if input.supervisor_policy_ref.is_some() {
                    "pass"
                } else {
                    "fail"
                }),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("single-active-service"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bounded-restart-policy"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("shutdown-drain-bound"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn service_run_check_sequence(input: &ServiceRunReceiptValueInput<'_>) -> IoValue {
    let has_supervisor_policy_binding =
        input.supervisor_policy_ref.is_none() || !input.supervisor_receipt_refs.is_empty();
    crate::preserves_rail::sequence(vec![
        receipt_check_value("single-active-service", pass_if(input.service_lock_ref.is_some())),
        receipt_check_value("ingress-before-loop", "pass"),
        receipt_check_value("loop-reuse", "pass"),
        receipt_check_value("shutdown-stop-semantics", "pass"),
        receipt_check_value("bounded-ticks", "pass"),
        receipt_check_value("supervisor-policy-bound", pass_if(has_supervisor_policy_binding)),
    ])
}

fn service_run_receipt_value(input: &ServiceRunReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    Ok(crate::preserves_rail::record("node-control-service-run-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_SERVICE_RUN_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("startup", vec![crate::preserves_rail::string(input.startup_receipt_ref)]),
        crate::preserves_rail::record("service-lock", vec![optional_string(input.service_lock_ref)]),
        crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(input.topic)]),
        crate::preserves_rail::record("max-ticks", vec![crate::preserves_rail::string(input.max_ticks.to_string())]),
        crate::preserves_rail::record("max-requests-per-tick", vec![crate::preserves_rail::string(
            input.max_requests_per_tick.to_string(),
        )]),
        crate::preserves_rail::record("ticks", vec![crate::preserves_rail::string(input.ticks.to_string())]),
        crate::preserves_rail::record("heartbeats", vec![crate::preserves_rail::sequence(
            input.heartbeat_receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("ingress-receipts", vec![crate::preserves_rail::sequence(
            input.ingress_receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("loop-receipts", vec![crate::preserves_rail::sequence(
            input.loop_receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("processed-requests", vec![crate::preserves_rail::sequence(
            input.processed_request_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("stopped", vec![crate::preserves_rail::string(if input.has_stopped {
            "true"
        } else {
            "false"
        })]),
        crate::preserves_rail::record("supervisor-policy", vec![optional_string(input.supervisor_policy_ref)]),
        crate::preserves_rail::record("supervisor-receipts", vec![crate::preserves_rail::sequence(
            input.supervisor_receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![service_run_check_sequence(input)]),
    ]))
}

fn ingress_envelope_value(
    input: &ControlIngressEnvelopeInput<'_>,
    request: &crate::node_runtime::ControlRequest,
    operation_ref: &str,
    transport: &str,
    transport_check: &str,
) -> Result<IoValue> {
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

fn ingress_receipt_value(input: &IngressReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
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
        crate::preserves_rail::record("idempotency", vec![optional_string(input.idempotency_receipt_ref)]),
        crate::preserves_rail::record("queue", vec![optional_string(input.queue_receipt_ref)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![ingress_check_sequence(input)]),
    ]))
}
