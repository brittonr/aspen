
pub fn parse_envelope(value: &IoValue) -> Result<Envelope> {
    let (fields, has_operation_ref) =
        if let Some(fields) = value.collect_simple_record("remote-dataspace-envelope-v1", Some(12)) {
            (fields, true)
        } else {
            (
                value
                    .collect_simple_record("remote-dataspace-envelope-v1", Some(10))
                    .ok_or_else(|| MoltenError::invalid_harness("expected <remote-dataspace-envelope-v1 ...>"))?,
                false,
            )
        };
    require_schema(&fields[0], ENVELOPE_SCHEMA, "remote dataspace envelope schema")?;
    let from_peer = record_string(&fields[1], "from-peer")?;
    let from_actor = record_string(&fields[2], "from-actor")?;
    let to_peer = record_string(&fields[3], "to-peer")?;
    let topic = record_string(&fields[4], "topic")?;
    let operation = Operation::parse(&record_string(&fields[5], "operation")?)?;
    let payload = record_iovalue(&fields[6], "payload")?;
    let content_refs = record_string_sequence(&fields[7], "content-refs")?;
    let capability_refs = record_string_sequence(&fields[8], "capability-refs")?;
    let evidence_refs = record_string_sequence(&fields[9], "evidence-refs")?;
    validate_name(&from_peer, "from peer")?;
    validate_name(&from_actor, "from actor")?;
    validate_name(&to_peer, "to peer")?;
    validate_name(&topic, "topic")?;
    validate_refs(&content_refs, "content ref")?;
    validate_refs(&capability_refs, "capability ref")?;
    validate_refs(&evidence_refs, "evidence ref")?;
    let sequence = if has_operation_ref {
        record_u64(&fields[10], "delivery-sequence")?
    } else {
        payload_delivery_sequence(&payload)?
    };
    let stored_ref = if has_operation_ref {
        Some(record_string(&fields[11], "operation-ref")?)
    } else {
        None
    };
    let operation_ref = parsed_ref(RefParts {
        stored_ref,
        sequence,
        from_peer: &from_peer,
        from_actor: &from_actor,
        to_peer: &to_peer,
        topic: &topic,
        operation,
        payload: &payload,
        capability_refs: &capability_refs,
        evidence_refs: &evidence_refs,
    })?;
    Ok(Envelope {
        envelope_ref: canonical_hash(value)?,
        from_peer,
        from_actor,
        to_peer,
        topic,
        operation,
        payload,
        content_refs,
        capability_refs,
        evidence_refs,
        sequence,
        operation_ref,
        value: value.clone(),
    })
}

struct RefParts<'a> {
    stored_ref: Option<String>,
    sequence: u64,
    from_peer: &'a str,
    from_actor: &'a str,
    to_peer: &'a str,
    topic: &'a str,
    operation: Operation,
    payload: &'a IoValue,
    capability_refs: &'a [String],
    evidence_refs: &'a [String],
}

fn parsed_ref(input: RefParts<'_>) -> Result<String> {
    let operation_ref = if let Some(stored_ref) = input.stored_ref {
        stored_ref
    } else {
        envelope_operation_ref(EnvelopeOperationRefInput {
            from_peer: input.from_peer,
            from_actor: input.from_actor,
            to_peer: input.to_peer,
            topic: input.topic,
            operation: input.operation,
            payload: input.payload,
            capability_refs: input.capability_refs,
            evidence_refs: input.evidence_refs,
            sequence: input.sequence,
        })?
    };
    let expected_operation_ref = envelope_operation_ref(EnvelopeOperationRefInput {
        from_peer: input.from_peer,
        from_actor: input.from_actor,
        to_peer: input.to_peer,
        topic: input.topic,
        operation: input.operation,
        payload: input.payload,
        capability_refs: input.capability_refs,
        evidence_refs: input.evidence_refs,
        sequence: input.sequence,
    })?;
    if operation_ref != expected_operation_ref {
        return Err(MoltenError::invalid_harness(format!(
            "remote dataspace operation ref {operation_ref} does not match canonical ref {expected_operation_ref}"
        )));
    }
    Ok(operation_ref)
}

pub fn store_content_blob(root: &Path, bytes: &[u8]) -> Result<String> {
    let root = open_capability_dataspace_root(root)?;
    store_content_blob_with_root(&root, bytes)
}

