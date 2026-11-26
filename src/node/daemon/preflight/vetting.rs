//! Live send preflight and send input checks.

pub fn preflight_control_live_send(
    input: &super::offers::ControlLiveSendInput<'_>,
) -> crate::error::Result<super::transits::ControlLiveSendPreflight> {
    let state_root = input.state_root.map(crate::node_state::DirectoryRoot::open).transpose()?;
    preflight_control_live_send_with_root(input, state_root.as_ref())
}

pub(super) fn preflight_control_live_send_with_root(
    input: &super::offers::ControlLiveSendInput<'_>,
    state_root: Option<&crate::node_state::DirectoryRoot>,
) -> crate::error::Result<super::transits::ControlLiveSendPreflight> {
    validate_live_send_request(input, state_root)?;
    let ticket = super::issuance::parse_control_live_ticket(input.receiver_ticket_value)?;
    let envelope = super::ingresses::control_live_ingress_envelope(&super::requests::ControlIngressEnvelopeInput {
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
    diagnostics.extend(super::admissibility::live_send_ticket_diagnostics(input, &ticket));
    let profile = super::shaping::live_send_profile_preflight(super::offers::LiveProfilePreflightInput {
        send: input,
        ticket: &ticket,
        envelope: &envelope,
    })?;
    diagnostics.extend(profile.diagnostics.iter().cloned());
    if let Some(state_root) = state_root {
        diagnostics
            .extend(super::admissibility::live_send_state_root_evidence_diagnostics(state_root, input, &envelope)?);
    }
    if ticket.address_refs.is_empty() {
        diagnostics.push(
            "node control live send ticket has no endpoint addresses; import a bound live ticket with live-ticket-import or use serve --live-ticket-out"
                .to_string(),
        );
    } else if let Err(error) = super::admissibility::live_ticket_endpoint_addr(&ticket) {
        diagnostics.push(format!(
            "node control live send ticket address unsupported or malformed: {error}; import a fresh live ticket with live-ticket-import"
        ));
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    Ok(super::transits::ControlLiveSendPreflight {
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

/// Validates the state root, the sender, the send bounds, and every expected identity the caller
/// pinned.
fn validate_live_send_request(
    input: &super::offers::ControlLiveSendInput<'_>,
    state_root: Option<&crate::node_state::DirectoryRoot>,
) -> crate::error::Result<()> {
    if let Some(path) = input.state_root {
        super::storage::validate_state_root(path)?;
    }
    if let Some(state_root) = state_root {
        super::storage::ensure_state_layout(state_root)?;
    }
    super::storage::validate_node_id(input.from_peer)?;
    super::guards::validate_live_send_timeout(input.join_timeout_ms)?;
    super::guards::validate_live_send_attempts(input.max_attempts)?;
    if let Some(operation_ref) = input.expected_operation_ref {
        super::guards::validate_ingress_ref(operation_ref, "node control live send operation id")?;
    }
    if let Some(node) = input.expected_receiver_node {
        super::storage::validate_node_id(node)?;
    }
    if let Some(topic) = input.expected_topic {
        super::storage::validate_node_id(topic)?;
    }
    if let Some(endpoint) = input.expected_endpoint {
        super::storage::validate_node_id(endpoint)?;
    }
    Ok(())
}

pub(super) fn validate_send_input(
    input: &super::offers::ControlLiveSendInput<'_>,
    state_root: Option<&crate::node_state::DirectoryRoot>,
) -> crate::error::Result<()> {
    if let Some(path) = input.state_root {
        super::storage::validate_state_root(path)?;
    }
    if let Some(state_root) = state_root {
        super::storage::ensure_state_layout(state_root)?;
    }
    super::storage::validate_node_id(input.from_peer)?;
    super::guards::validate_live_send_timeout(input.join_timeout_ms)?;
    super::guards::validate_live_send_attempts(input.max_attempts)?;
    if let Some(operation_ref) = input.expected_operation_ref {
        super::guards::validate_ingress_ref(operation_ref, "node control live send operation id")?;
    }
    if let Some(node) = input.expected_receiver_node {
        super::storage::validate_node_id(node)?;
    }
    if let Some(topic) = input.expected_topic {
        super::storage::validate_node_id(topic)?;
    }
    if let Some(endpoint) = input.expected_endpoint {
        super::storage::validate_node_id(endpoint)?;
    }
    if let Some(profile) = input.topology_profile {
        super::shaping::validate_live_topology_profile(profile)?;
    }
    if let Some(profile) = input.transport_profile {
        super::shaping::validate_live_transport_profile_shape(profile)?;
    }
    Ok(())
}

pub(super) fn send_envelope(
    input: &super::offers::ControlLiveSendInput<'_>,
    ticket: &super::entitlements::ControlLiveTicket,
) -> crate::error::Result<super::transits::ControlIngressEnvelope> {
    super::ingresses::control_live_ingress_envelope(&super::requests::ControlIngressEnvelopeInput {
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
    })
}

pub(super) fn send_receiver_addr(
    input: &super::offers::ControlLiveSendInput<'_>,
    state_root: Option<&crate::node_state::DirectoryRoot>,
    ticket: &super::entitlements::ControlLiveTicket,
    envelope: &super::transits::ControlIngressEnvelope,
) -> crate::error::Result<std::result::Result<iroh::EndpointAddr, Vec<String>>> {
    let mut diagnostics = super::admissibility::live_send_ticket_diagnostics(input, ticket);
    let profile = super::shaping::live_send_profile_preflight(super::offers::LiveProfilePreflightInput {
        send: input,
        ticket,
        envelope,
    })?;
    diagnostics.extend(profile.diagnostics);
    if let Some(state_root) = state_root {
        diagnostics
            .extend(super::admissibility::live_send_state_root_evidence_diagnostics(state_root, input, envelope)?);
    }
    if ticket.address_refs.is_empty() {
        diagnostics.push(
            "node control live send ticket has no endpoint addresses; import a bound live ticket with live-ticket-import or use serve --live-ticket-out"
                .to_string(),
        );
        return Ok(Err(diagnostics));
    }
    match super::admissibility::live_ticket_endpoint_addr(ticket) {
        Ok(addr) if diagnostics.is_empty() => Ok(Ok(addr)),
        Ok(_) => Ok(Err(diagnostics)),
        Err(error) => {
            diagnostics.push(format!(
                "node control live send ticket address unsupported or malformed: {error}; import a fresh live ticket with live-ticket-import"
            ));
            Ok(Err(diagnostics))
        }
    }
}
