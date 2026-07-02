
pub fn gate_protocol_session_lifecycle_with_diagnostics(
    input: ProtocolSessionGateInput,
    extra_diagnostics: Vec<String>,
) -> Result<ProtocolSessionGate> {
    ensure_count_at_most(extra_diagnostics.len(), MAX_PROTOCOL_ITEMS, "protocol gate extra diagnostics")?;
    let parsed = parse_protocol_session_gate_input(input)?;
    let mut diagnostics = protocol_session_gate_diagnostics(&parsed)?;
    diagnostics.extend(extra_diagnostics);
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let initial_state_refs = state_refs(&parsed.initial_states);
    let operation_refs = operation_refs(&parsed.operation_receipts);
    let message_refs = message_refs(&parsed.messages);
    let final_state_refs = terminal_state_refs(&parsed.next_states);
    let session_ids = session_ids(&parsed.initial_states)?;
    let receipt_value = protocol_session_gate_receipt_value(&ProtocolSessionGateValueInput {
        decision,
        install_ref: &parsed.install.receipt_ref,
        protocol_ref: &parsed.install.manifest.manifest_ref,
        session_ids: &session_ids,
        initial_state_refs: &initial_state_refs,
        operation_refs: &operation_refs,
        message_refs: &message_refs,
        final_state_refs: &final_state_refs,
        diagnostics: &diagnostics,
    })?;
    Ok(ProtocolSessionGate {
        receipt_ref: canonical_hash(&receipt_value)?,
        decision: decision.to_string(),
        install_ref: parsed.install.receipt_ref,
        protocol_ref: parsed.install.manifest.manifest_ref,
        session_ids,
        initial_state_count: parsed.initial_states.len(),
        operation_count: parsed.operation_receipts.len(),
        message_count: parsed.messages.len(),
        final_state_count: final_state_refs.len(),
        diagnostics,
        value: receipt_value,
    })
}

pub fn parse_protocol_session_gate_receipt(value: &IoValue) -> Result<ProtocolSessionGateReceipt> {
    let fields = value
        .collect_simple_record("protocol-session-gate-receipt-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("expected <protocol-session-gate-receipt-v1 ...>"))?;
    require_schema(&fields[0], PROTOCOL_SESSION_GATE_RECEIPT_SCHEMA, "protocol session gate receipt schema")?;
    let checks = parse_checks(&fields[10])?;
    require_check(&checks, "protocol-session-gate-is-not-authority", "protocol session gate receipt")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_gate_decision(&decision, "protocol session gate decision")?;
    let session_ids = parse_string_sequence(&fields[4], "sessions")?;
    for session_id in &session_ids {
        validate_session_id(session_id)?;
    }
    Ok(ProtocolSessionGateReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        install_ref: record_ref(&fields[2], "install")?,
        protocol_ref: record_ref(&fields[3], "protocol")?,
        session_ids,
        initial_state_refs: parse_ref_sequence(&fields[5], "initial-states")?,
        operation_refs: parse_ref_sequence(&fields[6], "operations")?,
        message_refs: parse_ref_sequence(&fields[7], "messages")?,
        final_state_refs: parse_ref_sequence(&fields[8], "final-states")?,
        diagnostics: parse_string_sequence(&fields[9], "diagnostics")?,
    })
}

pub fn protocol_summary(value: &IoValue) -> Result<String> {
    if value.collect_simple_record("protocol-install-receipt-v1", Some(12)).is_some() {
        let install = parse_protocol_install_receipt(value)?;
        return Ok(format!(
            "protocol install receipt ref={} decision={} protocol={} endpoints={} diagnostics={}",
            install.receipt_ref,
            install.decision,
            install.manifest.protocol_id,
            install.endpoints.len(),
            install.diagnostics.len()
        ));
    }
    if value.collect_simple_record("protocol-operation-receipt-v1", Some(15)).is_some() {
        let receipt = parse_protocol_operation_receipt(value)?;
        return Ok(format!(
            "protocol operation receipt ref={} decision={} operation={} session={} role={} sequence={}",
            receipt.receipt_ref,
            receipt.decision,
            receipt.operation,
            receipt.session_id,
            receipt.role,
            receipt.sequence
        ));
    }
    if value.collect_simple_record("protocol-session-gate-receipt-v1", Some(11)).is_some() {
        let receipt = parse_protocol_session_gate_receipt(value)?;
        return Ok(format!(
            "protocol session gate receipt ref={} decision={} protocol={} sessions={} operations={} diagnostics={}",
            receipt.receipt_ref,
            receipt.decision,
            receipt.protocol_ref,
            receipt.session_ids.len(),
            receipt.operation_refs.len(),
            receipt.diagnostics.len()
        ));
    }
    Err(MoltenError::invalid_harness("unsupported protocol summary record"))
}

