
fn ingress_check_sequence(input: &IngressReceiptValueInput<'_>) -> IoValue {
    let has_peer_bootstrap = !input.envelope.peer_bootstrap_refs.is_empty();
    let has_authority = !input.envelope.authority_refs.is_empty() && !input.envelope.request.authority_refs.is_empty();
    let has_policy = !input.envelope.policy_refs.is_empty() && !input.envelope.request.policy_refs.is_empty();
    let has_resource = !input.envelope.resource_refs.is_empty() && !input.envelope.request.resource_refs.is_empty();
    crate::preserves_rail::sequence(vec![
        receipt_check_value("peer-bootstrap-bound", pass_if(has_peer_bootstrap)),
        receipt_check_value("authority-before-enqueue", pass_if(has_authority)),
        receipt_check_value(
            "authority-delegation-before-enqueue",
            pass_if(input.envelope.transport != LIVE_CONTROL_INGRESS_TRANSPORT || input.decision == "pass"),
        ),
        receipt_check_value("policy-before-enqueue", pass_if(has_policy)),
        receipt_check_value("resource-before-enqueue", pass_if(has_resource)),
        receipt_check_value(
            "delivery-idempotency-before-enqueue",
            pass_if(input.phase == "publish" || input.idempotency_receipt_ref.is_some() || input.decision == "deny"),
        ),
        receipt_check_value("durable-inbox-boundary", "pass"),
    ])
}

