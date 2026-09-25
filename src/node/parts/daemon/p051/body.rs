
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