pub fn request_response_manifest_value() -> Result<IoValue> {
    let request_schema = synthetic_ref("request-schema")?;
    let response_schema = synthetic_ref("response-schema")?;
    let policy_ref = synthetic_ref("policy")?;
    let capability_ref = synthetic_ref("capability")?;
    let resource_ref = synthetic_ref("resource")?;
    let global = protocol_global_script_value(&[
        ProtocolCommInput {
            from_role: "client".to_string(),
            to_role: "server".to_string(),
            label: "request".to_string(),
            payload_tag: "request".to_string(),
        },
        ProtocolCommInput {
            from_role: "server".to_string(),
            to_role: "client".to_string(),
            label: "response".to_string(),
            payload_tag: "response".to_string(),
        },
    ])?;
    protocol_manifest_value(&ProtocolManifestInput {
        protocol_id: "proto:request-response".to_string(),
        roles: vec!["client".to_string(), "server".to_string()],
        labels: vec!["request".to_string(), "response".to_string()],
        payloads: vec![
            ProtocolPayloadInput {
                tag: "request".to_string(),
                schema_ref: request_schema,
            },
            ProtocolPayloadInput {
                tag: "response".to_string(),
                schema_ref: response_schema,
            },
        ],
        global,
        policy_refs: vec![policy_ref],
        capability_refs: vec![capability_ref],
        resource_refs: vec![resource_ref],
    })
}

