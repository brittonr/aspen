
fn service_run_receipt_ref(value: &IoValue) -> Result<String> {
    if let Some(fields) = value.collect_simple_record("node-control-service-run-receipt-v1", Some(17)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_SERVICE_RUN_RECEIPT_SCHEMA,
            "node control service run receipt",
        )?;
        return crate::preserves_rail::canonical_hash(value);
    }
    if let Some(fields) = value.collect_simple_record("node-control-service-run-receipt-v1", Some(15)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_SERVICE_RUN_RECEIPT_SCHEMA,
            "node control service run receipt",
        )?;
        return crate::preserves_rail::canonical_hash(value);
    }
    Err(MoltenError::invalid_harness("expected <node-control-service-run-receipt-v1 ...>"))
}

fn live_transport_receipt_ref(value: &IoValue) -> Result<(String, String, String)> {
    let fields = value
        .collect_simple_record("node-control-live-transport-receipt-v1", Some(13))
        .or_else(|| value.collect_simple_record("node-control-live-transport-receipt-v1", Some(11)))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-live-transport-receipt-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_LIVE_TRANSPORT_RECEIPT_SCHEMA,
        "node control live transport receipt",
    )?;
    Ok((
        crate::preserves_rail::canonical_hash(value)?,
        record_string(&fields[1], "operation")?,
        record_ref_string(&fields[7], "envelope")?,
    ))
}

fn live_listener_receipt_refs(value: &IoValue) -> Result<(String, Vec<String>, String)> {
    let fields = value
        .collect_simple_record("node-control-live-listener-receipt-v1", Some(16))
        .or_else(|| value.collect_simple_record("node-control-live-listener-receipt-v1", Some(14)))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-live-listener-receipt-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_LIVE_LISTENER_RECEIPT_SCHEMA,
        "node control live listener receipt",
    )?;
    Ok((
        crate::preserves_rail::canonical_hash(value)?,
        record_ref_strings(&fields[9], "transport-receipts")?,
        record_ref_string(&fields[11], "service-run")?,
    ))
}

pub async fn serve_control_live_listener(input: &ControlLiveServeInput<'_>) -> Result<ControlLiveServe> {
    validate_state_root(input.state_root)?;
    validate_node_id(input.topic)?;
    validate_listener_event_limit(input.max_events)?;
    validate_loop_request_limit(input.max_requests_per_tick)?;
    ensure_state_layout(input.state_root)?;
    let identity = crate::node_identity::parse_identity(&read_preserves(&input.state_root.join(IDENTITY_FILE))?)?;
    let lookup = iroh::address_lookup::memory::MemoryLookup::new();
    let endpoint = live_gossip_endpoint(&lookup, Some(stable_live_endpoint_secret(&identity))).await?;
    let bound_endpoint_id = format!("iroh:{}", endpoint.id());
    let live_ticket = live_ticket_for_bound_endpoint(input.state_root, &identity, input.topic, &endpoint.addr())?;
    lookup.add_endpoint_info(endpoint.addr());
    let gossip = iroh_gossip::Gossip::builder().spawn(endpoint.clone());
    let router = iroh::protocol::Router::builder(endpoint).accept(iroh_gossip::ALPN, gossip.clone()).spawn();
    let mut topic = gossip
        .subscribe(control_live_topic_id(input.topic), Vec::new())
        .await
        .map_err(|error| MoltenError::invalid_harness(format!("live Iroh serve subscribe failed: {error}")))?;
    let served = serve_node_control_live_listener_with_topic(
        input,
        &mut topic,
        &identity.node_id,
        &identity.endpoint_id,
        &bound_endpoint_id,
    )
    .await;
    router
        .shutdown()
        .await
        .map_err(|error| MoltenError::invalid_harness(format!("live Iroh serve router shutdown failed: {error}")))?;
    let mut served = served?;
    served.live_ticket_ref = Some(live_ticket.ticket_ref);
    served.live_ticket_value = Some(live_ticket.value);
    Ok(served)
}

pub async fn control_live_serve_listener_loopback(
    input: &ControlLiveServeLoopbackInput<'_>,
) -> Result<ControlLiveServeLoopback> {
    validate_state_root(input.state_root)?;
    ensure_state_layout(input.state_root)?;
    let envelope_input = ControlIngressEnvelopeInput {
        request_value: input.request_value,
        from_peer: input.from_peer,
        to_node: input.to_node,
        topic: input.topic,
        sequence: input.sequence,
        peer_bootstrap_refs: input.peer_bootstrap_refs,
        authority_refs: input.authority_refs,
        policy_refs: input.policy_refs,
        resource_refs: input.resource_refs,
        evidence_refs: input.evidence_refs,
    };
    let envelope = control_live_ingress_envelope(&envelope_input)?;
    let LoopbackPair {
        ticket_ref,
        ticket_value,
        bound_endpoint_id,
        mut receiver_topic,
        sender,
        receiver_router,
        sender_router,
        node_id,
        endpoint_id,
    } = loopback_pair(input.state_root, input.topic).await?;
    let published = publish_control_live_ingress(&ControlLiveIngressPublishInput {
        sender: &sender,
        envelope_value: &envelope.value,
        node_id: input.from_peer,
        topology_profile_ref: None,
        transport_profile_ref: None,
        effective_max_attempts: None,
        effective_join_timeout_ms: None,
    })
    .await?;
    let listener_input = ControlLiveServeInput {
        state_root: input.state_root,
        topic: input.topic,
        max_events: 4,
        event_timeout_ms: 1_000,
        max_requests_per_tick: input.max_requests_per_tick,
        supervisor_policy_value: None,
    };
    let mut listener = serve_node_control_live_listener_with_topic(
        &listener_input,
        &mut receiver_topic,
        &node_id,
        &endpoint_id,
        &bound_endpoint_id,
    )
    .await?;
    listener.live_ticket_ref = Some(ticket_ref);
    listener.live_ticket_value = Some(ticket_value);
    receiver_router.shutdown().await.map_err(|error| {
        MoltenError::invalid_harness(format!("live Iroh listener receiver shutdown failed: {error}"))
    })?;
    sender_router
        .shutdown()
        .await
        .map_err(|error| MoltenError::invalid_harness(format!("live Iroh listener sender shutdown failed: {error}")))?;
    Ok(ControlLiveServeLoopback {
        envelope_ref: envelope.envelope_ref,
        publish_receipt_ref: published.transport_receipt_ref,
        listener,
    })
}

