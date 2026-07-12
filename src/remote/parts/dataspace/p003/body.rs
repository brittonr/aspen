
pub fn transport_receipt_value(input: LocalTransportReceiptInput<'_>) -> IoValue {
    transport_receipt_value_for_transport(TransportReceiptInput {
        transport: LOCAL_GOSSIP_TRANSPORT,
        operation: input.operation,
        decision: input.decision,
        node: input.node,
        envelope: input.envelope,
        diagnostics: input.diagnostics,
        checks: input.checks,
    })
}

pub fn transport_receipt_value_for_transport(input: TransportReceiptInput<'_>) -> IoValue {
    record("remote-dataspace-transport-receipt-v1", vec![
        string(TRANSPORT_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("transport", vec![string(input.transport)]),
        record("envelope", vec![string(&input.envelope.envelope_ref)]),
        record("node", vec![string(input.node)]),
        record("from-peer", vec![string(&input.envelope.from_peer)]),
        record("to-peer", vec![string(&input.envelope.to_peer)]),
        record("topic", vec![string(&input.envelope.topic)]),
        record("delivery-sequence", vec![crate::preserves_rail::u64_value(input.envelope.sequence)]),
        record("operation-ref", vec![string(&input.envelope.operation_ref)]),
        record("content-refs", vec![sequence(input.envelope.content_refs.iter().map(string).collect())]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(
            input
                .checks
                .iter()
                .map(|(name, status)| record("check", vec![string(name), string(status)]))
                .collect(),
        )]),
    ])
}

fn remote_admission_receipt_value(input: AdmissionReceiptInput<'_>) -> IoValue {
    record("remote-dataspace-admission-receipt-v1", vec![
        string(ADMISSION_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("envelope", vec![string(&input.envelope.envelope_ref)]),
        record("transport-receipt", vec![string(input.transport_receipt_ref)]),
        record("operation-ref", vec![string(&input.envelope.operation_ref)]),
        record("peer-bootstrap-refs", vec![sequence(
            input.evidence.peer_bootstrap_refs.iter().map(string).collect(),
        )]),
        record("capability-refs", vec![sequence(input.evidence.capability_refs.iter().map(string).collect())]),
        record("policy-refs", vec![sequence(input.evidence.policy_refs.iter().map(string).collect())]),
        record("resource-refs", vec![sequence(input.evidence.resource_refs.iter().map(string).collect())]),
        record("authority-refs", vec![sequence(input.evidence.authority_refs.iter().map(string).collect())]),
        record("turn-journal-context-refs", vec![sequence(input.turn_context_refs.iter().map(string).collect())]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("peer-bootstrap-binding"), string(input.decision)]),
            record("check", vec![string("capability-policy-binding"), string(input.decision)]),
            record("check", vec![string("resource-binding"), string(input.decision)]),
            record("check", vec![string("authority-binding"), string(input.decision)]),
            record("check", vec![string("transport-is-not-authority"), string("pass")]),
        ])]),
    ])
}

fn validate_delivery_evidence(envelope: &Envelope, evidence: &DeliveryEvidence) -> Result<()> {
    require_non_empty_refs(&evidence.peer_bootstrap_refs, "peer bootstrap ref")?;
    require_non_empty_refs(&evidence.capability_refs, "capability ref")?;
    require_non_empty_refs(&evidence.policy_refs, "policy ref")?;
    require_non_empty_refs(&evidence.resource_refs, "resource ref")?;
    require_non_empty_refs(&evidence.authority_refs, "authority ref")?;
    for capability_ref in &envelope.capability_refs {
        if !evidence.capability_refs.contains(capability_ref) {
            return Err(MoltenError::invalid_harness(format!(
                "remote dataspace capability evidence missing declared capability {capability_ref}"
            )));
        }
    }
    let evidence_refs: Vec<&String> = evidence
        .peer_bootstrap_refs
        .iter()
        .chain(evidence.policy_refs.iter())
        .chain(evidence.resource_refs.iter())
        .chain(evidence.authority_refs.iter())
        .collect();
    for evidence_ref in &envelope.evidence_refs {
        if !evidence_refs.contains(&evidence_ref) {
            return Err(MoltenError::invalid_harness(format!(
                "remote dataspace admission evidence missing declared evidence ref {evidence_ref}"
            )));
        }
    }
    Ok(())
}

