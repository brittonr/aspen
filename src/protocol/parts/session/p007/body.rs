
fn parse_local_branches_record(value: &Value<IoValue>) -> Result<Vec<ProtocolLocalBranch>> {
    let fields = value
        .collect_simple_record("branches", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected protocol local branch record"))?;
    parse_local_branches(&fields[0])
}

fn parse_local_branches(value: &Value<IoValue>) -> Result<Vec<ProtocolLocalBranch>> {
    let values = value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("expected protocol local branch sequence"))?;
    ensure_count_at_most(values.len(), MAX_PROTOCOL_ITEMS, "protocol local branches")?;
    let mut branches = Vec::with_capacity(values.len());
    for branch in values.iter() {
        let fields = branch
            .collect_simple_record("branch", Some(2))
            .ok_or_else(|| MoltenError::invalid_harness("expected protocol local branch"))?;
        branches.push(ProtocolLocalBranch {
            label: required_string(&fields[0], "protocol local branch label")?,
            actions: parse_local_action_sequence(&fields[1])?,
        });
    }
    Ok(branches)
}

fn install_receipt(
    manifest: &ProtocolManifest,
    registries: &ProtocolRegistries,
    endpoints: Vec<ProtocolEndpoint>,
    decision: &str,
    diagnostics: Vec<String>,
) -> Result<ProtocolInstallReceipt> {
    let mut endpoint_values = Vec::with_capacity(endpoints.len());
    for endpoint in &endpoints {
        endpoint_values.push(endpoint.value.clone());
    }
    let value = record("protocol-install-receipt-v1", vec![
        string(PROTOCOL_INSTALL_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("manifest", vec![manifest.value.clone()]),
        registry_value("role-registry", &registries.roles),
        registry_value("label-registry", &registries.labels),
        registry_value("payload-registry", &registries.payloads),
        record("endpoints", vec![sequence(endpoint_values)]),
        record("policy", vec![refs_sequence(&manifest.policy_refs)]),
        record("capability", vec![refs_sequence(&manifest.capability_refs)]),
        record("resource", vec![refs_sequence(&manifest.resource_refs)]),
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value(&[
            "trellis-projectability",
            "endpoint-projection",
            "install-receipt-binding",
        ]),
    ]);
    parse_protocol_install_receipt(&value)
}

fn registry_value(label: &str, entries: &[RegistryEntry]) -> IoValue {
    let mut values = Vec::with_capacity(entries.len());
    for entry in entries {
        values.push(record("entry", vec![string(&entry.name), u64_value(u64::from(entry.id))]));
    }
    if label == "role-registry" {
        return record("role-registry", vec![sequence(values)]);
    }
    if label == "label-registry" {
        return record("label-registry", vec![sequence(values)]);
    }
    record("payload-registry", vec![sequence(values)])
}

fn parse_registry(value: &Value<IoValue>, label: &str) -> Result<Vec<RegistryEntry>> {
    let values = field_sequence(value, label)?;
    let mut entries = Vec::with_capacity(values.len());
    for entry in values.iter() {
        let fields = entry
            .collect_simple_record("entry", Some(2))
            .ok_or_else(|| MoltenError::invalid_harness("expected protocol registry entry"))?;
        entries.push(RegistryEntry {
            name: required_string(&fields[0], "registry entry name")?,
            id: u32::try_from(required_u64(&fields[1], "registry entry id")?)
                .map_err(|error| MoltenError::invalid_harness(format!("registry id out of range: {error}")))?,
        });
    }
    Ok(entries)
}

fn parse_endpoint_sequence(value: &Value<IoValue>) -> Result<Vec<ProtocolEndpoint>> {
    let values = field_sequence(value, "endpoints")?;
    let mut endpoints = Vec::with_capacity(values.len());
    for endpoint in values.iter() {
        endpoints.push(parse_protocol_endpoint(&value_to_iovalue(endpoint))?);
    }
    Ok(endpoints)
}

fn operation_receipt_value(input: &OperationReceiptValueInput<'_>) -> Result<IoValue> {
    validate_refs(input.authority_refs, "protocol operation authority ref")?;
    validate_refs(input.resource_refs, "protocol operation resource ref")?;
    validate_refs(input.carrier_refs, "protocol operation carrier ref")?;
    Ok(record("protocol-operation-receipt-v1", vec![
        string(PROTOCOL_OPERATION_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("protocol", vec![string(input.protocol_ref)]),
        record("session", vec![string(input.session_id)]),
        record("role", vec![string(input.role)]),
        record("prior-state", vec![string(input.prior_state_ref)]),
        record("message", vec![optional_ref_value(input.message_ref)]),
        record("next-state", vec![optional_ref_value(input.next_state_ref)]),
        record("sequence", vec![u64_value(input.sequence)]),
        record("authority", vec![refs_sequence(input.authority_refs)]),
        record("resource", vec![refs_sequence(input.resource_refs)]),
        record("carrier", vec![refs_sequence(input.carrier_refs)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(&[
            "projected-local-state",
            "sequence-window",
            "decision-before-side-effects",
        ]),
    ]))
}

pub fn evaluate_protocol_endpoint_transition(
    input: ProtocolEndpointTransitionInput<'_>,
) -> Result<ProtocolEndpointTransitionDecision> {
    validate_name(input.operation, "protocol endpoint operation")?;
    validate_name(input.label, "protocol endpoint transition label")?;
    if let Some(peer) = input.peer {
        validate_name(peer, "protocol endpoint transition peer")?;
    }
    if let Some(payload_tag) = input.payload_tag {
        validate_name(payload_tag, "protocol endpoint transition payload tag")?;
    }
    let mut diagnostics = Vec::new();
    let (next_local_state, seen_message_refs) = match input.operation {
        "send" => transition_send(input, &mut diagnostics)?,
        "receive" => transition_receive(input, &mut diagnostics)?,
        "branch" => transition_branch(input, &mut diagnostics)?,
        "offer" => transition_offer(input, &mut diagnostics)?,
        _ => {
            diagnostics.push(PROTOCOL_TRANSITION_UNSUPPORTED_OPERATION.to_string());
            (None, input.prior.seen_message_refs.clone())
        }
    };
    if let (Some(next), Some(expected_local_state)) = (input.next, next_local_state.as_ref()) {
        validate_transition_next_state(input.prior, next, expected_local_state, &seen_message_refs, &mut diagnostics);
    }
    let decision = if diagnostics.is_empty() && next_local_state.is_some() {
        "pass"
    } else {
        "deny"
    };
    Ok(ProtocolEndpointTransitionDecision {
        decision: decision.to_string(),
        diagnostics,
        next_local_state,
        seen_message_refs,
    })
}

fn transition_send(
    input: ProtocolEndpointTransitionInput<'_>,
    diagnostics: &mut Vec<String>,
) -> Result<(Option<ProtocolLocalState>, Vec<String>)> {
    let Some(action) = input.prior.local_state.actions.first() else {
        diagnostics.push(PROTOCOL_TRANSITION_SEND_EXPECTED.to_string());
        return Ok((None, input.prior.seen_message_refs.clone()));
    };
    let peer = input.peer.unwrap_or_default();
    let payload_tag = input.payload_tag.unwrap_or_default();
    if action.direction != "send" || action.peer != peer || action.label != input.label || action.payload_tag != payload_tag {
        diagnostics.push(format!("{PROTOCOL_TRANSITION_SEND_MISMATCH} label={}", action.label));
        return Ok((None, input.prior.seen_message_refs.clone()));
    }
    if let Some(message) = input.message {
        validate_send_message(input.prior, message, peer, input.label, payload_tag, diagnostics);
    }
    Ok((
        Some(consume_first_action(&input.prior.local_state)?),
        input.prior.seen_message_refs.clone(),
    ))
}

fn transition_receive(
    input: ProtocolEndpointTransitionInput<'_>,
    diagnostics: &mut Vec<String>,
) -> Result<(Option<ProtocolLocalState>, Vec<String>)> {
    let Some(message) = input.message else {
        diagnostics.push(PROTOCOL_TRANSITION_MESSAGE_MISSING.to_string());
        return Ok((None, input.prior.seen_message_refs.clone()));
    };
    if input.prior.seen_message_refs.iter().any(|reference| reference == &message.message_ref) {
        diagnostics.push("duplicate protocol message replay".to_string());
        return Ok((None, input.prior.seen_message_refs.clone()));
    }
    let Some(action) = input.prior.local_state.actions.first() else {
        diagnostics.push(PROTOCOL_TRANSITION_RECEIVE_EXPECTED.to_string());
        return Ok((None, input.prior.seen_message_refs.clone()));
    };
    let expected = ExpectedReceive {
        peer: &action.peer,
        label: &action.label,
        payload_tag: &action.payload_tag,
    };
    if action.direction != "recv" || !message_matches(message, input.prior, expected) {
        diagnostics.push(PROTOCOL_TRANSITION_RECEIVE_MISMATCH.to_string());
        return Ok((None, input.prior.seen_message_refs.clone()));
    }
    let mut seen = Vec::with_capacity(input.prior.seen_message_refs.len().saturating_add(1));
    seen.extend(input.prior.seen_message_refs.iter().cloned());
    seen.push(message.message_ref.clone());
    Ok((Some(consume_first_action(&input.prior.local_state)?), seen))
}

fn transition_branch(
    input: ProtocolEndpointTransitionInput<'_>,
    diagnostics: &mut Vec<String>,
) -> Result<(Option<ProtocolLocalState>, Vec<String>)> {
    let ProtocolLocalTerminal::InternalChoice(branches) = &input.prior.local_state.terminal else {
        diagnostics.push(PROTOCOL_TRANSITION_BRANCH_EXPECTED.to_string());
        return Ok((None, input.prior.seen_message_refs.clone()));
    };
    let Some(branch) = branch_for_label(branches, input.label) else {
        diagnostics.push(PROTOCOL_TRANSITION_BRANCH_MISSING.to_string());
        return Ok((None, input.prior.seen_message_refs.clone()));
    };
    Ok((
        Some(ProtocolLocalState {
            actions: branch.actions.clone(),
            terminal: ProtocolLocalTerminal::End,
        }),
        input.prior.seen_message_refs.clone(),
    ))
}

fn transition_offer(
    input: ProtocolEndpointTransitionInput<'_>,
    diagnostics: &mut Vec<String>,
) -> Result<(Option<ProtocolLocalState>, Vec<String>)> {
    let ProtocolLocalTerminal::Offer { from_role, branches } = &input.prior.local_state.terminal else {
        diagnostics.push(PROTOCOL_TRANSITION_OFFER_EXPECTED.to_string());
        return Ok((None, input.prior.seen_message_refs.clone()));
    };
    if let Some(peer) = input.peer
        && peer != from_role
    {
        diagnostics.push(PROTOCOL_TRANSITION_OFFER_EXPECTED.to_string());
        return Ok((None, input.prior.seen_message_refs.clone()));
    }
    let Some(branch) = branch_for_label(branches, input.label) else {
        diagnostics.push(PROTOCOL_TRANSITION_OFFER_MISSING.to_string());
        return Ok((None, input.prior.seen_message_refs.clone()));
    };
    Ok((
        Some(ProtocolLocalState {
            actions: branch.actions.clone(),
            terminal: ProtocolLocalTerminal::End,
        }),
        input.prior.seen_message_refs.clone(),
    ))
}

fn validate_send_message(
    prior: &ProtocolSessionState,
    message: &ProtocolMessage,
    peer: &str,
    label: &str,
    payload_tag: &str,
    diagnostics: &mut Vec<String>,
) {
    if message.protocol_ref != prior.protocol_ref
        || message.session_id != prior.session_id
        || message.from_role != prior.role
        || message.to_role != peer
        || message.label != label
        || message.payload_tag != payload_tag
        || message.sequence != prior.sequence
    {
        diagnostics.push(PROTOCOL_TRANSITION_SEND_MISMATCH.to_string());
    }
}

fn validate_transition_next_state(
    prior: &ProtocolSessionState,
    next: &ProtocolSessionState,
    expected_local_state: &ProtocolLocalState,
    seen_message_refs: &[String],
    diagnostics: &mut Vec<String>,
) {
    if next.protocol_ref != prior.protocol_ref || next.session_id != prior.session_id || next.role != prior.role {
        diagnostics.push(PROTOCOL_TRANSITION_NEXT_BINDING.to_string());
    }
    if next.sequence != prior.sequence.saturating_add(1) {
        diagnostics.push(PROTOCOL_TRANSITION_NEXT_SEQUENCE.to_string());
    }
    if &next.local_state != expected_local_state {
        diagnostics.push(PROTOCOL_TRANSITION_NEXT_STATE.to_string());
    }
    if next.seen_message_refs != seen_message_refs {
        diagnostics.push(PROTOCOL_TRANSITION_SEEN_MESSAGES.to_string());
    }
}

#[derive(Clone, Copy)]
struct OperationGates<'a> {
    authority_refs: &'a [String],
    resource_refs: &'a [String],
    carrier_refs: &'a [String],
}

fn operation_gates<'a>(
    authority_refs: &'a [String],
    resource_refs: &'a [String],
    carrier_refs: &'a [String],
) -> OperationGates<'a> {
    OperationGates {
        authority_refs,
        resource_refs,
        carrier_refs,
    }
}

fn pass_operation(
    operation: &str,
    prior: &ProtocolSessionState,
    message: Option<&ProtocolMessage>,
    next: &ProtocolSessionState,
    gates: OperationGates<'_>,
) -> Result<ProtocolOperationRun> {
    let receipt_value = operation_receipt_value(&OperationReceiptValueInput {
        operation,
        decision: "pass",
        protocol_ref: &prior.protocol_ref,
        session_id: &prior.session_id,
        role: &prior.role,
        prior_state_ref: &prior.state_ref,
        message_ref: message.map(|value| value.message_ref.as_str()),
        next_state_ref: Some(&next.state_ref),
        sequence: prior.sequence,
        authority_refs: gates.authority_refs,
        resource_refs: gates.resource_refs,
        carrier_refs: gates.carrier_refs,
        diagnostics: &[],
    })?;
    Ok(ProtocolOperationRun {
        decision: "pass".to_string(),
        message: message.cloned(),
        next_state: Some(next.clone()),
        receipt: parse_protocol_operation_receipt(&receipt_value)?,
    })
}

fn deny_operation(
    operation: &str,
    prior: &ProtocolSessionState,
    message: Option<&ProtocolMessage>,
    gates: OperationGates<'_>,
    diagnostics: Vec<String>,
) -> Result<ProtocolOperationRun> {
    let receipt_value = operation_receipt_value(&OperationReceiptValueInput {
        operation,
        decision: "deny",
        protocol_ref: &prior.protocol_ref,
        session_id: &prior.session_id,
        role: &prior.role,
        prior_state_ref: &prior.state_ref,
        message_ref: message.map(|value| value.message_ref.as_str()),
        next_state_ref: None,
        sequence: prior.sequence,
        authority_refs: gates.authority_refs,
        resource_refs: gates.resource_refs,
        carrier_refs: gates.carrier_refs,
        diagnostics: &diagnostics,
    })?;
    Ok(ProtocolOperationRun {
        decision: "deny".to_string(),
        message: None,
        next_state: None,
        receipt: parse_protocol_operation_receipt(&receipt_value)?,
    })
}

fn advance_state(
    prior: &ProtocolSessionState,
    local_state: ProtocolLocalState,
    sequence_value: u64,
    seen_message_refs: Vec<String>,
) -> Result<ProtocolSessionState> {
    let local_value = protocol_local_state_value(&local_state)?;
    let state_value = protocol_session_state_value(&ProtocolSessionStateInput {
        protocol_ref: prior.protocol_ref.clone(),
        session_id: prior.session_id.clone(),
        role: prior.role.clone(),
        sequence: sequence_value,
        endpoint: prior.endpoint.value.clone(),
        local_state: local_value,
        seen_message_refs,
        authority_refs: prior.authority_refs.clone(),
        resource_refs: prior.resource_refs.clone(),
    })?;
    parse_protocol_session_state(&state_value)
}

fn consume_first_action(local_state: &ProtocolLocalState) -> Result<ProtocolLocalState> {
    if local_state.actions.is_empty() {
        return Err(MoltenError::invalid_harness("cannot advance local state with no actions"));
    }
    let mut actions = Vec::with_capacity(local_state.actions.len().saturating_sub(1));
    for action in local_state.actions.iter().skip(1) {
        actions.push(action.clone());
    }
    Ok(ProtocolLocalState {
        actions,
        terminal: local_state.terminal.clone(),
    })
}

struct ExpectedReceive<'a> {
    peer: &'a str,
    label: &'a str,
    payload_tag: &'a str,
}

