//! Ingress envelope construction, live publish, and Iroh loopback.

pub async fn publish_control_live_ingress(
    input: &super::requests::ControlLiveIngressPublishInput<'_>,
) -> crate::error::Result<super::transits::ControlLiveIngressPublish> {
    super::storage::validate_node_id(input.node_id)?;
    let envelope = super::delivery::parse_control_ingress_envelope(input.envelope_value)?;
    let mut diagnostics = Vec::new();
    if envelope.transport != super::settings::LIVE_CONTROL_INGRESS_TRANSPORT {
        diagnostics.push(format!(
            "node control live publish requires transport {LIVE_CONTROL_INGRESS_TRANSPORT}, got {}",
            envelope.transport,
            LIVE_CONTROL_INGRESS_TRANSPORT = super::settings::LIVE_CONTROL_INGRESS_TRANSPORT
        ));
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    if diagnostics.is_empty() {
        input
            .sender
            .broadcast(crate::preserves_rail::canonical_bytes(&envelope.value)?.into())
            .await
            .map_err(|error| {
                crate::error::Failure::invalid_harness(format!("live Iroh node control publish failed: {error}"))
            })?;
    }
    let receipt_value =
        super::transports::live_transport_receipt_value(&super::transports::LiveTransportReceiptValueInput {
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
    Ok(super::transits::ControlLiveIngressPublish {
        envelope_ref: envelope.envelope_ref,
        transport_receipt_ref,
        transport_receipt_value: receipt_value,
    })
}

fn envelope_for_loopback(
    input: &super::requests::ControlLiveLoopbackInput<'_>,
) -> crate::error::Result<super::transits::ControlIngressEnvelope> {
    super::ingresses::control_live_ingress_envelope(&super::requests::ControlIngressEnvelopeInput {
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

pub async fn control_live_iroh_loopback(
    input: &super::requests::ControlLiveLoopbackInput<'_>,
) -> crate::error::Result<super::transits::ControlLiveLoopback> {
    let state_root = crate::node_state::DirectoryRoot::open(input.state_root)?;
    super::storage::validate_state_root(input.state_root)?;
    super::storage::ensure_state_layout(&state_root)?;
    let envelope = envelope_for_loopback(input)?;
    let topic_id = super::endpoints::control_live_topic_id(input.topic);
    let lookup = iroh::address_lookup::memory::MemoryLookup::new();
    let receiver_endpoint = super::endpoints::live_gossip_endpoint(&lookup, None).await?;
    let sender_endpoint = super::endpoints::live_gossip_endpoint(&lookup, None).await?;
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
    let mut receiver_topic = receiver_gossip.subscribe(topic_id, vec![sender_id]).await.map_err(|error| {
        crate::error::Failure::invalid_harness(format!("live Iroh receiver subscribe failed: {error}"))
    })?;
    let sender_topic = sender_gossip
        .subscribe_and_join(topic_id, vec![receiver_id])
        .await
        .map_err(|error| crate::error::Failure::invalid_harness(format!("live Iroh sender join failed: {error}")))?;
    let (sender, _receiver_unused) = sender_topic.split();
    receiver_topic
        .joined()
        .await
        .map_err(|error| crate::error::Failure::invalid_harness(format!("live Iroh receiver join failed: {error}")))?;
    let published = publish_control_live_ingress(&super::requests::ControlLiveIngressPublishInput {
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
        super::inbound::receive_first_live_ingress_event(&state_root, &mut receiver_topic, input.topic, input.to_node),
    )
    .await
    .map_err(|_| {
        crate::error::Failure::invalid_harness("live Iroh node control loopback timed out waiting for envelope")
    })??;
    receiver_router.shutdown().await.map_err(|error| {
        crate::error::Failure::invalid_harness(format!("live Iroh receiver router shutdown failed: {error}"))
    })?;
    sender_router.shutdown().await.map_err(|error| {
        crate::error::Failure::invalid_harness(format!("live Iroh sender router shutdown failed: {error}"))
    })?;
    Ok(super::transits::ControlLiveLoopback {
        envelope_ref: envelope.envelope_ref,
        publish_receipt_ref: published.transport_receipt_ref,
        publish_receipt_value: published.transport_receipt_value,
        receive_receipt_ref: received.transport_receipt_ref,
        receive_receipt_value: received.transport_receipt_value,
        ingress_receipt_ref: received.ingress_receipt_ref,
        has_enqueued: received.has_enqueued,
    })
}
