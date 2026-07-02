
fn initial_state_gate_diagnostics(parsed: &ProtocolSessionGateParsed, state: &ProtocolSessionState) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(4);
    if state.protocol_ref != parsed.install.manifest.manifest_ref {
        diagnostics.push(format!("initial state {} protocol does not match install", state.state_ref));
    }
    if state.endpoint.protocol_ref != state.protocol_ref {
        diagnostics.push(format!("initial state {} endpoint protocol mismatch", state.state_ref));
    }
    if !parsed.install.endpoints.iter().any(|endpoint| endpoint.endpoint_ref == state.endpoint.endpoint_ref) {
        diagnostics.push(format!("initial state {} endpoint is not installed", state.state_ref));
    }
    if !parsed.install.manifest.roles.iter().any(|role| role == &state.role) {
        diagnostics.push(format!("initial state {} role is not in manifest", state.state_ref));
    }
    diagnostics
}

fn message_gate_diagnostics(parsed: &ProtocolSessionGateParsed, message: &ProtocolMessage) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(3);
    if message.protocol_ref != parsed.install.manifest.manifest_ref {
        diagnostics.push(format!("protocol message {} protocol does not match install", message.message_ref));
    }
    if !parsed.install.manifest.roles.iter().any(|role| role == &message.from_role) {
        diagnostics.push(format!("protocol message {} sender role is not in manifest", message.message_ref));
    }
    if !parsed.install.manifest.roles.iter().any(|role| role == &message.to_role) {
        diagnostics.push(format!("protocol message {} receiver role is not in manifest", message.message_ref));
    }
    if !parsed.install.manifest.payloads.iter().any(|payload| payload.tag == message.payload_tag) {
        diagnostics.push(format!("protocol message {} payload tag is not declared", message.message_ref));
    }
    diagnostics
}

fn operation_gate_diagnostics(
    parsed: &ProtocolSessionGateParsed,
    receipt: &ProtocolOperationReceipt,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(8);
    if !matches!(receipt.decision.as_str(), "pass" | "deny") {
        diagnostics.push(format!("protocol operation {} has unsupported decision", receipt.receipt_ref));
    }
    if !matches!(receipt.operation.as_str(), "send" | "receive" | "branch" | "offer") {
        diagnostics.push(format!("protocol operation {} has unsupported operation", receipt.receipt_ref));
    }
    if receipt.protocol_ref != parsed.install.manifest.manifest_ref {
        diagnostics.push(format!("protocol operation {} protocol does not match install", receipt.receipt_ref));
    }
    let Some(prior) = find_state(parsed, &receipt.prior_state_ref) else {
        diagnostics.push(format!("protocol operation {} prior state is missing", receipt.receipt_ref));
        return Ok(diagnostics);
    };
    diagnostics.extend(operation_prior_diagnostics(receipt, prior));
    let message = match &receipt.message_ref {
        Some(reference) => match find_message(parsed, reference) {
            Some(message) => Some(message),
            None => {
                diagnostics.push(format!("protocol operation {} message is missing", receipt.receipt_ref));
                None
            }
        },
        None => None,
    };
    if let Some(message) = message {
        diagnostics.extend(operation_message_diagnostics(receipt, prior, message));
    }
    match receipt.decision.as_str() {
        "pass" => diagnostics.extend(pass_operation_gate_diagnostics(parsed, receipt, prior, message)?),
        "deny" => diagnostics.extend(deny_operation_gate_diagnostics(receipt)),
        _ => {}
    }
    Ok(diagnostics)
}

fn operation_prior_diagnostics(receipt: &ProtocolOperationReceipt, prior: &ProtocolSessionState) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(4);
    if receipt.session_id != prior.session_id {
        diagnostics.push(format!("protocol operation {} session does not match prior state", receipt.receipt_ref));
    }
    if receipt.role != prior.role {
        diagnostics.push(format!("protocol operation {} role does not match prior state", receipt.receipt_ref));
    }
    if receipt.sequence != prior.sequence {
        diagnostics.push(format!("protocol operation {} sequence does not match prior state", receipt.receipt_ref));
    }
    diagnostics
}

