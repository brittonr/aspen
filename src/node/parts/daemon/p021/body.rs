
fn control_ingress_envelope_for_transport(
    input: &ControlIngressEnvelopeInput<'_>,
    transport: &str,
    transport_check: &str,
) -> Result<ControlIngressEnvelope> {
    let request = crate::node_runtime::parse_control_request(input.request_value)?;
    validate_node_id(input.from_peer)?;
    validate_node_id(input.to_node)?;
    validate_node_id(input.topic)?;
    validate_node_id(transport)?;
    validate_ingress_refs(input.peer_bootstrap_refs, "node control ingress peer bootstrap ref")?;
    validate_ingress_refs(input.authority_refs, "node control ingress authority ref")?;
    validate_ingress_refs(input.policy_refs, "node control ingress policy ref")?;
    validate_ingress_refs(input.resource_refs, "node control ingress resource ref")?;
    validate_ingress_refs(input.evidence_refs, "node control ingress evidence ref")?;
    let scope_ref = crate::delivery_idempotency::remote_topic_scope_ref(input.topic, input.to_node)?;
    let operation = crate::delivery_idempotency::derive_operation_id(crate::delivery_idempotency::OperationIdInput {
        scope_ref,
        producer: input.from_peer.to_string(),
        consumer: input.to_node.to_string(),
        sequence: input.sequence,
        intent: "node-control-ingress".to_string(),
        payload_ref: request.request_ref.clone(),
        policy_refs: input.policy_refs.to_vec(),
    })?;
    let value = ingress_envelope_value(input, &request, &operation.operation_ref, transport, transport_check)?;
    parse_control_ingress_envelope(&value)
}

