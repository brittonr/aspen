//! Stable live endpoint identity, tickets for bound endpoints, and gossip setup.

pub(super) struct EndpointIdentityInput<'a> {
    pub(super) node_id: &'a str,
    pub(super) logical_endpoint_id: &'a str,
    pub(super) bound_endpoint_id: &'a str,
}

pub(super) async fn serve_node_control_live_listener_with_topic(
    state_root: &crate::node_state::DirectoryRoot,
    input: &super::requests::ControlLiveServeInput<'_>,
    receiver: &mut iroh_gossip::api::GossipTopic,
    endpoint: EndpointIdentityInput<'_>,
) -> crate::error::Result<super::transits::ControlLiveServe> {
    let EndpointIdentityInput {
        node_id,
        logical_endpoint_id,
        bound_endpoint_id,
    } = endpoint;
    super::guards::validate_listener_event_limit(input.max_events)?;
    super::guards::validate_loop_request_limit(input.max_requests_per_tick)?;
    let startup = super::leases::current_startup_receipt(state_root)?;
    let mut scan = super::listening::scan_events(state_root, input, receiver, node_id).await?;
    let service_input = super::requests::ControlServeInput {
        state_root: input.state_root,
        topic: input.topic,
        max_ticks: 1,
        max_requests_per_tick: input.max_requests_per_tick,
        supervisor_policy_value: input.supervisor_policy_value,
    };
    let service = super::serving::serve_control_with_root(state_root, &service_input)?;
    if service.decision != "pass" {
        scan.diagnostics
            .push(format!("node control live listener service drain decision {}", service.decision));
    }
    let decision = if scan.diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value =
        super::transports::live_listener_receipt_value(&super::transports::ListenerReceiptValueInput {
            decision,
            startup_receipt_ref: &startup.receipt_ref,
            node_id,
            logical_endpoint_id,
            bound_endpoint_id,
            topic: input.topic,
            max_events: input.max_events,
            observed_events: scan.observed_events,
            transport_receipt_refs: &scan.transport_receipt_refs,
            neighbor_events: &scan.neighbor_events,
            service_receipt_ref: &service.service_receipt_ref,
            diagnostics: &scan.diagnostics,
        })?;
    let listener_receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    super::storage::write_preserves(
        state_root,
        &super::paths::control_live_listener_receipt_path(&listener_receipt_ref)?,
        &receipt_value,
    )?;
    super::storage::import_artifact(state_root, &receipt_value)?;
    Ok(super::transits::ControlLiveServe {
        listener_receipt_ref,
        listener_receipt_value: receipt_value,
        service,
        transport_receipt_refs: scan.transport_receipt_refs,
        neighbor_events: scan.neighbor_events,
        observed_events: scan.observed_events,
        bound_endpoint_id: bound_endpoint_id.to_string(),
        live_ticket_ref: None,
        live_ticket_value: None,
    })
}

pub(super) fn stable_live_endpoint_secret(
    state_root: &crate::node_state::DirectoryRoot,
    identity: &crate::node_identity::Identity,
) -> crate::error::Result<iroh::SecretKey> {
    let identity_namespace = state_root.identity()?;
    crate::fabric_crypto_identity::load_transport_secret_for(
        &identity_namespace,
        &identity.endpoint_id,
        &identity.secret_ref,
        &identity.backend_ref,
    )
}

pub(super) fn stable_live_endpoint_id(identity: &crate::node_identity::Identity) -> String {
    identity.endpoint_id.clone()
}

/// Only exchanged-hint-domain IP addresses are exported; relay addresses pass through.
fn live_ticket_address_refs(addr: &iroh::EndpointAddr) -> Vec<String> {
    addr.addrs
        .iter()
        .filter(|address| crate::fabric_transport::is_exportable_hint(address))
        .map(ToString::to_string)
        .collect()
}

pub(super) fn live_ticket_for_bound_endpoint(
    state_root: &crate::node_state::DirectoryRoot,
    identity: &crate::node_identity::Identity,
    topic: &str,
    addr: &iroh::EndpointAddr,
) -> crate::error::Result<super::entitlements::ControlLiveTicket> {
    let address_refs = live_ticket_address_refs(addr);
    let value = super::issuance::control_live_ticket_value(&super::offers::ControlLiveTicketInput {
        node_id: &identity.node_id,
        identity_ref: &identity.identity_ref,
        logical_endpoint_id: &identity.endpoint_id,
        live_endpoint_id: &format!("iroh:{}", addr.id),
        topic,
        address_refs: &address_refs,
        policy_refs: &identity.policy_refs,
        evidence_refs: &identity.receipt_refs,
    })?;
    let ticket = super::issuance::parse_control_live_ticket(&value)?;
    super::storage::import_artifact(state_root, &value)?;
    Ok(ticket)
}

pub(super) async fn live_gossip_endpoint(
    lookup: &iroh::address_lookup::memory::MemoryLookup,
    secret_key: Option<iroh::SecretKey>,
) -> crate::error::Result<iroh::Endpoint> {
    let mut builder = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .relay_mode(iroh::RelayMode::Disabled)
        .address_lookup(lookup.clone())
        .alpns(vec![iroh_gossip::ALPN.to_vec()])
        .clear_ip_transports()
        .bind_addr((std::net::Ipv4Addr::LOCALHOST, 0))
        .map_err(|error| {
            crate::error::Failure::invalid_harness(format!("live Iroh endpoint bind addr failed: {error}"))
        })?;
    if let Some(secret_key) = secret_key {
        builder = builder.secret_key(secret_key);
    }
    builder
        .bind()
        .await
        .map_err(|error| crate::error::Failure::invalid_harness(format!("live Iroh endpoint bind failed: {error}")))
}

pub(super) fn control_live_topic_id(topic: &str) -> iroh_gossip::TopicId {
    let digest = blake3::hash(format!("molten.node-control.live.topic.v1:{topic}").as_bytes());
    iroh_gossip::TopicId::from_bytes(*digest.as_bytes())
}
