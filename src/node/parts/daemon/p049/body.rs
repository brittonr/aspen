
async fn attempt_control_live_send(
    input: &ControlLiveSendInput<'_>,
    receiver_addr: &iroh::EndpointAddr,
    envelope: &ControlIngressEnvelope,
) -> Result<std::result::Result<ControlLiveIngressPublish, String>> {
    let lookup = iroh::address_lookup::memory::MemoryLookup::new();
    lookup.add_endpoint_info(receiver_addr.clone());
    let sender_endpoint = match live_gossip_endpoint(&lookup, None).await {
        Ok(endpoint) => endpoint,
        Err(error) => return Ok(Err(format!("live Iroh sender endpoint failed: {error}"))),
    };
    lookup.add_endpoint_info(sender_endpoint.addr());
    let sender_gossip = iroh_gossip::Gossip::builder().spawn(sender_endpoint.clone());
    let sender_router = iroh::protocol::Router::builder(sender_endpoint)
        .accept(iroh_gossip::ALPN, sender_gossip.clone())
        .spawn();
    let topic_id = control_live_topic_id(&envelope.topic);
    let join_timeout = std::time::Duration::from_millis(effective_live_send_join_timeout_ms(input));
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
            let published = publish_control_live_ingress(&ControlLiveIngressPublishInput {
                sender: &sender,
                envelope_value: &envelope.value,
                node_id: input.from_peer,
                topology_profile_ref: selected_topology_profile_ref(input),
                transport_profile_ref: selected_transport_profile_ref(input),
                effective_max_attempts: Some(effective_live_send_max_attempts(input)),
                effective_join_timeout_ms: Some(effective_live_send_join_timeout_ms(input)),
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