pub async fn publish_control_live_ingress(
    input: &ControlLiveIngressPublishInput<'_>,
) -> Result<ControlLiveIngressPublish> {
    validate_node_id(input.node_id)?;
    let envelope = parse_control_ingress_envelope(input.envelope_value)?;
    let mut diagnostics = Vec::new();
    if envelope.transport != LIVE_CONTROL_INGRESS_TRANSPORT {
        diagnostics.push(format!(
            "node control live publish requires transport {LIVE_CONTROL_INGRESS_TRANSPORT}, got {}",
            envelope.transport
        ));
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    if diagnostics.is_empty() {
        input
            .sender
            .broadcast(crate::preserves_rail::canonical_bytes(&envelope.value)?.into())
            .await
            .map_err(|error| MoltenError::invalid_harness(format!("live Iroh node control publish failed: {error}")))?;
    }
    let receipt_value = live_transport_receipt_value(&LiveTransportReceiptValueInput {
        operation: "publish",
        decision,
        node_id: input.node_id,
        delivered_from: None,
        envelope: &envelope,
        ingress_receipt_ref: None,
        topology_profile_ref: input.topology_profile_ref,
        transport_profile_ref: input.transport_profile_ref,
        effective_max_attempts: input.effective_max_attempts,
        effective_join_timeout_ms: input.effective_join_timeout_ms,
        diagnostics: &diagnostics,
    })?;
    let transport_receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    Ok(ControlLiveIngressPublish {
        envelope_ref: envelope.envelope_ref,
        transport_receipt_ref,
        transport_receipt_value: receipt_value,
    })
}

pub fn receive_control_live_ingress_event(
    state_root: &Path,
    event: &iroh_gossip::api::Event,
    topic: &str,
    receiver_node: &str,
) -> Result<Option<ControlLiveIngressReceive>> {
    match event {
        iroh_gossip::api::Event::Received(message) => {
            receive_control_live_ingress_bytes(&ControlLiveIngressReceiveBytesInput {
                state_root,
                topic,
                receiver_node,
                delivered_from: &format!("iroh:{}", message.delivered_from),
                bytes: message.content.as_ref(),
            })
            .map(Some)
        }
        iroh_gossip::api::Event::NeighborUp(_)
        | iroh_gossip::api::Event::NeighborDown(_)
        | iroh_gossip::api::Event::Lagged => Ok(None),
    }
}

pub fn receive_control_live_ingress_bytes(
    input: &ControlLiveIngressReceiveBytesInput<'_>,
) -> Result<ControlLiveIngressReceive> {
    validate_state_root(input.state_root)?;
    validate_node_id(input.topic)?;
    validate_node_id(input.receiver_node)?;
    validate_node_id(input.delivered_from)?;
    ensure_state_layout(input.state_root)?;
    let value = crate::preserves_rail::parse_canonical_bytes(input.bytes)?;
    let envelope = parse_control_ingress_envelope(&value)?;
    let mut diagnostics = live_receive_diagnostics(input, &envelope);
    write_ingress_envelope_and_verify(input.state_root, input.topic, &envelope)?;
    import_artifact(input.state_root, &value)?;
    let delivered = if diagnostics.is_empty() {
        deliver_control_ingress(&ControlIngressDeliverInput {
            state_root: input.state_root,
            topic: input.topic,
            envelope_ref: &envelope.envelope_ref,
        })?
    } else {
        denied_live_ingress_delivery(input.state_root, &envelope, &diagnostics)?
    };
    let ingress_decision = ingress_receipt_decision(&delivered.ingress_receipt_value)?;
    if ingress_decision != "pass" {
        diagnostics.push(format!("node control live ingress delivery decision {ingress_decision}"));
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = live_transport_receipt_value(&LiveTransportReceiptValueInput {
        operation: "receive",
        decision,
        node_id: input.receiver_node,
        delivered_from: Some(input.delivered_from),
        envelope: &envelope,
        ingress_receipt_ref: Some(&delivered.ingress_receipt_ref),
        topology_profile_ref: None,
        transport_profile_ref: None,
        effective_max_attempts: None,
        effective_join_timeout_ms: None,
        diagnostics: &diagnostics,
    })?;
    let transport_receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    write_preserves(
        &control_live_transport_receipt_path(input.state_root, &envelope.envelope_ref, "receive"),
        &receipt_value,
    )?;
    import_artifact(input.state_root, &receipt_value)?;
    Ok(ControlLiveIngressReceive {
        envelope_ref: envelope.envelope_ref,
        transport_receipt_ref,
        transport_receipt_value: receipt_value,
        ingress_receipt_ref: delivered.ingress_receipt_ref,
        ingress_receipt_value: delivered.ingress_receipt_value,
        has_enqueued: delivered.has_enqueued,
    })
}

fn envelope_for_loopback(input: &ControlLiveLoopbackInput<'_>) -> Result<ControlIngressEnvelope> {
    control_live_ingress_envelope(&ControlIngressEnvelopeInput {
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
    })
}

pub async fn control_live_iroh_loopback(input: &ControlLiveLoopbackInput<'_>) -> Result<ControlLiveLoopback> {
    validate_state_root(input.state_root)?;
    ensure_state_layout(input.state_root)?;
    let envelope = envelope_for_loopback(input)?;
    let topic_id = control_live_topic_id(input.topic);
    let lookup = iroh::address_lookup::memory::MemoryLookup::new();
    let receiver_endpoint = live_gossip_endpoint(&lookup, None).await?;
    let sender_endpoint = live_gossip_endpoint(&lookup, None).await?;
    lookup.add_endpoint_info(receiver_endpoint.addr());
    lookup.add_endpoint_info(sender_endpoint.addr());
    let receiver_id = receiver_endpoint.id();
    let sender_id = sender_endpoint.id();
    let receiver_gossip = iroh_gossip::Gossip::builder().spawn(receiver_endpoint.clone());
    let sender_gossip = iroh_gossip::Gossip::builder().spawn(sender_endpoint.clone());
    let receiver_router = iroh::protocol::Router::builder(receiver_endpoint)
        .accept(iroh_gossip::ALPN, receiver_gossip.clone())
        .spawn();
    let sender_router = iroh::protocol::Router::builder(sender_endpoint)
        .accept(iroh_gossip::ALPN, sender_gossip.clone())
        .spawn();
    let mut receiver_topic = receiver_gossip
        .subscribe(topic_id, vec![sender_id])
        .await
        .map_err(|error| MoltenError::invalid_harness(format!("live Iroh receiver subscribe failed: {error}")))?;
    let sender_topic = sender_gossip
        .subscribe_and_join(topic_id, vec![receiver_id])
        .await
        .map_err(|error| MoltenError::invalid_harness(format!("live Iroh sender join failed: {error}")))?;
    let (sender, _receiver_unused) = sender_topic.split();
    receiver_topic
        .joined()
        .await
        .map_err(|error| MoltenError::invalid_harness(format!("live Iroh receiver join failed: {error}")))?;
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
    let received = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        receive_first_live_ingress_event(input.state_root, &mut receiver_topic, input.topic, input.to_node),
    )
    .await
    .map_err(|_| MoltenError::invalid_harness("live Iroh node control loopback timed out waiting for envelope"))??;
    receiver_router
        .shutdown()
        .await
        .map_err(|error| MoltenError::invalid_harness(format!("live Iroh receiver router shutdown failed: {error}")))?;
    sender_router
        .shutdown()
        .await
        .map_err(|error| MoltenError::invalid_harness(format!("live Iroh sender router shutdown failed: {error}")))?;
    Ok(ControlLiveLoopback {
        envelope_ref: envelope.envelope_ref,
        publish_receipt_ref: published.transport_receipt_ref,
        publish_receipt_value: published.transport_receipt_value,
        receive_receipt_ref: received.transport_receipt_ref,
        receive_receipt_value: received.transport_receipt_value,
        ingress_receipt_ref: received.ingress_receipt_ref,
        has_enqueued: received.has_enqueued,
    })
}

pub fn preflight_control_live_send(input: &ControlLiveSendInput<'_>) -> Result<ControlLiveSendPreflight> {
    if let Some(state_root) = input.state_root {
        validate_state_root(state_root)?;
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
    if let Some(state_root) = input.state_root {
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