fn operation_message_diagnostics(
    receipt: &ProtocolOperationReceipt,
    prior: &ProtocolSessionState,
    message: &ProtocolMessage,
) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(4);
    if message.protocol_ref != prior.protocol_ref || message.session_id != prior.session_id {
        diagnostics.push(format!("protocol operation {} message session binding mismatch", receipt.receipt_ref));
    }
    if receipt.operation == "send" && message.from_role != prior.role {
        diagnostics.push(format!("protocol operation {} send message sender mismatch", receipt.receipt_ref));
    }
    if receipt.operation == "receive" && message.to_role != prior.role {
        diagnostics.push(format!("protocol operation {} receive message receiver mismatch", receipt.receipt_ref));
    }
    diagnostics
}

fn pass_operation_gate_diagnostics(
    parsed: &ProtocolSessionGateParsed,
    receipt: &ProtocolOperationReceipt,
    prior: &ProtocolSessionState,
    message: Option<&ProtocolMessage>,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(8);
    let Some(next_ref) = &receipt.next_state_ref else {
        diagnostics.push(format!("protocol operation {} pass is missing next state", receipt.receipt_ref));
        return Ok(diagnostics);
    };
    let Some(next) = find_state(parsed, next_ref) else {
        diagnostics.push(format!("protocol operation {} next state is missing", receipt.receipt_ref));
        return Ok(diagnostics);
    };
    if next.protocol_ref != prior.protocol_ref || next.session_id != prior.session_id || next.role != prior.role {
        diagnostics.push(format!("protocol operation {} next state binding mismatch", receipt.receipt_ref));
    }
    if next.sequence != prior.sequence.saturating_add(1) {
        diagnostics.push(format!("protocol operation {} next sequence is not prior+1", receipt.receipt_ref));
    }
    match replay_protocol_operation(receipt, prior, message, next) {
        Ok(replayed) => diagnostics.extend(replayed_operation_diagnostics(receipt, &replayed)),
        Err(error) => diagnostics.push(format!("protocol operation {} replay failed: {error}", receipt.receipt_ref)),
    }
    Ok(diagnostics)
}

fn deny_operation_gate_diagnostics(receipt: &ProtocolOperationReceipt) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(2);
    if receipt.next_state_ref.is_some() {
        diagnostics.push(format!("protocol operation {} deny unexpectedly has next state", receipt.receipt_ref));
    }
    if receipt.diagnostics.is_empty() {
        diagnostics.push(format!("protocol operation {} deny is missing diagnostics", receipt.receipt_ref));
    }
    diagnostics
}

fn replay_protocol_operation(
    receipt: &ProtocolOperationReceipt,
    prior: &ProtocolSessionState,
    message: Option<&ProtocolMessage>,
    next: &ProtocolSessionState,
) -> Result<ProtocolOperationRun> {
    match receipt.operation.as_str() {
        "send" => {
            let message = message.ok_or_else(|| MoltenError::invalid_harness("send replay requires message"))?;
            let evidence_refs =
                send_evidence_prefix(&message.evidence_refs, &receipt.authority_refs, &receipt.resource_refs)?;
            send_protocol_message(ProtocolSendInput {
                state: prior.value.clone(),
                to_role: message.to_role.clone(),
                label: message.label.clone(),
                payload_tag: message.payload_tag.clone(),
                body_or_ref: message.body_or_ref.clone(),
                authority_refs: receipt.authority_refs.clone(),
                resource_refs: receipt.resource_refs.clone(),
                evidence_refs,
            })
        }
        "receive" => {
            let message = message.ok_or_else(|| MoltenError::invalid_harness("receive replay requires message"))?;
            receive_protocol_message(ProtocolReceiveInput {
                state: prior.value.clone(),
                message: message.value.clone(),
                authority_refs: receipt.authority_refs.clone(),
                resource_refs: receipt.resource_refs.clone(),
                carrier_refs: receipt.carrier_refs.clone(),
            })
        }
        "branch" => choose_protocol_branch(ProtocolBranchOperationInput {
            state: prior.value.clone(),
            label: transition_branch_label(prior, next, "branch")?,
            authority_refs: receipt.authority_refs.clone(),
            resource_refs: receipt.resource_refs.clone(),
            carrier_refs: receipt.carrier_refs.clone(),
        }),
        "offer" => offer_protocol_branch(ProtocolBranchOperationInput {
            state: prior.value.clone(),
            label: transition_branch_label(prior, next, "offer")?,
            authority_refs: receipt.authority_refs.clone(),
            resource_refs: receipt.resource_refs.clone(),
            carrier_refs: receipt.carrier_refs.clone(),
        }),
        value => Err(MoltenError::invalid_harness(format!("unsupported protocol operation replay {value}"))),
    }
}

