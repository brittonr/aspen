//! Live listener serve and loopback with bounded event scans.

use n0_future::StreamExt;

const LOOPBACK_LISTENER_MAX_EVENTS: u64 = 4;

const LOOPBACK_LISTENER_TIMEOUT_MS: u64 = 1_000;

/// Bind the live listener, then serve it until its event bound or timeout.
pub async fn serve_control_live_listener(
    input: &super::requests::ControlLiveServeInput<'_>,
) -> crate::error::Result<super::transits::ControlLiveServe> {
    super::attach::bind_control_live_listener(input).await?.serve().await
}

pub async fn control_live_serve_listener_loopback(
    input: &super::requests::ControlLiveServeLoopbackInput<'_>,
) -> crate::error::Result<super::transits::ControlLiveServeLoopback> {
    let state_root = crate::node_state::DirectoryRoot::open(input.state_root)?;
    super::storage::validate_state_root(input.state_root)?;
    super::storage::ensure_state_layout(&state_root)?;
    let envelope = super::ingresses::control_live_ingress_envelope(&loopback_envelope_input(input))?;
    let LoopbackPair {
        ticket_ref,
        ticket_value,
        bound_endpoint_id,
        mut receiver_topic,
        sender,
        receiver_router,
        sender_router,
        node_id,
        endpoint_id,
    } = loopback_pair(&state_root, input.topic).await?;
    let published = super::publishing::publish_control_live_ingress(&super::requests::ControlLiveIngressPublishInput {
        sender: &sender,
        envelope_value: &envelope.value,
        node_id: input.from_peer,
        topology_profile_ref: None,
        transport_profile_ref: None,
        effective_max_attempts: None,
        effective_join_timeout_ms: None,
    })
    .await?;
    let listener_input = super::requests::ControlLiveServeInput {
        state_root: input.state_root,
        topic: input.topic,
        max_events: LOOPBACK_LISTENER_MAX_EVENTS,
        event_timeout_ms: LOOPBACK_LISTENER_TIMEOUT_MS,
        max_requests_per_tick: input.max_requests_per_tick,
        supervisor_policy_value: None,
    };
    let mut listener = super::endpoints::serve_node_control_live_listener_with_topic(
        &state_root,
        &listener_input,
        &mut receiver_topic,
        super::endpoints::EndpointIdentityInput {
            node_id: &node_id,
            logical_endpoint_id: &endpoint_id,
            bound_endpoint_id: &bound_endpoint_id,
        },
    )
    .await?;
    listener.live_ticket_ref = Some(ticket_ref);
    listener.live_ticket_value = Some(ticket_value);
    receiver_router.shutdown().await.map_err(|error| {
        crate::error::Failure::invalid_harness(format!("live Iroh listener receiver shutdown failed: {error}"))
    })?;
    sender_router.shutdown().await.map_err(|error| {
        crate::error::Failure::invalid_harness(format!("live Iroh listener sender shutdown failed: {error}"))
    })?;
    Ok(super::transits::ControlLiveServeLoopback {
        envelope_ref: envelope.envelope_ref,
        publish_receipt_ref: published.transport_receipt_ref,
        listener,
    })
}

fn loopback_envelope_input<'a>(
    input: &super::requests::ControlLiveServeLoopbackInput<'a>,
) -> super::requests::ControlIngressEnvelopeInput<'a> {
    super::requests::ControlIngressEnvelopeInput {
        request_value: input.request_value,
        from_peer: input.from_peer,
        to_node: input.to_node,
        topic: input.topic,
        sequence: input.sequence,
        peer_bootstrap_refs: input.peer_bootstrap_refs,
        authority_refs: input.authority_refs,
        policy_refs: input.policy_refs,
        resource_refs: input.resource_refs,
        evidence_refs: input.evidence_refs,
    }
}

struct LoopbackPair {
    ticket_ref: String,
    ticket_value: preserves::IOValue,
    bound_endpoint_id: String,
    receiver_topic: iroh_gossip::api::GossipTopic,
    sender: iroh_gossip::api::GossipSender,
    receiver_router: iroh::protocol::Router,
    sender_router: iroh::protocol::Router,
    node_id: String,
    endpoint_id: String,
}