fn require_non_empty_refs(refs: &[String], label: &str) -> Result<()> {
    if refs.is_empty() {
        return Err(MoltenError::invalid_harness(format!("missing remote dataspace {label}")));
    }
    validate_refs(refs, label)
}

struct EnvelopeOperationRefInput<'a> {
    from_peer: &'a str,
    from_actor: &'a str,
    to_peer: &'a str,
    topic: &'a str,
    operation: Operation,
    payload: &'a IoValue,
    capability_refs: &'a [String],
    evidence_refs: &'a [String],
    sequence: u64,
}

fn envelope_operation_ref(input: EnvelopeOperationRefInput<'_>) -> Result<String> {
    let scope_ref = crate::delivery_idempotency::remote_topic_scope_ref(input.topic, input.to_peer)?;
    let operation = crate::delivery_idempotency::derive_operation_id(crate::delivery_idempotency::OperationIdInput {
        scope_ref,
        producer: remote_actor_id_parts(input.from_peer, input.from_actor),
        consumer: input.to_peer.to_owned(),
        sequence: input.sequence,
        intent: format!("remote-dataspace-{}", input.operation.as_str()),
        payload_ref: canonical_hash(input.payload)?,
        policy_refs: envelope_policy_refs(input.capability_refs, input.evidence_refs)?,
    })?;
    Ok(operation.operation_ref)
}

fn envelope_policy_refs(capability_refs: &[String], evidence_refs: &[String]) -> Result<Vec<String>> {
    let total = capability_refs
        .len()
        .checked_add(evidence_refs.len())
        .ok_or_else(|| MoltenError::invalid_harness("remote dataspace policy ref count overflow"))?;
    ensure_count_at_most(total, MAX_REPLAY_EVENTS, "remote dataspace operation policy refs")?;
    let mut refs = Vec::with_capacity(total);
    refs.extend(capability_refs.iter().cloned());
    refs.extend(evidence_refs.iter().cloned());
    refs.sort();
    refs.dedup();
    validate_refs(&refs, "operation policy ref")?;
    Ok(refs)
}

fn payload_delivery_sequence(payload: &IoValue) -> Result<u64> {
    if let Some(fields) = payload.collect_simple_record("protocol-message-v1", Some(11)) {
        return record_u64(&fields[8], "sequence");
    }
    Ok(1)
}

fn envelope_value(input: &EnvelopeInput) -> Result<IoValue> {
    let delivery_sequence = payload_delivery_sequence(&input.payload)?;
    let operation_ref = envelope_operation_ref(EnvelopeOperationRefInput {
        from_peer: &input.from_peer,
        from_actor: &input.from_actor,
        to_peer: &input.to_peer,
        topic: &input.topic,
        operation: input.operation,
        payload: &input.payload,
        capability_refs: &input.capability_refs,
        evidence_refs: &input.evidence_refs,
        sequence: delivery_sequence,
    })?;
    Ok(record("remote-dataspace-envelope-v1", vec![
        string(ENVELOPE_SCHEMA),
        record("from-peer", vec![string(&input.from_peer)]),
        record("from-actor", vec![string(&input.from_actor)]),
        record("to-peer", vec![string(&input.to_peer)]),
        record("topic", vec![string(&input.topic)]),
        record("operation", vec![string(input.operation.as_str())]),
        record("payload", vec![input.payload.clone()]),
        record("content-refs", vec![sequence(input.content_refs.iter().map(string).collect())]),
        record("capability-refs", vec![sequence(input.capability_refs.iter().map(string).collect())]),
        record("evidence-refs", vec![sequence(input.evidence_refs.iter().map(string).collect())]),
        record("delivery-sequence", vec![crate::preserves_rail::u64_value(delivery_sequence)]),
        record("operation-ref", vec![string(&operation_ref)]),
    ]))
}