fn replayed_operation_diagnostics(receipt: &ProtocolOperationReceipt, replayed: &ProtocolOperationRun) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(3);
    if replayed.receipt.receipt_ref != receipt.receipt_ref {
        diagnostics.push(format!("protocol operation {} receipt does not replay", receipt.receipt_ref));
    }
    if replayed.receipt.message_ref != receipt.message_ref {
        diagnostics.push(format!("protocol operation {} message ref does not replay", receipt.receipt_ref));
    }
    if replayed.receipt.next_state_ref != receipt.next_state_ref {
        diagnostics.push(format!("protocol operation {} next state ref does not replay", receipt.receipt_ref));
    }
    diagnostics
}

fn send_evidence_prefix(
    evidence_refs: &[String],
    authority_refs: &[String],
    resource_refs: &[String],
) -> Result<Vec<String>> {
    let suffix_count = authority_refs
        .len()
        .checked_add(resource_refs.len())
        .ok_or_else(|| MoltenError::invalid_harness("protocol evidence suffix overflow"))?;
    if evidence_refs.len() < suffix_count {
        return Err(MoltenError::invalid_harness("protocol message evidence is missing gate refs"));
    }
    let prefix_count = evidence_refs.len() - suffix_count;
    let authority_end = prefix_count + authority_refs.len();
    if &evidence_refs[prefix_count..authority_end] != authority_refs {
        return Err(MoltenError::invalid_harness("protocol message evidence authority suffix mismatch"));
    }
    if &evidence_refs[authority_end..] != resource_refs {
        return Err(MoltenError::invalid_harness("protocol message evidence resource suffix mismatch"));
    }
    Ok(evidence_refs[..prefix_count].to_vec())
}

fn transition_branch_label(
    prior: &ProtocolSessionState,
    next: &ProtocolSessionState,
    operation: &str,
) -> Result<String> {
    let branches = match (operation, &prior.local_state.terminal) {
        ("branch", ProtocolLocalTerminal::InternalChoice(branches)) => branches,
        ("offer", ProtocolLocalTerminal::Offer { branches, .. }) => branches,
        _ => return Err(MoltenError::invalid_harness("protocol state does not contain requested branch shape")),
    };
    let mut matched = Vec::with_capacity(branches.len());
    for branch in branches {
        let candidate = ProtocolLocalState {
            actions: branch.actions.clone(),
            terminal: ProtocolLocalTerminal::End,
        };
        if candidate == next.local_state {
            matched.push(branch.label.clone());
        }
    }
    if matched.len() == 1 {
        return Ok(matched.remove(0));
    }
    Err(MoltenError::invalid_harness("protocol branch transition is ambiguous or missing"))
}

fn terminal_role_diagnostics(parsed: &ProtocolSessionGateParsed) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(parsed.initial_states.len());
    for state in &parsed.initial_states {
        diagnostics.extend(terminal_trace_diagnostics(parsed, state));
    }
    diagnostics
}
