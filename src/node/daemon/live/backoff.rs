//! Bounded live publish attempts.

pub(super) async fn publish_with_retries(
    input: &super::offers::ControlLiveSendInput<'_>,
    state_root: Option<&crate::node_state::DirectoryRoot>,
    receiver_addr: &iroh::EndpointAddr,
    ticket: &super::entitlements::ControlLiveTicket,
    envelope: &super::transits::ControlIngressEnvelope,
) -> crate::error::Result<super::sending::SendRetryOutcome> {
    let effective_max_attempts = super::tuning::effective_live_send_max_attempts(input);
    let attempt_capacity = usize::try_from(effective_max_attempts)
        .map_err(|_| crate::error::Failure::invalid_harness("node control live send attempts exceed usize capacity"))?;
    let mut retry_receipt_refs = Vec::with_capacity(attempt_capacity);
    let mut retry_receipt_values = Vec::with_capacity(attempt_capacity);
    let mut diagnostics = Vec::with_capacity(attempt_capacity);
    let mut published = None;
    for attempt in 1..=effective_max_attempts {
        match attempt_control_live_send(input, receiver_addr, envelope).await? {
            Ok(receipt) => {
                published = Some(receipt);
                break;
            }
            Err(diagnostic) => {
                let attempt_diagnostics = vec![format!(
                    "node control live send attempt {attempt}/{} failed: {diagnostic}",
                    effective_max_attempts
                )];
                diagnostics.extend(attempt_diagnostics.iter().cloned());
                let retry_value =
                    super::sends::live_send_retry_receipt_value(&super::sends::LiveSendRetryReceiptValueInput {
                        decision: if attempt == effective_max_attempts {
                            "deny"
                        } else {
                            "fail"
                        },
                        attempt,
                        max_attempts: effective_max_attempts,
                        from_peer: input.from_peer,
                        ticket,
                        envelope,
                        diagnostics: &attempt_diagnostics,
                    })?;
                let retry_ref = crate::preserves_rail::canonical_hash(&retry_value)?;
                if let Some(state_root) = state_root {
                    super::storage::write_preserves(
                        state_root,
                        &super::paths::control_live_send_retry_receipt_path(&retry_ref)?,
                        &retry_value,
                    )?;
                    super::storage::import_artifact(state_root, &retry_value)?;
                }
                retry_receipt_refs.push(retry_ref);
                retry_receipt_values.push(retry_value);
            }
        }
    }
    Ok(super::sending::SendRetryOutcome {
        published,
        retry_receipt_refs,
        retry_receipt_values,
        diagnostics,
    })
}

async fn attempt_control_live_send(
    input: &super::offers::ControlLiveSendInput<'_>,
    receiver_addr: &iroh::EndpointAddr,
    envelope: &super::transits::ControlIngressEnvelope,
) -> crate::error::Result<std::result::Result<super::transits::ControlLiveIngressPublish, String>> {
    let lookup = iroh::address_lookup::memory::MemoryLookup::new();
    lookup.add_endpoint_info(receiver_addr.clone());
    let sender_endpoint = match super::endpoints::live_gossip_endpoint(&lookup, None).await {
        Ok(endpoint) => endpoint,
        Err(error) => return Ok(Err(format!("live Iroh sender endpoint failed: {error}"))),
    };
    lookup.add_endpoint_info(sender_endpoint.addr());
    let sender_gossip = iroh_gossip::Gossip::builder().spawn(sender_endpoint.clone());
    let sender_router = iroh::protocol::Router::builder(sender_endpoint)
        .accept(iroh_gossip::ALPN, sender_gossip.clone())
        .spawn();
    let topic_id = super::endpoints::control_live_topic_id(&envelope.topic);
    let join_timeout = std::time::Duration::from_millis(super::tuning::effective_live_send_join_timeout_ms(input));
    let join_result =
        tokio::time::timeout(join_timeout, sender_gossip.subscribe_and_join(topic_id, vec![receiver_addr.id])).await;
    let mut result = match join_result {
        Err(_) => Err(format!(
            "live Iroh node control send timed out joining topic {} at endpoint {}",
            envelope.topic, receiver_addr.id
        )),
        Ok(Err(error)) => Err(format!(
            "live Iroh node control send join failed for topic {} endpoint {}: {error}",
            envelope.topic, receiver_addr.id
        )),
        Ok(Ok(sender_topic)) => {
            let (sender, _receiver_unused) = sender_topic.split();
            let published =
                super::publishing::publish_control_live_ingress(&super::requests::ControlLiveIngressPublishInput {
                    sender: &sender,
                    envelope_value: &envelope.value,
                    node_id: input.from_peer,
                    topology_profile_ref: super::tuning::selected_topology_profile_ref(input),
                    transport_profile_ref: super::tuning::selected_transport_profile_ref(input),
                    effective_max_attempts: Some(super::tuning::effective_live_send_max_attempts(input)),
                    effective_join_timeout_ms: Some(super::tuning::effective_live_send_join_timeout_ms(input)),
                })
                .await;
            if published.is_ok() {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            published.map_err(|error| format!("live Iroh node control send publish failed: {error}"))
        }
    };
    if let Err(error) = sender_router.shutdown().await {
        let diagnostic = format!("live Iroh sender router shutdown failed: {error}");
        if result.is_ok() {
            return Ok(Err(diagnostic));
        }
        result = result.map_err(|existing| format!("{existing}; {diagnostic}"));
    }
    Ok(result)
}
