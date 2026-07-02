
pub fn start_protocol_session(
    install: &ProtocolInstallReceipt,
    role: &str,
    session_id: &str,
    authority_refs: Vec<String>,
    resource_refs: Vec<String>,
) -> Result<ProtocolSessionState> {
    if install.decision != "pass" {
        return Err(MoltenError::invalid_harness("cannot start session from denied protocol install"));
    }
    validate_session_id(session_id)?;
    validate_refs(&authority_refs, "protocol session authority ref")?;
    validate_refs(&resource_refs, "protocol session resource ref")?;
    let endpoint = endpoint_for_role(&install.endpoints, role)?;
    let local_value = protocol_local_state_value(&endpoint.local_state)?;
    let state_value = protocol_session_state_value(&ProtocolSessionStateInput {
        protocol_ref: install.manifest.manifest_ref.clone(),
        session_id: session_id.to_string(),
        role: role.to_string(),
        sequence: 0,
        endpoint: endpoint.value.clone(),
        local_state: local_value,
        seen_message_refs: Vec::new(),
        authority_refs,
        resource_refs,
    })?;
    parse_protocol_session_state(&state_value)
}

pub fn protocol_message_value(input: &ProtocolMessageInput) -> Result<IoValue> {
    validate_protocol_ref(&input.protocol_ref, "protocol message protocol ref")?;
    validate_session_id(&input.session_id)?;
    validate_name(&input.from_role, "protocol message from role")?;
    validate_name(&input.to_role, "protocol message to role")?;
    validate_name(&input.label, "protocol message label")?;
    validate_name(&input.payload_tag, "protocol message payload tag")?;
    validate_refs(&input.evidence_refs, "protocol message evidence ref")?;
    Ok(record("protocol-message-v1", vec![
        string(PROTOCOL_MESSAGE_SCHEMA),
        record("protocol", vec![string(&input.protocol_ref)]),
        record("session", vec![string(&input.session_id)]),
        record("from-role", vec![string(&input.from_role)]),
        record("to-role", vec![string(&input.to_role)]),
        record("label", vec![string(&input.label)]),
        record("payload-tag", vec![string(&input.payload_tag)]),
        record("body-or-ref", vec![input.body_or_ref.clone()]),
        record("sequence", vec![u64_value(input.sequence)]),
        record("evidence", vec![refs_sequence(&input.evidence_refs)]),
        checks_value(&["projected-action", "payload-schema-tag", "transport-neutral-payload"]),
    ]))
}