pub fn store_content_blob_with_root(root: &CapabilityDataspaceRoot, bytes: &[u8]) -> Result<String> {
    let content_ref = content_ref_from_bytes(bytes);
    root.root().write(&blob_store_path(&content_ref)?, bytes)?;
    Ok(content_ref)
}

pub fn publish_local_gossip(root: &Path, envelope: &Envelope, node: &str) -> Result<Exchange> {
    let root = open_capability_dataspace_root(root)?;
    publish_local_gossip_with_root(&root, envelope, node)
}

pub fn publish_local_gossip_with_root(
    root: &CapabilityDataspaceRoot,
    envelope: &Envelope,
    node: &str,
) -> Result<Exchange> {
    validate_name(node, "publisher node")?;
    validate_envelope_identity(envelope)?;
    validate_content_refs_available_with_root(root, &envelope.content_refs)?;
    root.root().write(
        &envelope_store_path(&envelope.topic, &envelope.envelope_ref)?,
        &canonical_bytes(&envelope.value)?,
    )?;
    Ok(Exchange {
        envelope_ref: envelope.envelope_ref.clone(),
        receipt_value: transport_receipt_value_for_transport(TransportReceiptInput {
            transport: LOCAL_GOSSIP_TRANSPORT,
            operation: "publish",
            decision: "pass",
            node,
            envelope,
            diagnostics: Vec::new(),
            checks: vec![
                ("canonical-envelope-ref".to_owned(), "pass".to_owned()),
                ("content-refs-verified".to_owned(), "pass".to_owned()),
                ("transport-is-not-authority".to_owned(), "pass".to_owned()),
            ],
        }),
    })
}

pub async fn publish_live_gossip(
    sender: &iroh_gossip::api::GossipSender,
    envelope: &Envelope,
    node: &str,
) -> Result<Exchange> {
    validate_name(node, "publisher node")?;
    validate_envelope_identity(envelope)?;
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

pub fn deliver_local_gossip(root: &Path, topic: &str, envelope_ref: &str, receiver_peer: &str) -> Result<Delivery> {
    let root = open_capability_dataspace_root(root)?;
    deliver_local_gossip_with_root(&root, topic, envelope_ref, receiver_peer)
}

pub fn deliver_local_gossip_with_root(
    root: &CapabilityDataspaceRoot,
    topic: &str,
    envelope_ref: &str,
    receiver_peer: &str,
) -> Result<Delivery> {
    validate_name(topic, "topic")?;
    validate_name(receiver_peer, "receiver peer")?;
    validate_ref(envelope_ref, "envelope ref")?;
    let bytes = root.root().read(&envelope_store_path(topic, envelope_ref)?)?;
    let value = parse_canonical_bytes(&bytes)?;
    let actual_ref = canonical_hash(&value)?;
    if actual_ref != envelope_ref {
        return Err(MoltenError::invalid_harness(format!(
            "remote dataspace envelope hashes to {actual_ref}, expected {envelope_ref}"
        )));
    }
    let envelope = parse_envelope(&value)?;
    if envelope.topic != topic {
        return Err(MoltenError::invalid_harness(format!(
            "remote dataspace envelope topic {} does not match requested topic {topic}",
            envelope.topic
        )));
    }
    if envelope.to_peer != receiver_peer && envelope.to_peer != "*" {
        return Err(MoltenError::invalid_harness(format!(
            "remote dataspace envelope target {} does not match receiver {receiver_peer}",
            envelope.to_peer
        )));
    }
    validate_content_refs_available_with_root(root, &envelope.content_refs)?;
    let receipt_value = transport_receipt_value_for_transport(TransportReceiptInput {
        transport: LOCAL_GOSSIP_TRANSPORT,
        operation: "deliver",
        decision: "pass",
        node: receiver_peer,
        envelope: &envelope,
        diagnostics: Vec::new(),
        checks: vec![
            ("canonical-envelope-ref".to_owned(), "pass".to_owned()),
            ("topic-peer-binding".to_owned(), "pass".to_owned()),
            ("content-refs-verified".to_owned(), "pass".to_owned()),
            ("transport-is-not-authority".to_owned(), "pass".to_owned()),
        ],
    });
    Ok(Delivery {
        envelope,
        receipt_value,
    })
}
