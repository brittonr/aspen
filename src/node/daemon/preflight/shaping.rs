//! Live topology and transport profile preflight.

const LIVE_PROFILE_RELAY_DIRECT: &str = "direct";

const LIVE_PROFILE_RELAY_RELAY: &str = "relay";

pub(super) const LIVE_PROFILE_RELAY_AUTO: &str = "auto";

pub(super) fn validate_live_topology_profile(
    profile: &super::offers::LiveTopologyProfile<'_>,
) -> crate::error::Result<()> {
    super::guards::validate_ingress_ref(profile.profile_ref, "live topology profile ref")?;
    super::storage::validate_node_id(profile.expected_node)?;
    super::storage::validate_node_id(profile.expected_peer)?;
    super::storage::validate_node_id(profile.expected_topic)?;
    if let Some(endpoint) = profile.expected_endpoint {
        super::storage::validate_node_id(endpoint)?;
    }
    if profile.allowed_alpns.is_empty() {
        return Err(crate::error::Failure::invalid_harness("live topology profile allowed ALPNs must not be empty"));
    }
    for alpn in profile.allowed_alpns {
        if alpn.trim().is_empty() {
            return Err(crate::error::Failure::invalid_harness("live topology profile ALPN must not be empty"));
        }
    }
    super::guards::validate_ingress_refs(profile.ticket_refs, "live topology profile ticket ref")?;
    super::guards::validate_ingress_refs(profile.peer_admission_refs, "live topology profile peer admission ref")?;
    if let Some(role) = profile.role
        && role.trim().is_empty()
    {
        return Err(crate::error::Failure::invalid_harness("live topology profile role must not be empty"));
    }
    Ok(())
}

pub(super) fn validate_live_transport_profile_shape(
    profile: &super::offers::LiveTransportProfile<'_>,
) -> crate::error::Result<()> {
    super::guards::validate_ingress_ref(profile.profile_ref, "live transport profile ref")?;
    if !matches!(
        profile.relay_preference,
        LIVE_PROFILE_RELAY_DIRECT | LIVE_PROFILE_RELAY_RELAY | LIVE_PROFILE_RELAY_AUTO
    ) {
        return Err(crate::error::Failure::invalid_harness(format!(
            "live transport profile relay preference {} is not supported",
            profile.relay_preference
        )));
    }
    Ok(())
}

pub fn preflight_live_profiles(
    input: &super::offers::ControlLiveSendInput<'_>,
) -> crate::error::Result<super::offers::LiveProfilePreflight> {
    let ticket = super::issuance::parse_control_live_ticket(input.receiver_ticket_value)?;
    let envelope = super::vetting::send_envelope(input, &ticket)?;
    live_send_profile_preflight(super::offers::LiveProfilePreflightInput {
        send: input,
        ticket: &ticket,
        envelope: &envelope,
    })
}

pub(super) fn live_send_profile_preflight(
    input: super::offers::LiveProfilePreflightInput<'_>,
) -> crate::error::Result<super::offers::LiveProfilePreflight> {
    let mut diagnostics = Vec::with_capacity(super::settings::LIVE_PROFILE_DIAGNOSTIC_CAPACITY);
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
    let effective_max_attempts = super::tuning::effective_live_send_max_attempts(input.send);
    let effective_join_timeout_ms = super::tuning::effective_live_send_join_timeout_ms(input.send);
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    Ok(super::offers::LiveProfilePreflight {
        decision: decision.to_string(),
        topology_profile_ref: super::tuning::selected_topology_profile_ref(input.send).map(ToOwned::to_owned),
        transport_profile_ref: super::tuning::selected_transport_profile_ref(input.send).map(ToOwned::to_owned),
        effective_max_attempts,
        effective_join_timeout_ms,
        diagnostics,
    })
}

fn live_topology_profile_diagnostics(
    profile: &super::offers::LiveTopologyProfile<'_>,
    send: &super::offers::ControlLiveSendInput<'_>,
    ticket: &super::entitlements::ControlLiveTicket,
    envelope: &super::transits::ControlIngressEnvelope,
) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(super::settings::LIVE_PROFILE_DIAGNOSTIC_CAPACITY);
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
        diagnostics.push(format!("live topology profile does not admit ticket ref {}", ticket.ticket_ref));
    }
    if !profile.allowed_alpns.iter().any(|alpn| live_profile_alpn_matches(alpn)) {
        diagnostics
            .push(format!("live topology profile ALPNs {:?} do not admit live Iroh gossip", profile.allowed_alpns));
    }
    diagnostics
}

fn live_transport_profile_diagnostics(profile: &super::offers::LiveTransportProfile<'_>) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(super::settings::LIVE_PROFILE_DIAGNOSTIC_CAPACITY);
    if profile.max_attempts == 0 {
        diagnostics.push("live transport profile max attempts must be positive".to_string());
    }
    if profile.max_attempts > super::settings::MAX_CONTROL_LIVE_SEND_ATTEMPTS {
        diagnostics.push(format!(
            "live transport profile attempts {} exceed hard cap {MAX_CONTROL_LIVE_SEND_ATTEMPTS}",
            profile.max_attempts,
            MAX_CONTROL_LIVE_SEND_ATTEMPTS = super::settings::MAX_CONTROL_LIVE_SEND_ATTEMPTS
        ));
    }
    if profile.join_timeout_ms == 0 {
        diagnostics.push("live transport profile join timeout must be positive".to_string());
    }
    if profile.join_timeout_ms > super::settings::MAX_CONTROL_LIVE_SEND_TIMEOUT_MS {
        diagnostics.push(format!(
            "live transport profile join timeout {} exceeds hard cap {MAX_CONTROL_LIVE_SEND_TIMEOUT_MS}",
            profile.join_timeout_ms,
            MAX_CONTROL_LIVE_SEND_TIMEOUT_MS = super::settings::MAX_CONTROL_LIVE_SEND_TIMEOUT_MS
        ));
    }
    if profile.publish_timeout_ms == 0 {
        diagnostics.push("live transport profile publish timeout must be positive".to_string());
    }
    if profile.publish_timeout_ms > super::settings::MAX_CONTROL_LIVE_SEND_TIMEOUT_MS {
        diagnostics.push(format!(
            "live transport profile publish timeout {} exceeds hard cap {MAX_CONTROL_LIVE_SEND_TIMEOUT_MS}",
            profile.publish_timeout_ms,
            MAX_CONTROL_LIVE_SEND_TIMEOUT_MS = super::settings::MAX_CONTROL_LIVE_SEND_TIMEOUT_MS
        ));
    }
    diagnostics
}

fn live_profile_non_authority_diagnostics(input: &super::offers::ControlLiveSendInput<'_>) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(super::settings::LIVE_PROFILE_DIAGNOSTIC_CAPACITY);
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
    alpn == super::settings::LIVE_CONTROL_INGRESS_TRANSPORT
        || std::str::from_utf8(iroh_gossip::ALPN).is_ok_and(|live_alpn| alpn == live_alpn)
}
