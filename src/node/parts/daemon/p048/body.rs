
fn preflight_control_live_send_with_root(
    input: &ControlLiveSendInput<'_>,
    state_root: Option<&crate::node_state::NodeStateRoot>,
) -> Result<ControlLiveSendPreflight> {
    validate_live_send_request(input, state_root)?;
    let ticket = parse_control_live_ticket(input.receiver_ticket_value)?;
    let envelope = control_live_ingress_envelope(&ControlIngressEnvelopeInput {
        request_value: input.request_value,
        from_peer: input.from_peer,
        to_node: &ticket.node_id,
        topic: &ticket.topic,
        sequence: input.sequence,
        peer_bootstrap_refs: input.peer_bootstrap_refs,
        authority_refs: input.authority_refs,
        policy_refs: input.policy_refs,
        resource_refs: input.resource_refs,
        evidence_refs: input.evidence_refs,
    })?;
    let mut diagnostics = Vec::with_capacity(8);
    if let Some(operation_ref) = input.expected_operation_ref
        && operation_ref != envelope.operation_ref
    {
        diagnostics.push(format!(
            "node control live send operation-id {operation_ref} does not match derived {}",
            envelope.operation_ref
        ));
    }
    diagnostics.extend(live_send_ticket_diagnostics(input, &ticket));
    let profile = live_send_profile_preflight(LiveProfilePreflightInput {
        send: input,
        ticket: &ticket,
        envelope: &envelope,
    })?;
    diagnostics.extend(profile.diagnostics.iter().cloned());
    if let Some(state_root) = state_root {
        diagnostics.extend(live_send_state_root_evidence_diagnostics(state_root, input, &envelope)?);
    }
    if ticket.address_refs.is_empty() {
        diagnostics.push(
            "node control live send ticket has no endpoint addresses; import a bound live ticket with live-ticket-import or use serve --live-ticket-out"
                .to_string(),
        );
    } else if let Err(error) = live_ticket_endpoint_addr(&ticket) {
        diagnostics.push(format!(
            "node control live send ticket address unsupported or malformed: {error}; import a fresh live ticket with live-ticket-import"
        ));
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    Ok(ControlLiveSendPreflight {
        decision: decision.to_string(),
        envelope_ref: envelope.envelope_ref,
        operation_ref: envelope.operation_ref,
        receiver_ticket_ref: ticket.ticket_ref,
        topology_profile_ref: profile.topology_profile_ref,
        transport_profile_ref: profile.transport_profile_ref,
        effective_max_attempts: profile.effective_max_attempts,
        effective_join_timeout_ms: profile.effective_join_timeout_ms,
        diagnostics,
    })
}

/// Validates the state root, the sender, the send bounds, and every expected identity the caller pinned.
fn validate_live_send_request(
    input: &ControlLiveSendInput<'_>,
    state_root: Option<&crate::node_state::NodeStateRoot>,
) -> Result<()> {
    if let Some(path) = input.state_root {
        validate_state_root(path)?;
    }
    if let Some(state_root) = state_root {
        ensure_state_layout(state_root)?;
    }
    validate_node_id(input.from_peer)?;
    validate_live_send_timeout(input.join_timeout_ms)?;
    validate_live_send_attempts(input.max_attempts)?;
    if let Some(operation_ref) = input.expected_operation_ref {
        validate_ingress_ref(operation_ref, "node control live send operation id")?;
    }
    if let Some(node) = input.expected_receiver_node {
        validate_node_id(node)?;
    }
    if let Some(topic) = input.expected_topic {
        validate_node_id(topic)?;
    }
    if let Some(endpoint) = input.expected_endpoint {
        validate_node_id(endpoint)?;
    }
    Ok(())
}