pub fn request_response_lifecycle() -> Result<RequestResponseLifecycle> {
    let manifest_value = request_response_manifest_value()?;
    let install = install_protocol_manifest_value(&manifest_value)?;
    let authority_ref = synthetic_ref("authority")?;
    let resource_ref = synthetic_ref("resource-run")?;
    let client0 =
        start_protocol_session(&install, "client", "session:request-response:1", vec![authority_ref.clone()], vec![
            resource_ref.clone(),
        ])?;
    let server0 =
        start_protocol_session(&install, "server", "session:request-response:1", vec![authority_ref.clone()], vec![
            resource_ref.clone(),
        ])?;
    let send_request = send_protocol_message(ProtocolSendInput {
        state: client0.value.clone(),
        to_role: "server".to_string(),
        label: "request".to_string(),
        payload_tag: "request".to_string(),
        body_or_ref: record("body", vec![string("hello")]),
        authority_refs: vec![authority_ref.clone()],
        resource_refs: vec![resource_ref.clone()],
        evidence_refs: vec![install.receipt_ref.clone()],
    })?;
    let request_message = required_message(&send_request)?;
    let receive_request = receive_protocol_message(ProtocolReceiveInput {
        state: server0.value.clone(),
        message: request_message.value.clone(),
        authority_refs: vec![authority_ref.clone()],
        resource_refs: vec![resource_ref.clone()],
        carrier_refs: Vec::new(),
    })?;
    let server1 = required_next_state(&receive_request)?;
    let send_response = send_protocol_message(ProtocolSendInput {
        state: server1.value.clone(),
        to_role: "client".to_string(),
        label: "response".to_string(),
        payload_tag: "response".to_string(),
        body_or_ref: record("body", vec![string("ok")]),
        authority_refs: vec![authority_ref.clone()],
        resource_refs: vec![resource_ref.clone()],
        evidence_refs: vec![receive_request.receipt.receipt_ref.clone()],
    })?;
    let response_message = required_message(&send_response)?;
    let client1 = required_next_state(&send_request)?;
    let receive_response = receive_protocol_message(ProtocolReceiveInput {
        state: client1.value.clone(),
        message: response_message.value.clone(),
        authority_refs: vec![authority_ref],
        resource_refs: vec![resource_ref],
        carrier_refs: Vec::new(),
    })?;
    Ok(RequestResponseLifecycle {
        manifest_value,
        install,
        initial_states: vec![client0, server0],
        operations: vec![send_request, receive_request, send_response, receive_response],
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestResponseLifecycle {
    pub manifest_value: IoValue,
    pub install: ProtocolInstallReceipt,
    pub initial_states: Vec<ProtocolSessionState>,
    pub operations: Vec<ProtocolOperationRun>,
}

fn parse_protocol_session_gate_input(input: ProtocolSessionGateInput) -> Result<ProtocolSessionGateParsed> {
    ensure_count_at_most(input.initial_states.len(), MAX_PROTOCOL_ITEMS, "protocol gate initial states")?;
    ensure_count_at_most(input.operation_receipts.len(), MAX_PROTOCOL_STEPS, "protocol gate operations")?;
    ensure_count_at_most(input.messages.len(), MAX_PROTOCOL_STEPS, "protocol gate messages")?;
    ensure_count_at_most(input.next_states.len(), MAX_PROTOCOL_STEPS, "protocol gate next states")?;
    Ok(ProtocolSessionGateParsed {
        install: parse_protocol_install_receipt(&input.install_receipt)?,
        initial_states: parse_protocol_states(&input.initial_states)?,
        operation_receipts: parse_protocol_operation_receipts(&input.operation_receipts)?,
        messages: parse_protocol_messages(&input.messages)?,
        next_states: parse_protocol_states(&input.next_states)?,
    })
}

fn parse_protocol_states(values: &[IoValue]) -> Result<Vec<ProtocolSessionState>> {
    let mut states = Vec::with_capacity(values.len());
    for value in values {
        states.push(parse_protocol_session_state(value)?);
    }
    Ok(states)
}

fn parse_protocol_operation_receipts(values: &[IoValue]) -> Result<Vec<ProtocolOperationReceipt>> {
    let mut receipts = Vec::with_capacity(values.len());
    for value in values {
        receipts.push(parse_protocol_operation_receipt(value)?);
    }
    Ok(receipts)
}

fn parse_protocol_messages(values: &[IoValue]) -> Result<Vec<ProtocolMessage>> {
    let mut messages = Vec::with_capacity(values.len());
    for value in values {
        messages.push(parse_protocol_message(value)?);
    }
    Ok(messages)
}

fn protocol_session_gate_diagnostics(parsed: &ProtocolSessionGateParsed) -> Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(8);
    if parsed.install.decision != "pass" {
        diagnostics.push("protocol session gate requires a passing install receipt".to_string());
    }
    match install_protocol_manifest(&parsed.install.manifest) {
        Ok(recomputed) => {
            if recomputed.receipt_ref != parsed.install.receipt_ref {
                diagnostics.push("protocol install receipt does not replay from manifest".to_string());
            }
        }
        Err(error) => diagnostics.push(format!("protocol install replay failed: {error}")),
    }
    if parsed.initial_states.is_empty() {
        diagnostics.push("protocol session gate requires initial state evidence".to_string());
    }
    if parsed.operation_receipts.is_empty() {
        diagnostics.push("protocol session gate requires operation receipt evidence".to_string());
    }
    for state in &parsed.initial_states {
        diagnostics.extend(initial_state_gate_diagnostics(parsed, state));
    }
    for message in &parsed.messages {
        diagnostics.extend(message_gate_diagnostics(parsed, message));
    }
    for receipt in &parsed.operation_receipts {
        diagnostics.extend(operation_gate_diagnostics(parsed, receipt)?);
    }
    diagnostics.extend(terminal_role_diagnostics(parsed));
    Ok(diagnostics)
}