fn validate_envelope_identity(envelope: &Envelope) -> Result<()> {
    let actual_ref = canonical_hash(&envelope.value)?;
    if actual_ref != envelope.envelope_ref {
        return Err(MoltenError::invalid_harness(format!(
            "remote dataspace envelope ref {} does not match canonical ref {actual_ref}",
            envelope.envelope_ref
        )));
    }
    Ok(())
}

fn validate_content_refs_available_with_root(root: &CapabilityDataspaceRoot, refs: &[String]) -> Result<()> {
    for reference in refs {
        validate_ref(reference, "content ref")?;
        let bytes = root.root().read(&blob_store_path(reference)?)?;
        let actual_ref = content_ref_from_bytes(&bytes);
        if actual_ref != *reference {
            return Err(MoltenError::invalid_harness(format!(
                "remote dataspace content ref {reference} hashes to {actual_ref}"
            )));
        }
    }
    Ok(())
}

fn ensure_count_at_most(actual: usize, maximum: usize, label: &str) -> Result<()> {
    crate::bounded::ensure_count_at_most(actual, maximum, label)
}

fn extend_bounded<T>(
    values: &mut impl crate::bounded::VecSink<T>,
    incoming: Vec<T>,
    maximum: usize,
    label: &str,
) -> Result<()>
where
    T: Clone,
{
    crate::bounded::extend_bounded(values, &incoming, maximum, label)
}

fn validate_refs(refs: &[String], label: &str) -> Result<()> {
    for reference in refs {
        validate_ref(reference, label)?;
    }
    Ok(())
}

fn validate_ref(reference: &str, label: &str) -> Result<()> {
    validate_content_ref(reference).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "unsupported {label} {reference}; expected canonical content ref: {error}"
        ))
    })
}

fn validate_name(value: &str, field: &str) -> Result<()> {
    if value.trim().is_empty() || value.contains('\0') || value.contains('/') {
        return Err(MoltenError::invalid_harness(format!("invalid remote dataspace {field} {value:?}")));
    }
    Ok(())
}

fn blob_store_path(reference: &str) -> Result<LocalStorePath> {
    LocalStorePath::parse(&format!("blobs/{}", filename_for_ref(reference)?))
}

fn envelope_store_path(topic: &str, envelope_ref: &str) -> Result<LocalStorePath> {
    topic_store_path(topic)?.join(&filename_for_ref(envelope_ref)?)
}

fn topic_store_path(topic: &str) -> Result<LocalStorePath> {
    let topic_hash = blake3::hash(topic.as_bytes()).to_hex().to_string();
    LocalStorePath::parse(&format!("gossip/topic_{topic_hash}"))
}

#[cfg(test)]
fn blob_path(root: &Path, reference: &str) -> Result<PathBuf> {
    Ok(root.join("blobs").join(filename_for_ref(reference)?))
}

#[cfg(test)]
fn envelope_path(root: &Path, topic: &str, envelope_ref: &str) -> Result<PathBuf> {
    Ok(topic_dir(root, topic).join(filename_for_ref(envelope_ref)?))
}

#[cfg(test)]
fn topic_dir(root: &Path, topic: &str) -> PathBuf {
    let topic_hash = blake3::hash(topic.as_bytes()).to_hex().to_string();
    root.join("gossip").join(format!("topic_{topic_hash}"))
}

fn filename_for_ref(reference: &str) -> Result<String> {
    validate_ref(reference, "local materialized ref")?;
    let hex = content_ref_hex(reference)?;
    Ok(format!("blake3_{hex}.bin"))
}
