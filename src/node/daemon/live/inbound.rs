//! Live ingress receive helpers: the first gossip event, receive diagnostics, and denied
//! deliveries.

use n0_future::StreamExt;

pub(super) async fn receive_first_live_ingress_event(
    state_root: &crate::node_state::DirectoryRoot,
    receiver: &mut iroh_gossip::api::GossipTopic,
    topic: &str,
    receiver_node: &str,
) -> crate::error::Result<super::transits::ControlLiveIngressReceive> {
    for _ in 0..super::settings::MAX_CONTROL_LIVE_LISTENER_EVENTS {
        let Some(event) = receiver.next().await else {
            return Err(crate::error::Failure::invalid_harness(
                "live Iroh receiver closed before node control envelope arrived",
            ));
        };
        let event = event
            .map_err(|error| crate::error::Failure::invalid_harness(format!("live Iroh receive failed: {error}")))?;
        if let Some(received) =
            super::receiving::receive_control_live_ingress_event_with_root(state_root, &event, topic, receiver_node)?
        {
            return Ok(received);
        }
    }
    Err(crate::error::Failure::invalid_harness(
        "live Iroh receiver exceeded bounded event scan before node control envelope arrived",
    ))
}

pub(super) fn denied_live_ingress_delivery(
    state_root: &crate::node_state::DirectoryRoot,
    envelope: &super::transits::ControlIngressEnvelope,
    diagnostics: &[String],
) -> crate::error::Result<super::transits::ControlIngressDeliver> {
    let receipt_value = super::ingresses::ingress_receipt_value(&super::ingresses::IngressReceiptValueInput {
        decision: "deny",
        phase: "live-receive-deny",
        transport: &envelope.transport,
        envelope,
        idempotency_receipt_ref: None,
        queue_receipt_ref: None,
        diagnostics,
    })?;
    let ingress_receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    super::storage::write_preserves(
        state_root,
        &super::paths::control_ingress_receipt_path(&envelope.envelope_ref, "deliver")?,
        &receipt_value,
    )?;
    super::storage::import_artifact(state_root, &receipt_value)?;
    Ok(super::transits::ControlIngressDeliver {
        envelope_ref: envelope.envelope_ref.clone(),
        request_ref: envelope.request.request_ref.clone(),
        ingress_receipt_ref,
        ingress_receipt_value: receipt_value,
        idempotency_receipt_ref: None,
        queue_receipt_ref: None,
        has_enqueued: false,
    })
}

pub(super) fn live_receive_diagnostics(
    topic: &str,
    receiver_node: &str,
    envelope: &super::transits::ControlIngressEnvelope,
) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if envelope.transport != super::settings::LIVE_CONTROL_INGRESS_TRANSPORT {
        diagnostics.push(format!(
            "node control live receive requires transport {LIVE_CONTROL_INGRESS_TRANSPORT}, got {}",
            envelope.transport,
            LIVE_CONTROL_INGRESS_TRANSPORT = super::settings::LIVE_CONTROL_INGRESS_TRANSPORT
        ));
    }
    if envelope.topic != topic {
        diagnostics.push(format!(
            "node control live receive topic {} does not match subscribed topic {}",
            envelope.topic, topic
        ));
    }
    if envelope.to_node != receiver_node {
        diagnostics.push(format!(
            "node control live receive target {} does not match receiver {}",
            envelope.to_node, receiver_node
        ));
    }
    if envelope.peer_bootstrap_refs.is_empty() {
        diagnostics.push("node control live receive peer bootstrap refs missing".to_string());
    }
    diagnostics
}