async fn loopback_pair(
    state_root: &crate::node_state::DirectoryRoot,
    topic: &str,
) -> crate::error::Result<LoopbackPair> {
    let identity = crate::node_identity::parse_identity(&super::storage::read_preserves(
        state_root,
        &crate::node_state::RelativePath::parse(super::settings::IDENTITY_FILE)?,
    )?)?;
    let lookup = iroh::address_lookup::memory::MemoryLookup::new();
    let receiver_endpoint = super::endpoints::live_gossip_endpoint(
        &lookup,
        Some(super::endpoints::stable_live_endpoint_secret(state_root, &identity)?),
    )
    .await?;
    let sender_endpoint = super::endpoints::live_gossip_endpoint(&lookup, None).await?;
    let ticket =
        super::endpoints::live_ticket_for_bound_endpoint(state_root, &identity, topic, &receiver_endpoint.addr())?;
    lookup.add_endpoint_info(receiver_endpoint.addr());
    lookup.add_endpoint_info(sender_endpoint.addr());
    let receiver_id = receiver_endpoint.id();
    let sender_id = sender_endpoint.id();
    let bound_endpoint_id = format!("iroh:{receiver_id}");
    let receiver_gossip = iroh_gossip::Gossip::builder().spawn(receiver_endpoint.clone());
    let sender_gossip = iroh_gossip::Gossip::builder().spawn(sender_endpoint.clone());
    let receiver_router = iroh::protocol::Router::builder(receiver_endpoint)
        .accept(iroh_gossip::ALPN, receiver_gossip.clone())
        .spawn();
    let sender_router = iroh::protocol::Router::builder(sender_endpoint)
        .accept(iroh_gossip::ALPN, sender_gossip.clone())
        .spawn();
    let topic_id = super::endpoints::control_live_topic_id(topic);
    let receiver_topic = receiver_gossip.subscribe(topic_id, vec![sender_id]).await.map_err(|error| {
        crate::error::Failure::invalid_harness(format!("live Iroh listener receiver subscribe failed: {error}"))
    })?;
    let sender_topic = sender_gossip.subscribe_and_join(topic_id, vec![receiver_id]).await.map_err(|error| {
        crate::error::Failure::invalid_harness(format!("live Iroh listener sender join failed: {error}"))
    })?;
    let (sender, _unused_receiver) = sender_topic.split();
    Ok(LoopbackPair {
        ticket_ref: ticket.ticket_ref,
        ticket_value: ticket.value,
        bound_endpoint_id,
        receiver_topic,
        sender,
        receiver_router,
        sender_router,
        node_id: identity.node_id.clone(),
        endpoint_id: identity.endpoint_id.clone(),
    })
}

pub(super) struct EventScan {
    pub(super) diagnostics: Vec<String>,
    pub(super) transport_receipt_refs: Vec<String>,
    pub(super) neighbor_events: Vec<String>,
    pub(super) observed_events: u64,
}

pub(super) async fn scan_events(
    state_root: &crate::node_state::DirectoryRoot,
    input: &super::requests::ControlLiveServeInput<'_>,
    receiver: &mut iroh_gossip::api::GossipTopic,
    node_id: &str,
) -> crate::error::Result<EventScan> {
    let event_capacity = usize::try_from(input.max_events).map_err(|_| {
        crate::error::Failure::invalid_harness("node control live listener max events exceeds usize capacity")
    })?;
    let mut diagnostics = Vec::with_capacity(event_capacity.saturating_add(2));
    let mut transport_receipt_refs = Vec::with_capacity(event_capacity);
    let mut neighbor_events = Vec::with_capacity(event_capacity);
    let mut observed_events = 0_u64;
    let timeout = std::time::Duration::from_millis(input.event_timeout_ms);
    for _ in 0..input.max_events {
        let event = match tokio::time::timeout(timeout, receiver.next()).await {
            Ok(Some(Ok(event))) => event,
            Ok(Some(Err(error))) => {
                diagnostics.push(format!("live Iroh serve listener receive failed: {error}"));
                break;
            }
            Ok(None) => break,
            Err(_) => break,
        };
        observed_events += 1;
        match &event {
            iroh_gossip::api::Event::NeighborUp(endpoint) => {
                neighbor_events.push(format!("up:iroh:{endpoint}"));
            }
            iroh_gossip::api::Event::NeighborDown(endpoint) => {
                neighbor_events.push(format!("down:iroh:{endpoint}"));
            }
            iroh_gossip::api::Event::Lagged => diagnostics.push("live Iroh serve listener lagged".to_string()),
            iroh_gossip::api::Event::Received(_) => {
                if let Some(received) = super::receiving::receive_control_live_ingress_event_with_root(
                    state_root,
                    &event,
                    input.topic,
                    node_id,
                )? {
                    transport_receipt_refs.push(received.transport_receipt_ref);
                }
            }
        }
        if !transport_receipt_refs.is_empty() {
            break;
        }
    }
    Ok(EventScan {
        diagnostics,
        transport_receipt_refs,
        neighbor_events,
        observed_events,
    })
}
