// Live Iroh gossip publication and delivery.

pub async fn publish_live_gossip(
    sender: &iroh_gossip::api::GossipSender,
    envelope: &Envelope,
    node: &str,
) -> Result<Exchange> {
    validate_name(node, "publisher node")?;
    // This unbound live entrypoint has no receiver-observation capability. A
    // local recording is not a live receiver acknowledgment, so only reference-
    // free messages and introducing operations are admissible here.
    require_outbound_envelope(envelope, None)?;
    sender
        .broadcast(canonical_bytes(&envelope.value)?.into())
        .await
        .map_err(|error| MoltenError::invalid_harness(format!("live Iroh gossip publish failed: {error}")))?;
    Ok(Exchange {
        envelope_ref: envelope.envelope_ref.clone(),
        receipt_value: transport_receipt_value_for_transport(TransportReceiptInput {
            transport: LIVE_GOSSIP_TRANSPORT,
            operation: "publish",
            decision: "pass",
            node,
            envelope,
            diagnostics: Vec::new(),
            checks: vec![
                ("canonical-envelope-ref".to_owned(), "pass".to_owned()),
                ("live-iroh-gossip".to_owned(), "pass".to_owned()),
                ("transport-is-not-authority".to_owned(), "pass".to_owned()),
            ],
        }),
    })
}

pub fn deliver_live_gossip_event(
    root: &Path,
    event: &iroh_gossip::api::Event,
    topic: &str,
    receiver_peer: &str,
) -> Result<Option<Delivery>> {
    let root = open_capability_dataspace_root(root)?;
    deliver_live_gossip_event_with_root(&root, event, topic, receiver_peer)
}

pub fn deliver_live_gossip_event_with_root(
    root: &CapabilityDataspaceRoot,
    event: &iroh_gossip::api::Event,
    topic: &str,
    receiver_peer: &str,
) -> Result<Option<Delivery>> {
    match event {
        iroh_gossip::api::Event::Received(message) => deliver_live_gossip_bytes_with_root(
            root,
            message.content.as_ref(),
            topic,
            receiver_peer,
            &message.delivered_from.to_string(),
        )
        .map(Some),
        iroh_gossip::api::Event::NeighborUp(_)
        | iroh_gossip::api::Event::NeighborDown(_)
        | iroh_gossip::api::Event::Lagged => Ok(None),
    }
}

pub fn deliver_live_gossip_bytes(
    root: &Path,
    bytes: &[u8],
    topic: &str,
    receiver_peer: &str,
    delivered_from: &str,
) -> Result<Delivery> {
    let root = open_capability_dataspace_root(root)?;
    deliver_live_gossip_bytes_with_root(&root, bytes, topic, receiver_peer, delivered_from)
}

pub fn deliver_live_gossip_bytes_with_root(
    root: &CapabilityDataspaceRoot,
    bytes: &[u8],
    topic: &str,
    receiver_peer: &str,
    delivered_from: &str,
) -> Result<Delivery> {
    validate_name(topic, "topic")?;
    validate_name(receiver_peer, "receiver peer")?;
    validate_name(delivered_from, "delivered from")?;
    let value = parse_canonical_bytes(bytes)?;
    let envelope = parse_envelope(&value)?;
    if envelope.topic != topic {
        return Err(MoltenError::invalid_harness(format!(
            "live Iroh envelope topic {} does not match subscribed topic {topic}",
            envelope.topic
        )));
    }
    if envelope.to_peer != receiver_peer && envelope.to_peer != "*" {
        return Err(MoltenError::invalid_harness(format!(
            "live Iroh envelope target {} does not match receiver {receiver_peer}",
            envelope.to_peer
        )));
    }
    validate_content_refs_available_with_root(root, &envelope.content_refs)?;
    let receipt_value = transport_receipt_value_for_transport(TransportReceiptInput {
        transport: LIVE_GOSSIP_TRANSPORT,
        operation: "deliver",
        decision: "pass",
        node: receiver_peer,
        envelope: &envelope,
        diagnostics: Vec::new(),
        checks: vec![
            ("canonical-envelope-ref".to_owned(), "pass".to_owned()),
            ("topic-peer-binding".to_owned(), "pass".to_owned()),
            ("content-refs-verified".to_owned(), "pass".to_owned()),
            ("live-iroh-gossip".to_owned(), "pass".to_owned()),
            ("transport-is-not-authority".to_owned(), "pass".to_owned()),
        ],
    });
    Ok(Delivery {
        envelope,
        receipt_value,
    })
}
