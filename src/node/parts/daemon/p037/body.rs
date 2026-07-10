
fn validate_live_topology_profile(profile: &LiveTopologyProfile<'_>) -> Result<()> {
    validate_ingress_ref(profile.profile_ref, "live topology profile ref")?;
    validate_node_id(profile.expected_node)?;
    validate_node_id(profile.expected_peer)?;
    validate_node_id(profile.expected_topic)?;
    if let Some(endpoint) = profile.expected_endpoint {
        validate_node_id(endpoint)?;
    }
    if profile.allowed_alpns.is_empty() {
        return Err(MoltenError::invalid_harness("live topology profile allowed ALPNs must not be empty"));
    }
    for alpn in profile.allowed_alpns {
        if alpn.trim().is_empty() {
            return Err(MoltenError::invalid_harness("live topology profile ALPN must not be empty"));
        }
    }
    validate_ingress_refs(profile.ticket_refs, "live topology profile ticket ref")?;
    validate_ingress_refs(profile.peer_admission_refs, "live topology profile peer admission ref")?;
    if let Some(role) = profile.role
        && role.trim().is_empty()
    {
        return Err(MoltenError::invalid_harness("live topology profile role must not be empty"));
    }
    Ok(())
}

fn validate_live_transport_profile_shape(profile: &LiveTransportProfile<'_>) -> Result<()> {
    validate_ingress_ref(profile.profile_ref, "live transport profile ref")?;
    if !matches!(
        profile.relay_preference,
        LIVE_PROFILE_RELAY_DIRECT | LIVE_PROFILE_RELAY_RELAY | LIVE_PROFILE_RELAY_AUTO
    ) {
        return Err(MoltenError::invalid_harness(format!(
            "live transport profile relay preference {} is not supported",
            profile.relay_preference
        )));
    }
    Ok(())
}

pub fn preflight_live_profiles(input: &ControlLiveSendInput<'_>) -> Result<LiveProfilePreflight> {
    let ticket = parse_control_live_ticket(input.receiver_ticket_value)?;
    let envelope = send_envelope(input, &ticket)?;
    live_send_profile_preflight(LiveProfilePreflightInput {
        send: input,
        ticket: &ticket,
        envelope: &envelope,
    })
}

fn live_send_profile_preflight(input: LiveProfilePreflightInput<'_>) -> Result<LiveProfilePreflight> {
    let mut diagnostics = Vec::with_capacity(LIVE_PROFILE_DIAGNOSTIC_CAPACITY);
    if let Some(profile) = input.send.topology_profile {
        validate_live_topology_profile(profile)?;
        diagnostics.extend(live_topology_profile_diagnostics(profile, input.send, input.ticket, input.envelope));
    }
    if let Some(profile) = input.send.transport_profile {
        validate_live_transport_profile_shape(profile)?;
        diagnostics.extend(live_transport_profile_diagnostics(profile));
    }
    if input.send.topology_profile.is_some() || input.send.transport_profile.is_some() {
        diagnostics.extend(live_profile_non_authority_diagnostics(input.send));
    }
    let effective_max_attempts = effective_live_send_max_attempts(input.send);
    let effective_join_timeout_ms = effective_live_send_join_timeout_ms(input.send);
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    Ok(LiveProfilePreflight {
        decision: decision.to_string(),
        topology_profile_ref: selected_topology_profile_ref(input.send).map(ToOwned::to_owned),
        transport_profile_ref: selected_transport_profile_ref(input.send).map(ToOwned::to_owned),
        effective_max_attempts,
        effective_join_timeout_ms,
        diagnostics,
    })
}

fn live_topology_profile_diagnostics(
    profile: &LiveTopologyProfile<'_>,
    send: &ControlLiveSendInput<'_>,
    ticket: &ControlLiveTicket,
    envelope: &ControlIngressEnvelope,
) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(LIVE_PROFILE_DIAGNOSTIC_CAPACITY);
    if profile.expected_node != ticket.node_id {
        diagnostics.push(format!(
            "live topology profile node {} does not match ticket node {}",
            profile.expected_node, ticket.node_id
        ));
    }
    if profile.expected_peer != send.from_peer {
        diagnostics.push(format!(
            "live topology profile peer {} does not match sender peer {}",
            profile.expected_peer, send.from_peer
        ));
    }
    if profile.expected_topic != ticket.topic || profile.expected_topic != envelope.topic {
        diagnostics.push(format!(
            "live topology profile topic {} does not match ticket/envelope topic {}/{}",
            profile.expected_topic, ticket.topic, envelope.topic
        ));
    }
    if let Some(expected_endpoint) = profile.expected_endpoint
        && expected_endpoint != ticket.live_endpoint_id
    {
        diagnostics.push(format!(
            "live topology profile endpoint {expected_endpoint} does not match ticket endpoint {}",
            ticket.live_endpoint_id
        ));
    }
    if !profile.ticket_refs.is_empty() && !profile.ticket_refs.iter().any(|reference| reference == &ticket.ticket_ref) {
        diagnostics.push(format!(
            "live topology profile does not admit ticket ref {}",
            ticket.ticket_ref
        ));
    }
    if !profile.allowed_alpns.iter().any(|alpn| live_profile_alpn_matches(alpn)) {
        diagnostics.push(format!(
            "live topology profile ALPNs {:?} do not admit live Iroh gossip",
            profile.allowed_alpns
        ));
    }
    diagnostics
}

