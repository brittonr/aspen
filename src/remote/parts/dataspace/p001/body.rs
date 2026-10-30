
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

pub enum EnvelopePublication {
    Published(Exchange),
    Denied { receipt_value: IoValue },
}

fn require_publication(outcome: EnvelopePublication) -> Result<Exchange> {
    match outcome {
        EnvelopePublication::Published(exchange) => Ok(exchange),
        EnvelopePublication::Denied { receipt_value } => Err(MoltenError::invalid_harness(format!(
            "remote envelope publication denied: {}", crate::preserves_rail::to_text(&receipt_value)?
        ))),
    }
}

pub fn publish_local_gossip(root: &Path, envelope: &Envelope, node: &str, session: Option<&SenderSessionView<'_>>) -> Result<Exchange> {
    require_publication(publish_local_gossip_with_evidence(root, envelope, node, session)?)
}

pub fn publish_local_gossip_with_evidence(root: &Path, envelope: &Envelope, node: &str, session: Option<&SenderSessionView<'_>>) -> Result<EnvelopePublication> {
    validate_name(node, "publisher node")?;
    if let Some(receipt_value) = outbound_envelope_denial(envelope, session)? {
        return Ok(EnvelopePublication::Denied { receipt_value });
    }
    let root = open_capability_dataspace_root(root)?;
    Ok(EnvelopePublication::Published(publish_local_gossip_admitted(&root, envelope, node)?))
}

pub fn publish_local_gossip_with_root(
    root: &CapabilityDataspaceRoot,
    envelope: &Envelope,
    node: &str,
    session: Option<&SenderSessionView<'_>>,
) -> Result<Exchange> {
    require_publication(publish_local_gossip_with_root_and_evidence(root, envelope, node, session)?)
}

pub fn publish_local_gossip_with_root_and_evidence(
    root: &CapabilityDataspaceRoot, envelope: &Envelope, node: &str, session: Option<&SenderSessionView<'_>>,
) -> Result<EnvelopePublication> {
    validate_name(node, "publisher node")?;
    if let Some(receipt_value) = outbound_envelope_denial(envelope, session)? {
        return Ok(EnvelopePublication::Denied { receipt_value });
    }
    Ok(EnvelopePublication::Published(publish_local_gossip_admitted(root, envelope, node)?))
}

fn publish_local_gossip_admitted(root: &CapabilityDataspaceRoot, envelope: &Envelope, node: &str) -> Result<Exchange> {
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

/// Untrusted incoming wire transport fixture, not sender admission evidence.
#[cfg(test)]
pub(crate) fn incoming_publish_fixture(root: &Path, envelope: &Envelope, node: &str) -> Result<Exchange> {
    validate_name(node, "fixture publisher node")?;
    validate_envelope_identity(envelope)?;
    let root = open_capability_dataspace_root(root)?;
    publish_local_gossip_admitted(&root, envelope, node)
}

pub fn deliver_local_gossip(root: &Path, topic: &str, envelope_ref: &str, receiver_peer: &str) -> Result<Delivery> {
    let root = open_capability_dataspace_root(root)?;
    deliver_local_gossip_with_root(&root, topic, envelope_ref, receiver_peer)
}
