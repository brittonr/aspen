//! Live send ticket, state-root evidence, and grant diagnostics.

pub(super) fn live_send_ticket_diagnostics(
    input: &super::offers::ControlLiveSendInput<'_>,
    ticket: &super::entitlements::ControlLiveTicket,
) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(3);
    if let Some(expected) = input.expected_receiver_node
        && ticket.node_id != expected
    {
        diagnostics
            .push(format!("node control live send ticket node {} does not match expected {expected}", ticket.node_id));
    }
    if let Some(expected) = input.expected_topic
        && ticket.topic != expected
    {
        diagnostics
            .push(format!("node control live send ticket topic {} does not match expected {expected}", ticket.topic));
    }
    if let Some(expected) = input.expected_endpoint
        && ticket.live_endpoint_id != expected
    {
        diagnostics.push(format!(
            "node control live send ticket endpoint {} does not match expected {expected}",
            ticket.live_endpoint_id
        ));
    }
    diagnostics
}

pub(super) fn live_send_state_root_evidence_diagnostics(
    state_root: &crate::node_state::DirectoryRoot,
    input: &super::offers::ControlLiveSendInput<'_>,
    envelope: &super::transits::ControlIngressEnvelope,
) -> crate::error::Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(
        input.peer_bootstrap_refs.len().saturating_add(input.authority_refs.len()).saturating_add(4),
    );
    if input.peer_bootstrap_refs.is_empty() {
        diagnostics.push(
            "node control live send peer admission refs missing; run live-ticket-import --peer-admission before live send"
                .to_string(),
        );
    } else {
        let peer_diagnostics = super::newcomers::evaluate_live_peer_bootstrap(state_root, envelope)?;
        if !peer_diagnostics.is_empty() {
            diagnostics.extend(peer_diagnostics);
            diagnostics.push(
                "node control live send peer admission unavailable in sender state root; run live-ticket-import --peer-admission before live send"
                    .to_string(),
            );
        }
    }
    if input.authority_refs.is_empty() || envelope.request.authority_refs.is_empty() {
        diagnostics.push(
            "node control live send authority grant refs missing; run authority-grant-import before live send"
                .to_string(),
        );
    } else {
        let authority_diagnostics = live_send_authority_grant_diagnostics(state_root, envelope)?;
        if !authority_diagnostics.is_empty() {
            diagnostics.extend(authority_diagnostics);
            diagnostics.push(
                "node control live send authority grant unavailable in sender state root; run authority-grant-import before live send"
                    .to_string(),
            );
        }
    }
    Ok(diagnostics)
}

pub(super) fn live_send_authority_grant_diagnostics(
    state_root: &crate::node_state::DirectoryRoot,
    envelope: &super::transits::ControlIngressEnvelope,
) -> crate::error::Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(envelope.authority_refs.len().saturating_add(2));
    let mut has_candidate_authority = false;
    let mut has_admitted_grant = false;
    for authority_ref in envelope
        .authority_refs
        .iter()
        .filter(|authority_ref| envelope.request.authority_refs.contains(*authority_ref))
    {
        has_candidate_authority = true;
        match super::storage::read_ledger_artifact(state_root, authority_ref) {
            Ok(value) => match super::issuance::parse_control_authority_grant(&value) {
                Ok(grant) => {
                    let grant_diagnostics = super::mandates::authority_grant_diagnostics(envelope, &grant);
                    if grant_diagnostics.is_empty() {
                        has_admitted_grant = true;
                        break;
                    }
                    diagnostics.extend(grant_diagnostics);
                }
                Err(error) => {
                    if let Some(diagnostic) = super::newcomers::transport_evidence_not_authority_diagnostic(
                        &value,
                        authority_ref,
                        "node control live send authority ref",
                        "authority",
                    ) {
                        diagnostics.push(diagnostic);
                    } else {
                        diagnostics.push(format!(
                            "node control live send authority ref {authority_ref} is not a grant: {error}"
                        ));
                    }
                }
            },
            Err(error) => diagnostics.push(format!(
                "node control live send authority grant {authority_ref} not found in sender state root: {error}"
            )),
        }
    }
    if !has_candidate_authority {
        diagnostics.push("node control live send authority refs are not bound to the request".to_string());
    }
    if !has_admitted_grant {
        diagnostics.push("node control live send authority delegation missing admitted grant".to_string());
    }
    Ok(diagnostics)
}

pub(super) fn live_ticket_endpoint_addr(
    ticket: &super::entitlements::ControlLiveTicket,
) -> crate::error::Result<iroh::EndpointAddr> {
    let endpoint_id = ticket
        .live_endpoint_id
        .strip_prefix("iroh:")
        .ok_or_else(|| {
            crate::error::Failure::invalid_harness("node control live ticket endpoint must use iroh: prefix")
        })?
        .parse::<iroh::EndpointId>()
        .map_err(|error| {
            crate::error::Failure::invalid_harness(format!("node control live ticket endpoint parse failed: {error}"))
        })?;
    let mut addrs = Vec::with_capacity(ticket.address_refs.len());
    for address_ref in &ticket.address_refs {
        let addr = if let Some(ip_addr) = address_ref.strip_prefix("ip:") {
            iroh::TransportAddr::Ip(crate::fabric_transport::parse_ip_hint(ip_addr)?)
        } else if let Some(relay_url) = address_ref.strip_prefix("relay:") {
            iroh::TransportAddr::Relay(relay_url.parse::<iroh::RelayUrl>().map_err(|error| {
                crate::error::Failure::invalid_harness(format!(
                    "node control live ticket relay address parse failed: {error}"
                ))
            })?)
        } else {
            return Err(crate::error::Failure::invalid_harness(format!(
                "node control live ticket unsupported transport address {address_ref}"
            )));
        };
        addrs.push(addr);
    }
    Ok(iroh::EndpointAddr::from_parts(endpoint_id, addrs))
}
