//! A bound live listener endpoint.

/// A live listener bound to its endpoint and subscribed to its topic.
///
/// The bound ticket is final once the listener is bound: it names the stable
/// endpoint and the addresses the endpoint actually bound. A shell publishes it
/// before serving, so a separate peer can send while the listener waits.
pub struct BoundControlLiveListener<'a> {
    input: super::requests::ControlLiveServeInput<'a>,
    state_root: crate::node_state::DirectoryRoot,
    identity: crate::node_identity::Identity,
    router: iroh::protocol::Router,
    topic: iroh_gossip::api::GossipTopic,
    bound_endpoint_id: String,
    live_ticket: super::entitlements::ControlLiveTicket,
}

impl BoundControlLiveListener<'_> {
    /// The bound live ticket value.
    pub fn live_ticket_value(&self) -> &preserves::IOValue {
        &self.live_ticket.value
    }

    /// The bound Iroh endpoint id, as `iroh:<id>`.
    pub fn bound_endpoint_id(&self) -> &str {
        &self.bound_endpoint_id
    }

    /// Serve until the event bound or the per-event timeout, drain once, and
    /// shut the endpoint down.
    pub async fn serve(self) -> crate::error::Result<super::transits::ControlLiveServe> {
        self.serve_until(std::future::pending())
            .await?
            .ok_or_else(|| crate::error::Failure::invalid_harness("live listener cancelled without a shutdown signal"))
    }

    /// Finish endpoint shutdown on an explicit owner signal without fabricating a serve receipt.
    pub async fn serve_until<F>(
        mut self,
        shutdown: F,
    ) -> crate::error::Result<Option<super::transits::ControlLiveServe>>
    where
        F: Future<Output = ()>,
    {
        let served = super::endpoints::serve_node_control_live_listener_with_topic(
            &self.state_root,
            &self.input,
            &mut self.topic,
            super::endpoints::EndpointIdentityInput {
                node_id: &self.identity.node_id,
                logical_endpoint_id: &self.identity.endpoint_id,
                bound_endpoint_id: &self.bound_endpoint_id,
            },
        );
        let served = tokio::select! {
            biased;
            () = shutdown => Ok(None),
            result = served => result.map(Some),
        };
        self.router.shutdown().await.map_err(|error| {
            crate::error::Failure::invalid_harness(format!("live Iroh serve router shutdown failed: {error}"))
        })?;
        let Some(mut served) = served? else {
            return Ok(None);
        };
        served.live_ticket_ref = Some(self.live_ticket.ticket_ref);
        served.live_ticket_value = Some(self.live_ticket.value);
        Ok(Some(served))
    }
}

/// Bind the live endpoint, record the bound ticket, and subscribe to the topic.
// r[impl molten.outside_in.real_stack]
pub async fn bind_control_live_listener<'a>(
    input: &super::requests::ControlLiveServeInput<'a>,
) -> crate::error::Result<BoundControlLiveListener<'a>> {
    let state_root = crate::node_state::DirectoryRoot::open(input.state_root)?;
    super::storage::validate_state_root(input.state_root)?;
    super::storage::validate_node_id(input.topic)?;
    super::guards::validate_listener_event_limit(input.max_events)?;
    super::guards::validate_loop_request_limit(input.max_requests_per_tick)?;
    super::storage::ensure_state_layout(&state_root)?;
    let identity = crate::node_identity::parse_identity(&super::storage::read_preserves(
        &state_root,
        &crate::node_state::RelativePath::parse(super::settings::IDENTITY_FILE)?,
    )?)?;
    let lookup = iroh::address_lookup::memory::MemoryLookup::new();
    let endpoint = super::endpoints::live_gossip_endpoint(
        &lookup,
        Some(super::endpoints::stable_live_endpoint_secret(&state_root, &identity)?),
    )
    .await?;
    let bound_endpoint_id = format!("iroh:{}", endpoint.id());
    let live_ticket =
        super::endpoints::live_ticket_for_bound_endpoint(&state_root, &identity, input.topic, &endpoint.addr())?;
    lookup.add_endpoint_info(endpoint.addr());
    let gossip = iroh_gossip::Gossip::builder().spawn(endpoint.clone());
    let router = iroh::protocol::Router::builder(endpoint).accept(iroh_gossip::ALPN, gossip.clone()).spawn();
    let topic =
        gossip
            .subscribe(super::endpoints::control_live_topic_id(input.topic), Vec::new())
            .await
            .map_err(|error| {
                crate::error::Failure::invalid_harness(format!("live Iroh serve subscribe failed: {error}"))
            })?;
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