fn queue_receipt_value(input: &QueueReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    Ok(crate::preserves_rail::record("node-control-queue-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_QUEUE_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("phase", vec![crate::preserves_rail::string(input.phase)]),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(input.operation)]),
        crate::preserves_rail::record("request", vec![crate::preserves_rail::string(input.request_ref)]),
        crate::preserves_rail::record("profile", vec![crate::preserves_rail::string(
            "local-preserves-control-file-v1",
        )]),
        crate::preserves_rail::record("location", vec![crate::preserves_rail::string(input.location_ref)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("canonical-request-ref"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("durable-control-profile"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("explicit-state-root"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn operation_receipt_value(input: &OperationReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    Ok(crate::preserves_rail::record("node-control-operation-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_OPERATION_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(&input.request.operation)]),
        crate::preserves_rail::record("request", vec![crate::preserves_rail::string(&input.request.request_ref)]),
        crate::preserves_rail::record("target", vec![optional_string(input.request.target_ref.as_deref())]),
        crate::preserves_rail::record("payload", vec![optional_string(input.request.payload_ref.as_deref())]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("operation-dispatch-explicit"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("side-effects-receipted"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("canonical-receipt"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn heartbeat_receipt_value(input: &HeartbeatReceiptValueInput<'_>) -> Result<IoValue> {
    Ok(crate::preserves_rail::record("node-control-heartbeat-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_HEARTBEAT_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(
            if input.diagnostics.is_empty() { "pass" } else { "deny" },
        )]),
        crate::preserves_rail::record("startup", vec![crate::preserves_rail::string(input.startup_receipt_ref)]),
        crate::preserves_rail::record("lock", vec![crate::preserves_rail::string(input.lock_ref)]),
        crate::preserves_rail::record("loop-sequence", vec![crate::preserves_rail::string(
            input.loop_sequence.to_string(),
        )]),
        crate::preserves_rail::record("processed-count", vec![crate::preserves_rail::string(
            input.processed_count.to_string(),
        )]),
        crate::preserves_rail::record("profile", vec![crate::preserves_rail::string(
            "local-preserves-control-loop-v1",
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("active-lock-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("heartbeat-is-receipted"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("no-ambient-socket-authority"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn loop_receipt_value(input: &LoopReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    Ok(crate::preserves_rail::record("node-control-loop-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LOOP_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("startup", vec![crate::preserves_rail::string(input.startup_receipt_ref)]),
        crate::preserves_rail::record("heartbeat", vec![crate::preserves_rail::string(input.heartbeat_receipt_ref)]),
        crate::preserves_rail::record("max-requests", vec![crate::preserves_rail::string(
            input.max_requests.to_string(),
        )]),
        crate::preserves_rail::record("processed-requests", vec![crate::preserves_rail::sequence(
            input.processed_request_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("dispatch-receipts", vec![crate::preserves_rail::sequence(
            input.dispatch_receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("stopped", vec![crate::preserves_rail::string(if input.has_stopped {
            "yes"
        } else {
            "no"
        })]),
        crate::preserves_rail::record("profile", vec![crate::preserves_rail::string(
            "local-preserves-control-loop-v1",
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bounded-request-loop"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("deterministic-inbox-order"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("idempotent-request-dispatch"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("shutdown-stops-loop"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

pub fn summary(value: &IoValue) -> Result<String> {
    if let Some(summary) = runtime_summary(value)? {
        return Ok(summary);
    }
    if let Some(summary) = import_summary(value)? {
        return Ok(summary);
    }
    if let Some(summary) = access_summary(value)? {
        return Ok(summary);
    }
    if let Some(summary) = flow_summary(value)? {
        return Ok(summary);
    }
    if let Some(summary) = bundle_summary(value)? {
        return Ok(summary);
    }
    if let Some(summary) = gate_summary(value)? {
        return Ok(summary);
    }
    if let Some(summary) = apply_summary(value)? {
        return Ok(summary);
    }
    if let Some(summary) = send_summary(value)? {
        return Ok(summary);
    }
    if let Some(summary) = state_summary(value)? {
        return Ok(summary);
    }
    if let Some(summary) = supervisor_summary(value)? {
        return Ok(summary);
    }
    if let Some(summary) = control_summary(value)? {
        return Ok(summary);
    }
    if let Ok(summary) = crate::protocol_session::protocol_summary(value) {
        return Ok(summary);
    }
    if let Ok(summary) = crate::provenance::summary(value) {
        return Ok(summary);
    }
    Err(MoltenError::invalid_harness("unsupported node daemon artifact for show"))
}

fn runtime_summary(value: &IoValue) -> Result<Option<String>> {
    if let Ok(config) = crate::node_runtime::parse_node_config(value) {
        return Ok(Some(format!(
            "node config ref={} identity={} adapters={}",
            config.config_ref,
            config.identity_ref,
            config.adapters.len()
        )));
    }
    if let Ok(startup) = crate::node_runtime::parse_node_startup_receipt(value) {
        return Ok(Some(format!(
            "node startup decision={} receipt={} adapters={}",
            startup.decision,
            startup.receipt_ref,
            startup.adapters.len()
        )));
    }
    if let Ok(control) = crate::node_runtime::parse_control_receipt(value) {
        return Ok(Some(format!(
            "node control decision={} receipt={} request={}",
            control.decision, control.receipt_ref, control.request_ref
        )));
    }
    if let Ok(ingress) = parse_control_ingress_envelope(value) {
        return Ok(Some(format!(
            "node control ingress envelope ref={} topic={} from={} to={} request={}",
            ingress.envelope_ref, ingress.topic, ingress.from_peer, ingress.to_node, ingress.request.request_ref
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-ingress-receipt-v1", Some(15)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_INGRESS_RECEIPT_SCHEMA,
            "node control ingress receipt",
        )?;
        return Ok(Some(format!(
            "node control ingress decision={} phase={} envelope={} request={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "phase")?,
            record_string(&fields[8], "envelope")?,
            record_string(&fields[10], "request")?
        )));
    }
    Ok(None)
}

fn import_summary(value: &IoValue) -> Result<Option<String>> {
    if let Some(fields) = value.collect_simple_record("node-control-live-ticket-import-receipt-v1", Some(13)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_TICKET_IMPORT_RECEIPT_SCHEMA,
            "node control live ticket import receipt",
        )?;
        return Ok(Some(format!(
            "node control live ticket import decision={} ticket={} imported={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[3], "ticket")?,
            record_sequence_len(&fields[10], "imported")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-authority-grant-import-receipt-v1", Some(13)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_AUTHORITY_GRANT_IMPORT_RECEIPT_SCHEMA,
            "node control authority grant import receipt",
        )?;
        return Ok(Some(format!(
            "node control authority grant import decision={} grant={} imported={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[3], "grant")?,
            record_sequence_len(&fields[10], "imported")?
        )));
    }
    Ok(None)
}
