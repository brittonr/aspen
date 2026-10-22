
/// A live listener bound to its endpoint and subscribed to its topic.
///
/// The bound ticket is final once the listener is bound: it names the stable
/// endpoint and the addresses the endpoint actually bound. A shell publishes it
/// before serving, so a separate peer can send while the listener waits.
pub struct BoundControlLiveListener<'a> {
    input: ControlLiveServeInput<'a>,
    state_root: crate::node_state::DirectoryRoot,
    identity: crate::node_identity::Identity,
    router: iroh::protocol::Router,
    topic: iroh_gossip::api::GossipTopic,
    bound_endpoint_id: String,
    live_ticket: ControlLiveTicket,
}

impl BoundControlLiveListener<'_> {
    /// The bound live ticket value.
    pub fn live_ticket_value(&self) -> &IoValue {
        &self.live_ticket.value
    }

    /// The bound Iroh endpoint id, as `iroh:<id>`.
    pub fn bound_endpoint_id(&self) -> &str {
        &self.bound_endpoint_id
    }

    /// Serve until the event bound or the per-event timeout, drain once, and
    /// shut the endpoint down.
    pub async fn serve(mut self) -> Result<ControlLiveServe> {
        let served = serve_node_control_live_listener_with_topic(
            &self.state_root,
            &self.input,
            &mut self.topic,
            EndpointIdentityInput {
                node_id: &self.identity.node_id,
                logical_endpoint_id: &self.identity.endpoint_id,
                bound_endpoint_id: &self.bound_endpoint_id,
            },
        )
        .await;
        self.router
            .shutdown()
            .await
            .map_err(|error| MoltenError::invalid_harness(format!("live Iroh serve router shutdown failed: {error}")))?;
        let mut served = served?;
        served.live_ticket_ref = Some(self.live_ticket.ticket_ref);
        served.live_ticket_value = Some(self.live_ticket.value);
        Ok(served)
    }
}

/// Bind the live endpoint, record the bound ticket, and subscribe to the topic.
// r[impl molten.outside_in.real_stack]
pub async fn bind_control_live_listener<'a>(
    input: &ControlLiveServeInput<'a>,
) -> Result<BoundControlLiveListener<'a>> {
    let state_root = crate::node_state::DirectoryRoot::open(input.state_root)?;
    validate_state_root(input.state_root)?;
    validate_node_id(input.topic)?;
    validate_listener_event_limit(input.max_events)?;
    validate_loop_request_limit(input.max_requests_per_tick)?;
    ensure_state_layout(&state_root)?;
    let identity = crate::node_identity::parse_identity(&read_preserves(
        &state_root,
        &crate::node_state::RelativePath::parse(IDENTITY_FILE)?,
    )?)?;
    let lookup = iroh::address_lookup::memory::MemoryLookup::new();
    let endpoint = live_gossip_endpoint(&lookup, Some(stable_live_endpoint_secret(&state_root, &identity)?)).await?;
    let bound_endpoint_id = format!("iroh:{}", endpoint.id());
    let live_ticket = live_ticket_for_bound_endpoint(&state_root, &identity, input.topic, &endpoint.addr())?;
    lookup.add_endpoint_info(endpoint.addr());
    let gossip = iroh_gossip::Gossip::builder().spawn(endpoint.clone());
    let router = iroh::protocol::Router::builder(endpoint).accept(iroh_gossip::ALPN, gossip.clone()).spawn();
    let topic = gossip
        .subscribe(control_live_topic_id(input.topic), Vec::new())
        .await
        .map_err(|error| MoltenError::invalid_harness(format!("live Iroh serve subscribe failed: {error}")))?;
    Ok(BoundControlLiveListener {
        input: *input,
        state_root,
        identity,
        router,
        topic,
        bound_endpoint_id,
        live_ticket,
    })
}