fn live_transport_profile_diagnostics(profile: &LiveTransportProfile<'_>) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(LIVE_PROFILE_DIAGNOSTIC_CAPACITY);
    if profile.max_attempts == 0 {
        diagnostics.push("live transport profile max attempts must be positive".to_string());
    }
    if profile.max_attempts > MAX_CONTROL_LIVE_SEND_ATTEMPTS {
        diagnostics.push(format!(
            "live transport profile attempts {} exceed hard cap {MAX_CONTROL_LIVE_SEND_ATTEMPTS}",
            profile.max_attempts
        ));
    }
    if profile.join_timeout_ms == 0 {
        diagnostics.push("live transport profile join timeout must be positive".to_string());
    }
    if profile.join_timeout_ms > MAX_CONTROL_LIVE_SEND_TIMEOUT_MS {
        diagnostics.push(format!(
            "live transport profile join timeout {} exceeds hard cap {MAX_CONTROL_LIVE_SEND_TIMEOUT_MS}",
            profile.join_timeout_ms
        ));
    }
    if profile.publish_timeout_ms == 0 {
        diagnostics.push("live transport profile publish timeout must be positive".to_string());
    }
    if profile.publish_timeout_ms > MAX_CONTROL_LIVE_SEND_TIMEOUT_MS {
        diagnostics.push(format!(
            "live transport profile publish timeout {} exceeds hard cap {MAX_CONTROL_LIVE_SEND_TIMEOUT_MS}",
            profile.publish_timeout_ms
        ));
    }
    diagnostics
}

fn live_profile_non_authority_diagnostics(input: &ControlLiveSendInput<'_>) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(LIVE_PROFILE_DIAGNOSTIC_CAPACITY);
    if input.peer_bootstrap_refs.is_empty() {
        diagnostics.push("live profile evidence is not peer bootstrap authority".to_string());
    }
    if input.authority_refs.is_empty() {
        diagnostics.push("live profile evidence is not operation authority".to_string());
    }
    if input.policy_refs.is_empty() {
        diagnostics.push("live profile evidence is not policy admission".to_string());
    }
    if input.resource_refs.is_empty() {
        diagnostics.push("live profile evidence is not resource authority".to_string());
    }
    diagnostics
}

fn live_profile_alpn_matches(alpn: &str) -> bool {
    alpn == LIVE_CONTROL_INGRESS_TRANSPORT
        || std::str::from_utf8(iroh_gossip::ALPN).is_ok_and(|live_alpn| alpn == live_alpn)
}

fn selected_topology_profile_ref<'a>(input: &'a ControlLiveSendInput<'a>) -> Option<&'a str> {
    input.topology_profile.map(|profile| profile.profile_ref)
}

fn selected_transport_profile_ref<'a>(input: &'a ControlLiveSendInput<'a>) -> Option<&'a str> {
    input.transport_profile.map(|profile| profile.profile_ref)
}

fn effective_live_send_max_attempts(input: &ControlLiveSendInput<'_>) -> u64 {
    input.transport_profile.map_or(input.max_attempts, |profile| profile.max_attempts)
}

fn effective_live_send_join_timeout_ms(input: &ControlLiveSendInput<'_>) -> u64 {
    input
        .transport_profile
        .map_or(input.join_timeout_ms, |profile| profile.join_timeout_ms)
}

fn selected_apply_topology_profile_ref<'a>(input: &'a ControlLiveWorkflowBundleApplyInput<'a>) -> Option<&'a str> {
    input.topology_profile.map(|profile| profile.profile_ref)
}

fn selected_apply_transport_profile_ref<'a>(input: &'a ControlLiveWorkflowBundleApplyInput<'a>) -> Option<&'a str> {
    input.transport_profile.map(|profile| profile.profile_ref)
}

fn effective_live_apply_max_attempts(input: &ControlLiveWorkflowBundleApplyInput<'_>) -> u64 {
    input.transport_profile.map_or(input.max_attempts, |profile| profile.max_attempts)
}

fn effective_live_apply_join_timeout_ms(input: &ControlLiveWorkflowBundleApplyInput<'_>) -> u64 {
    input
        .transport_profile
        .map_or(input.join_timeout_ms, |profile| profile.join_timeout_ms)
}

fn live_profile_ref_records(topology_profile_ref: Option<&str>, transport_profile_ref: Option<&str>) -> IoValue {
    let caveat = if topology_profile_ref.is_none() && transport_profile_ref.is_none() {
        "explicit-flags-no-profile"
    } else {
        "reviewed-live-profile-input"
    };
    crate::preserves_rail::record("live-profiles", vec![
        crate::preserves_rail::record("topology", vec![optional_string(topology_profile_ref)]),
        crate::preserves_rail::record("transport", vec![optional_string(transport_profile_ref)]),
        crate::preserves_rail::record("caveat", vec![crate::preserves_rail::string(caveat)]),
    ])
}

fn live_effective_transport_record(max_attempts: u64, join_timeout_ms: u64) -> IoValue {
    crate::preserves_rail::record("effective-transport", vec![
        crate::preserves_rail::record("max-attempts", vec![crate::preserves_rail::string(max_attempts.to_string())]),
        crate::preserves_rail::record("join-timeout-ms", vec![crate::preserves_rail::string(join_timeout_ms.to_string())]),
    ])
}

fn live_effective_transport_optional_record(max_attempts: Option<u64>, join_timeout_ms: Option<u64>) -> IoValue {
    crate::preserves_rail::record("effective-transport", vec![
        crate::preserves_rail::record(
            "max-attempts",
            vec![optional_string(max_attempts.as_ref().map(ToString::to_string).as_deref())],
        ),
        crate::preserves_rail::record(
            "join-timeout-ms",
            vec![optional_string(join_timeout_ms.as_ref().map(ToString::to_string).as_deref())],
        ),
    ])
}
