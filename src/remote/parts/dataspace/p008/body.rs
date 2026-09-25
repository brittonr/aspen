
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
