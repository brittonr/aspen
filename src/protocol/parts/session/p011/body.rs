
fn validate_transition_next_state(
    prior: &ProtocolSessionState,
    next: &ProtocolSessionState,
    expected_local_state: &ProtocolLocalState,
    seen_message_refs: &[String],
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) {
    if next.protocol_ref != prior.protocol_ref || next.session_id != prior.session_id || next.role != prior.role {
        diagnostics.push_item(PROTOCOL_TRANSITION_NEXT_BINDING.to_string());
    }
    if next.sequence != prior.sequence.saturating_add(1) {
        diagnostics.push_item(PROTOCOL_TRANSITION_NEXT_SEQUENCE.to_string());
    }
    if &next.local_state != expected_local_state {
        diagnostics.push_item(PROTOCOL_TRANSITION_NEXT_STATE.to_string());
    }
    if next.seen_message_refs != seen_message_refs {
        diagnostics.push_item(PROTOCOL_TRANSITION_SEEN_MESSAGES.to_string());
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