struct LoopbackPair {
    ticket_ref: String,
    ticket_value: IoValue,
    bound_endpoint_id: String,
    receiver_topic: iroh_gossip::api::GossipTopic,
    sender: iroh_gossip::api::GossipSender,
    receiver_router: iroh::protocol::Router,
    sender_router: iroh::protocol::Router,
    node_id: String,
    endpoint_id: String,
}

async fn loopback_pair(state_root: &Path, topic: &str) -> Result<LoopbackPair> {
    let identity = crate::node_identity::parse_identity(&read_preserves(&state_root.join(IDENTITY_FILE))?)?;
    let lookup = iroh::address_lookup::memory::MemoryLookup::new();
    let receiver_endpoint = live_gossip_endpoint(&lookup, Some(stable_live_endpoint_secret(&identity))).await?;
    let sender_endpoint = live_gossip_endpoint(&lookup, None).await?;
    let ticket = live_ticket_for_bound_endpoint(state_root, &identity, topic, &receiver_endpoint.addr())?;
    lookup.add_endpoint_info(receiver_endpoint.addr());
    lookup.add_endpoint_info(sender_endpoint.addr());
    let receiver_id = receiver_endpoint.id();
    let sender_id = sender_endpoint.id();
    let bound_endpoint_id = format!("iroh:{receiver_id}");
    let receiver_gossip = iroh_gossip::Gossip::builder().spawn(receiver_endpoint.clone());
    let sender_gossip = iroh_gossip::Gossip::builder().spawn(sender_endpoint.clone());
    let receiver_router = iroh::protocol::Router::builder(receiver_endpoint)
        .accept(iroh_gossip::ALPN, receiver_gossip.clone())
        .spawn();
    let sender_router = iroh::protocol::Router::builder(sender_endpoint)
        .accept(iroh_gossip::ALPN, sender_gossip.clone())
        .spawn();
    let topic_id = control_live_topic_id(topic);
    let receiver_topic = receiver_gossip.subscribe(topic_id, vec![sender_id]).await.map_err(|error| {
        MoltenError::invalid_harness(format!("live Iroh listener receiver subscribe failed: {error}"))
    })?;
    let sender_topic = sender_gossip
        .subscribe_and_join(topic_id, vec![receiver_id])
        .await
        .map_err(|error| MoltenError::invalid_harness(format!("live Iroh listener sender join failed: {error}")))?;
    let (sender, _unused_receiver) = sender_topic.split();
    Ok(LoopbackPair {
        ticket_ref: ticket.ticket_ref,
        ticket_value: ticket.value,
        bound_endpoint_id,
        receiver_topic,
        sender,
        receiver_router,
        sender_router,
        node_id: identity.node_id.clone(),
        endpoint_id: identity.endpoint_id.clone(),
    })
}

struct EventScan {
    diagnostics: Vec<String>,
    transport_receipt_refs: Vec<String>,
    neighbor_events: Vec<String>,
    observed_events: u64,
}

async fn scan_events(
    input: &ControlLiveServeInput<'_>,
    receiver: &mut iroh_gossip::api::GossipTopic,
    node_id: &str,
) -> Result<EventScan> {
    let event_capacity = usize::try_from(input.max_events)
        .map_err(|_| MoltenError::invalid_harness("node control live listener max events exceeds usize capacity"))?;
    let mut diagnostics = Vec::with_capacity(event_capacity.saturating_add(2));
    let mut transport_receipt_refs = Vec::with_capacity(event_capacity);
    let mut neighbor_events = Vec::with_capacity(event_capacity);
    let mut observed_events = 0_u64;
    let timeout = std::time::Duration::from_millis(input.event_timeout_ms);
    for _ in 0..input.max_events {
        let event = match tokio::time::timeout(timeout, receiver.next()).await {
            Ok(Some(Ok(event))) => event,
            Ok(Some(Err(error))) => {
                diagnostics.push(format!("live Iroh serve listener receive failed: {error}"));
                break;
            }
            Ok(None) => break,
            Err(_) => break,
        };
        observed_events += 1;
        match &event {
            iroh_gossip::api::Event::NeighborUp(endpoint) => {
                neighbor_events.push(format!("up:iroh:{endpoint}"));
            }
            iroh_gossip::api::Event::NeighborDown(endpoint) => {
                neighbor_events.push(format!("down:iroh:{endpoint}"));
            }
            iroh_gossip::api::Event::Lagged => diagnostics.push("live Iroh serve listener lagged".to_string()),
            iroh_gossip::api::Event::Received(_) => {
                if let Some(received) =
                    receive_control_live_ingress_event(input.state_root, &event, input.topic, node_id)?
                {
                    transport_receipt_refs.push(received.transport_receipt_ref);
                }
            }
        }
        if !transport_receipt_refs.is_empty() {
            break;
        }
    }
    Ok(EventScan {
        diagnostics,
        transport_receipt_refs,
        neighbor_events,
        observed_events,
    })
}
