
async fn serve_node_control_live_listener_with_topic(
    state_root: &crate::node_state::NodeStateRoot,
    input: &ControlLiveServeInput<'_>,
    receiver: &mut iroh_gossip::api::GossipTopic,
    node_id: &str,
    logical_endpoint_id: &str,
    bound_endpoint_id: &str,
) -> Result<ControlLiveServe> {
    validate_listener_event_limit(input.max_events)?;
    validate_loop_request_limit(input.max_requests_per_tick)?;
    let startup = current_startup_receipt(state_root)?;
    let mut scan = scan_events(state_root, input, receiver, node_id).await?;
    let service_input = ControlServeInput {
        state_root: input.state_root,
        topic: input.topic,
        max_ticks: 1,
        max_requests_per_tick: input.max_requests_per_tick,
        supervisor_policy_value: input.supervisor_policy_value,
    };
    let service = serve_control_with_root(state_root, &service_input)?;
    if service.decision != "pass" {
        scan.diagnostics
            .push(format!("node control live listener service drain decision {}", service.decision));
    }
    let decision = if scan.diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = live_listener_receipt_value(&ListenerReceiptValueInput {
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
    write_preserves(
        state_root,
        &control_live_listener_receipt_path(&listener_receipt_ref)?,
        &receipt_value,
    )?;
    import_artifact(state_root, &receipt_value)?;
    Ok(ControlLiveServe {
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

async fn receive_first_live_ingress_event(
    state_root: &crate::node_state::NodeStateRoot,
    receiver: &mut iroh_gossip::api::GossipTopic,
    topic: &str,
    receiver_node: &str,
) -> Result<ControlLiveIngressReceive> {
    for _ in 0..MAX_CONTROL_LIVE_LISTENER_EVENTS {
        let Some(event) = receiver.next().await else {
            return Err(MoltenError::invalid_harness("live Iroh receiver closed before node control envelope arrived"));
        };
        let event =
            event.map_err(|error| MoltenError::invalid_harness(format!("live Iroh receive failed: {error}")))?;
        if let Some(received) = receive_control_live_ingress_event_with_root(state_root, &event, topic, receiver_node)? {
            return Ok(received);
        }
    }
    Err(MoltenError::invalid_harness(
        "live Iroh receiver exceeded bounded event scan before node control envelope arrived",
    ))
}

fn stable_live_endpoint_secret(
    state_root: &crate::node_state::NodeStateRoot,
    identity: &crate::node_identity::Identity,
) -> Result<iroh::SecretKey> {
    let identity_namespace = state_root.identity()?;
    crate::fabric_crypto_identity::load_transport_secret_for_identity(
        &identity_namespace,
        &identity.endpoint_id,
        &identity.secret_ref,
        &identity.backend_ref,
    )
}

fn stable_live_endpoint_id(identity: &crate::node_identity::Identity) -> String {
    identity.endpoint_id.clone()
}

fn live_ticket_address_refs(addr: &iroh::EndpointAddr) -> Vec<String> {
    addr.addrs.iter().map(ToString::to_string).collect()
}

fn live_ticket_for_bound_endpoint(
    state_root: &crate::node_state::NodeStateRoot,
    identity: &crate::node_identity::Identity,
    topic: &str,
    addr: &iroh::EndpointAddr,
) -> Result<ControlLiveTicket> {
    let address_refs = live_ticket_address_refs(addr);
    let value = control_live_ticket_value(&ControlLiveTicketInput {
        node_id: &identity.node_id,
        identity_ref: &identity.identity_ref,
        logical_endpoint_id: &identity.endpoint_id,
        live_endpoint_id: &format!("iroh:{}", addr.id),
        topic,
        address_refs: &address_refs,
        policy_refs: &identity.policy_refs,
        evidence_refs: &identity.receipt_refs,
    })?;
    let ticket = parse_control_live_ticket(&value)?;
    import_artifact(state_root, &value)?;
    Ok(ticket)
}

fn live_send_ticket_diagnostics(input: &ControlLiveSendInput<'_>, ticket: &ControlLiveTicket) -> Vec<String> {
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

fn live_send_state_root_evidence_diagnostics(
    state_root: &crate::node_state::NodeStateRoot,
    input: &ControlLiveSendInput<'_>,
    envelope: &ControlIngressEnvelope,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(
        input.peer_bootstrap_refs.len().saturating_add(input.authority_refs.len()).saturating_add(4),
    );
    if input.peer_bootstrap_refs.is_empty() {
        diagnostics.push(
            "node control live send peer admission refs missing; run live-ticket-import --peer-admission before live send"
                .to_string(),
        );
    } else {
        let peer_diagnostics = evaluate_live_peer_bootstrap(state_root, envelope)?;
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

fn live_send_authority_grant_diagnostics(
    state_root: &crate::node_state::NodeStateRoot,
    envelope: &ControlIngressEnvelope,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(envelope.authority_refs.len().saturating_add(2));
    let mut has_candidate_authority = false;
    let mut has_admitted_grant = false;
    for authority_ref in envelope
        .authority_refs
        .iter()
        .filter(|authority_ref| envelope.request.authority_refs.contains(*authority_ref))
    {
        has_candidate_authority = true;
        match read_ledger_artifact(state_root, authority_ref) {
            Ok(value) => match parse_control_authority_grant(&value) {
                Ok(grant) => {
                    let grant_diagnostics = authority_grant_diagnostics(envelope, &grant);
                    if grant_diagnostics.is_empty() {
                        has_admitted_grant = true;
                        break;
                    }
                    diagnostics.extend(grant_diagnostics);
                }
                Err(error) => {
                    if let Some(diagnostic) = transport_evidence_not_authority_diagnostic(
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

fn live_ticket_endpoint_addr(ticket: &ControlLiveTicket) -> Result<iroh::EndpointAddr> {
    let endpoint_id = ticket
        .live_endpoint_id
        .strip_prefix("iroh:")
        .ok_or_else(|| MoltenError::invalid_harness("node control live ticket endpoint must use iroh: prefix"))?
        .parse::<iroh::EndpointId>()
        .map_err(|error| {
            MoltenError::invalid_harness(format!("node control live ticket endpoint parse failed: {error}"))
        })?;
    let mut addrs = Vec::with_capacity(ticket.address_refs.len());
    for address_ref in &ticket.address_refs {
        let addr = if let Some(ip_addr) = address_ref.strip_prefix("ip:") {
            iroh::TransportAddr::Ip(ip_addr.parse::<SocketAddr>().map_err(|error| {
                MoltenError::invalid_harness(format!("node control live ticket ip address parse failed: {error}"))
            })?)
        } else if let Some(relay_url) = address_ref.strip_prefix("relay:") {
            iroh::TransportAddr::Relay(relay_url.parse::<iroh::RelayUrl>().map_err(|error| {
                MoltenError::invalid_harness(format!("node control live ticket relay address parse failed: {error}"))
            })?)
        } else {
            return Err(MoltenError::invalid_harness(format!(
                "node control live ticket unsupported transport address {address_ref}"
            )));
        };
        addrs.push(addr);
    }
    Ok(iroh::EndpointAddr::from_parts(endpoint_id, addrs))
}

async fn live_gossip_endpoint(
    lookup: &iroh::address_lookup::memory::MemoryLookup,
    secret_key: Option<iroh::SecretKey>,
) -> Result<iroh::Endpoint> {
    let mut builder = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .relay_mode(iroh::RelayMode::Disabled)
        .address_lookup(lookup.clone())
        .alpns(vec![iroh_gossip::ALPN.to_vec()])
        .clear_ip_transports()
        .bind_addr((Ipv4Addr::LOCALHOST, 0))
        .map_err(|error| MoltenError::invalid_harness(format!("live Iroh endpoint bind addr failed: {error}")))?;
    if let Some(secret_key) = secret_key {
        builder = builder.secret_key(secret_key);
    }
    builder
        .bind()
        .await
        .map_err(|error| MoltenError::invalid_harness(format!("live Iroh endpoint bind failed: {error}")))
}

fn control_live_topic_id(topic: &str) -> iroh_gossip::TopicId {
    let digest = blake3::hash(format!("molten.node-control.live.topic.v1:{topic}").as_bytes());
    iroh_gossip::TopicId::from_bytes(*digest.as_bytes())
}

fn denied_live_ingress_delivery(
    state_root: &crate::node_state::NodeStateRoot,
    envelope: &ControlIngressEnvelope,
    diagnostics: &[String],
) -> Result<ControlIngressDeliver> {
    let receipt_value = ingress_receipt_value(&IngressReceiptValueInput {
        decision: "deny",
        phase: "live-receive-deny",
        transport: &envelope.transport,
        envelope,
        idempotency_receipt_ref: None,
        queue_receipt_ref: None,
        diagnostics,
    })?;
    let ingress_receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    write_preserves(
        state_root,
        &control_ingress_receipt_path(&envelope.envelope_ref, "deliver")?,
        &receipt_value,
    )?;
    import_artifact(state_root, &receipt_value)?;
    Ok(ControlIngressDeliver {
        envelope_ref: envelope.envelope_ref.clone(),
        request_ref: envelope.request.request_ref.clone(),
        ingress_receipt_ref,
        ingress_receipt_value: receipt_value,
        idempotency_receipt_ref: None,
        queue_receipt_ref: None,
        has_enqueued: false,
    })
}