pub fn parse_protocol_message(value: &IoValue) -> Result<ProtocolMessage> {
    let fields = value
        .collect_simple_record("protocol-message-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("expected <protocol-message-v1 ...>"))?;
    require_schema(&fields[0], PROTOCOL_MESSAGE_SCHEMA, "protocol message schema")?;
    let checks = parse_checks(&fields[10])?;
    require_check(&checks, "projected-action", "protocol message")?;
    let protocol_ref = record_ref(&fields[1], "protocol")?;
    let session_id = record_string(&fields[2], "session")?;
    validate_session_id(&session_id)?;
    Ok(ProtocolMessage {
        message_ref: canonical_hash(value)?,
        protocol_ref,
        session_id,
        from_role: record_string(&fields[3], "from-role")?,
        to_role: record_string(&fields[4], "to-role")?,
        label: record_string(&fields[5], "label")?,
        payload_tag: record_string(&fields[6], "payload-tag")?,
        body_or_ref: record_iovalue(&fields[7], "body-or-ref")?,
        sequence: record_u64(&fields[8], "sequence")?,
        evidence_refs: parse_ref_sequence(&fields[9], "evidence")?,
        value: value.clone(),
    })
}

pub fn send_protocol_message(input: ProtocolSendInput) -> Result<ProtocolOperationRun> {
    let state = parse_protocol_session_state(&input.state)?;
    let gates = operation_gates(&input.authority_refs, &input.resource_refs, &[]);
    let diagnostics = admission_diagnostics(&input.authority_refs, &input.resource_refs)?;
    if !diagnostics.is_empty() {
        return deny_operation("send", &state, None, gates, diagnostics);
    }
    let Some(action) = state.local_state.actions.first() else {
        return deny_operation("send", &state, None, gates, vec!["endpoint does not expect send".to_string()]);
    };
    if action.direction != "send" {
        return deny_operation("send", &state, None, gates, vec!["endpoint does not expect send".to_string()]);
    }
    if action.peer != input.to_role || action.label != input.label || action.payload_tag != input.payload_tag {
        return deny_operation("send", &state, None, gates, vec![format!(
            "send does not match projected action label={}",
            action.label
        )]);
    }
    let mut evidence_refs =
        Vec::with_capacity(input.evidence_refs.len() + input.authority_refs.len() + input.resource_refs.len());
    evidence_refs.extend(input.evidence_refs.iter().cloned());
    evidence_refs.extend(input.authority_refs.iter().cloned());
    evidence_refs.extend(input.resource_refs.iter().cloned());
    let message_value = protocol_message_value(&ProtocolMessageInput {
        protocol_ref: state.protocol_ref.clone(),
        session_id: state.session_id.clone(),
        from_role: state.role.clone(),
        to_role: input.to_role,
        label: input.label,
        payload_tag: input.payload_tag,
        body_or_ref: input.body_or_ref,
        sequence: state.sequence,
        evidence_refs,
    })?;
    let message = parse_protocol_message(&message_value)?;
    let next_state = advance_state(
        &state,
        consume_first_action(&state.local_state)?,
        state.sequence + 1,
        state.seen_message_refs.clone(),
    )?;
    pass_operation("send", &state, Some(&message), &next_state, gates)
}

pub fn receive_protocol_message(input: ProtocolReceiveInput) -> Result<ProtocolOperationRun> {
    let state = parse_protocol_session_state(&input.state)?;
    let message = parse_protocol_message(&input.message)?;
    let gates = operation_gates(&input.authority_refs, &input.resource_refs, &input.carrier_refs);
    let diagnostics = admission_diagnostics(&input.authority_refs, &input.resource_refs)?;
    if !diagnostics.is_empty() {
        return deny_operation("receive", &state, Some(&message), gates, diagnostics);
    }
    if state.seen_message_refs.iter().any(|reference| reference == &message.message_ref) {
        return deny_operation("receive", &state, Some(&message), gates, vec![
            "duplicate protocol message replay".to_string(),
        ]);
    }
    let Some(action) = state.local_state.actions.first() else {
        return deny_operation("receive", &state, Some(&message), gates, vec![
            "endpoint does not expect receive".to_string(),
        ]);
    };
    let expected = ExpectedReceive {
        peer: &action.peer,
        label: &action.label,
        payload_tag: &action.payload_tag,
    };
    if action.direction != "recv" || !message_matches(&message, &state, expected) {
        return deny_operation("receive", &state, Some(&message), gates, vec![
            "message does not match projected receive action".to_string(),
        ]);
    }
    let mut seen = Vec::with_capacity(state.seen_message_refs.len() + 1);
    seen.extend(state.seen_message_refs.iter().cloned());
    seen.push(message.message_ref.clone());
    let next_state = advance_state(&state, consume_first_action(&state.local_state)?, state.sequence + 1, seen)?;
    pass_operation("receive", &state, Some(&message), &next_state, gates)
}

pub fn choose_protocol_branch(input: ProtocolBranchOperationInput) -> Result<ProtocolOperationRun> {
    let state = parse_protocol_session_state(&input.state)?;
    let gates = operation_gates(&input.authority_refs, &input.resource_refs, &input.carrier_refs);
    let diagnostics = admission_diagnostics(&input.authority_refs, &input.resource_refs)?;
    if !diagnostics.is_empty() {
        return deny_operation("branch", &state, None, gates, diagnostics);
    }
    let ProtocolLocalTerminal::InternalChoice(branches) = &state.local_state.terminal else {
        return deny_operation("branch", &state, None, gates, vec![
            "endpoint does not expect internal choice".to_string(),
        ]);
    };
    let Some(branch) = branch_for_label(branches, &input.label) else {
        return deny_operation("branch", &state, None, gates, vec![
            "branch label is not offered by projected state".to_string(),
        ]);
    };
    let next_local = ProtocolLocalState {
        actions: branch.actions.clone(),
        terminal: ProtocolLocalTerminal::End,
    };
    let next_state = advance_state(&state, next_local, state.sequence + 1, state.seen_message_refs.clone())?;
    pass_operation("branch", &state, None, &next_state, gates)
}

pub fn offer_protocol_branch(input: ProtocolBranchOperationInput) -> Result<ProtocolOperationRun> {
    let state = parse_protocol_session_state(&input.state)?;
    let gates = operation_gates(&input.authority_refs, &input.resource_refs, &input.carrier_refs);
    let diagnostics = admission_diagnostics(&input.authority_refs, &input.resource_refs)?;
    if !diagnostics.is_empty() {
        return deny_operation("offer", &state, None, gates, diagnostics);
    }
    let ProtocolLocalTerminal::Offer { branches, .. } = &state.local_state.terminal else {
        return deny_operation("offer", &state, None, gates, vec!["endpoint does not expect offer".to_string()]);
    };
    let Some(branch) = branch_for_label(branches, &input.label) else {
        return deny_operation("offer", &state, None, gates, vec!["offer label is not projected".to_string()]);
    };
    let next_local = ProtocolLocalState {
        actions: branch.actions.clone(),
        terminal: ProtocolLocalTerminal::End,
    };
    let next_state = advance_state(&state, next_local, state.sequence + 1, state.seen_message_refs.clone())?;
    pass_operation("offer", &state, None, &next_state, gates)
}

pub fn protocol_message_remote_envelope(input: ProtocolRemoteEnvelopeInput) -> Result<Envelope> {
    let message = parse_protocol_message(&input.message)?;
    build_remote_envelope(EnvelopeInput {
        from_peer: input.from_peer,
        from_actor: input.from_actor,
        to_peer: input.to_peer,
        topic: input.topic,
        operation: Operation::Message,
        payload: message.value,
        content_refs: Vec::new(),
        capability_refs: input.capability_refs,
        evidence_refs: input.evidence_refs,
    })
}

pub fn parse_protocol_install_receipt(value: &IoValue) -> Result<ProtocolInstallReceipt> {
    let fields = value
        .collect_simple_record("protocol-install-receipt-v1", Some(12))
        .ok_or_else(|| MoltenError::invalid_harness("expected <protocol-install-receipt-v1 ...>"))?;
    require_schema(&fields[0], PROTOCOL_INSTALL_RECEIPT_SCHEMA, "protocol install receipt schema")?;
    let manifest_value = record_iovalue(&fields[2], "manifest")?;
    let endpoints = parse_endpoint_sequence(&fields[6])?;
    Ok(ProtocolInstallReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        manifest: parse_protocol_manifest(&manifest_value)?,
        registries: ProtocolRegistries {
            roles: parse_registry(&fields[3], "role-registry")?,
            labels: parse_registry(&fields[4], "label-registry")?,
            payloads: parse_registry(&fields[5], "payload-registry")?,
        },
        endpoints,
        diagnostics: parse_string_sequence(&fields[10], "diagnostics")?,
        value: value.clone(),
    })
}