fn message_matches(message: &ProtocolMessage, state: &ProtocolSessionState, expected: ExpectedReceive<'_>) -> bool {
    message.protocol_ref == state.protocol_ref
        && message.session_id == state.session_id
        && message.from_role == expected.peer
        && message.to_role == state.role
        && message.label == expected.label
        && message.payload_tag == expected.payload_tag
        && message.sequence == state.sequence
}

fn admission_diagnostics(authority_refs: &[String], resource_refs: &[String]) -> Result<Vec<String>> {
    validate_refs(authority_refs, "protocol operation authority ref")?;
    validate_refs(resource_refs, "protocol operation resource ref")?;
    if authority_refs.is_empty() {
        return Ok(vec!["missing protocol authority evidence".to_string()]);
    }
    if resource_refs.is_empty() {
        return Ok(vec!["missing protocol resource evidence".to_string()]);
    }
    Ok(Vec::new())
}

fn required_message(run: &ProtocolOperationRun) -> Result<ProtocolMessage> {
    run.message
        .clone()
        .ok_or_else(|| MoltenError::invalid_harness("expected protocol message in pass operation"))
}

fn required_next_state(run: &ProtocolOperationRun) -> Result<ProtocolSessionState> {
    run.next_state
        .clone()
        .ok_or_else(|| MoltenError::invalid_harness("expected next protocol state in pass operation"))
}

fn endpoint_for_role(endpoints: &[ProtocolEndpoint], role: &str) -> Result<ProtocolEndpoint> {
    for endpoint in endpoints {
        if endpoint.role == role {
            return Ok(endpoint.clone());
        }
    }
    Err(MoltenError::invalid_harness(format!("missing endpoint for role {role}")))
}

fn branch_for_label<'a>(branches: &'a [ProtocolLocalBranch], label: &str) -> Option<&'a ProtocolLocalBranch> {
    branches.iter().find(|branch| branch.label == label)
}
