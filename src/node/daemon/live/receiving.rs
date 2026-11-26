//! Live ingress receipt from gossip events and raw bytes.

pub fn receive_control_live_ingress_event(
    state_root: &std::path::Path,
    event: &iroh_gossip::api::Event,
    topic: &str,
    receiver_node: &str,
) -> crate::error::Result<Option<super::transits::ControlLiveIngressReceive>> {
    let state_root = crate::node_state::DirectoryRoot::open(state_root)?;
    receive_control_live_ingress_event_with_root(&state_root, event, topic, receiver_node)
}

pub(super) fn receive_control_live_ingress_event_with_root(
    state_root: &crate::node_state::DirectoryRoot,
    event: &iroh_gossip::api::Event,
    topic: &str,
    receiver_node: &str,
) -> crate::error::Result<Option<super::transits::ControlLiveIngressReceive>> {
    match event {
        iroh_gossip::api::Event::Received(message) => receive_control_live_ingress_bytes_with_root(
            state_root,
            topic,
            receiver_node,
            &format!("iroh:{}", message.delivered_from),
            message.content.as_ref(),
        )
        .map(Some),
        iroh_gossip::api::Event::NeighborUp(_)
        | iroh_gossip::api::Event::NeighborDown(_)
        | iroh_gossip::api::Event::Lagged => Ok(None),
    }
}

pub fn receive_control_live_ingress_bytes(
    input: &super::requests::ControlLiveIngressReceiveBytesInput<'_>,
) -> crate::error::Result<super::transits::ControlLiveIngressReceive> {
    let state_root = crate::node_state::DirectoryRoot::open(input.state_root)?;
    super::storage::validate_state_root(input.state_root)?;
    receive_control_live_ingress_bytes_with_root(
        &state_root,
        input.topic,
        input.receiver_node,
        input.delivered_from,
        input.bytes,
    )
}

fn receive_control_live_ingress_bytes_with_root(
    state_root: &crate::node_state::DirectoryRoot,
    topic: &str,
    receiver_node: &str,
    delivered_from: &str,
    bytes: &[u8],
) -> crate::error::Result<super::transits::ControlLiveIngressReceive> {
    super::storage::validate_node_id(topic)?;
    super::storage::validate_node_id(receiver_node)?;
    super::storage::validate_node_id(delivered_from)?;
    super::storage::ensure_state_layout(state_root)?;
    let value = crate::preserves_rail::parse_canonical_bytes(bytes)?;
    let envelope = super::delivery::parse_control_ingress_envelope(&value)?;
    let mut diagnostics = super::inbound::live_receive_diagnostics(topic, receiver_node, &envelope);
    super::writers::write_ingress_envelope_and_verify(state_root, topic, &envelope)?;
    super::storage::import_artifact(state_root, &value)?;
    let delivered = if diagnostics.is_empty() {
        super::delivery::deliver_control_ingress_with_root(state_root, topic, &envelope.envelope_ref)?
    } else {
        super::inbound::denied_live_ingress_delivery(state_root, &envelope, &diagnostics)?
    };
    let ingress_decision = super::draining::ingress_receipt_decision(&delivered.ingress_receipt_value)?;
    if ingress_decision != "pass" {
        diagnostics.push(format!("node control live ingress delivery decision {ingress_decision}"));
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value =
        super::transports::live_transport_receipt_value(&super::transports::LiveTransportReceiptValueInput {
            operation: "receive",
            decision,
            node_id: receiver_node,
            delivered_from: Some(delivered_from),
            envelope: &envelope,
            ingress_receipt_ref: Some(&delivered.ingress_receipt_ref),
            topology_profile_ref: None,
            transport_profile_ref: None,
            effective_max_attempts: None,
            effective_join_timeout_ms: None,
            diagnostics: &diagnostics,
        })?;
    let transport_receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    super::storage::write_preserves(
        state_root,
        &super::paths::control_live_transport_receipt_path(&envelope.envelope_ref, "receive")?,
        &receipt_value,
    )?;
    super::storage::import_artifact(state_root, &receipt_value)?;
    Ok(super::transits::ControlLiveIngressReceive {
        envelope_ref: envelope.envelope_ref,
        transport_receipt_ref,
        transport_receipt_value: receipt_value,
        ingress_receipt_ref: delivered.ingress_receipt_ref,
        ingress_receipt_value: delivered.ingress_receipt_value,
        has_enqueued: delivered.has_enqueued,
    })
}