pub fn parse_protocol_operation_receipt(value: &IoValue) -> Result<ProtocolOperationReceipt> {
    let fields = value
        .collect_simple_record("protocol-operation-receipt-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <protocol-operation-receipt-v1 ...>"))?;
    require_schema(&fields[0], PROTOCOL_OPERATION_RECEIPT_SCHEMA, "protocol operation receipt schema")?;
    Ok(ProtocolOperationReceipt {
        receipt_ref: canonical_hash(value)?,
        operation: record_string(&fields[1], "operation")?,
        decision: record_string(&fields[2], "decision")?,
        protocol_ref: record_ref(&fields[3], "protocol")?,
        session_id: record_string(&fields[4], "session")?,
        role: record_string(&fields[5], "role")?,
        prior_state_ref: record_ref(&fields[6], "prior-state")?,
        message_ref: record_optional_ref(&fields[7], "message")?,
        next_state_ref: record_optional_ref(&fields[8], "next-state")?,
        sequence: record_u64(&fields[9], "sequence")?,
        authority_refs: parse_ref_sequence(&fields[10], "authority")?,
        resource_refs: parse_ref_sequence(&fields[11], "resource")?,
        carrier_refs: parse_ref_sequence(&fields[12], "carrier")?,
        diagnostics: parse_string_sequence(&fields[13], "diagnostics")?,
        value: value.clone(),
    })
}

pub fn gate_protocol_session_lifecycle(input: ProtocolSessionGateInput) -> Result<ProtocolSessionGate> {
    gate_protocol_session_lifecycle_with_diagnostics(input, Vec::new())
}
