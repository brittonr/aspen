// r[impl molten.fabric_transport.direct_lan]
pub async fn proxy<S: ProtectedSocketService>(
    outbound: Outbound,
    service: &mut S,
    mut cancel: tokio::sync::watch::Receiver<bool>,
) -> crate::fabric::FabricPortResult<Evidence> {
    let Outbound { input, peer, bounds, request, clock } = outbound;
    validate_direct_stream_bounds(&input.profile.profile, &input.protocol, &bounds)
        .map_err(|issue| crate::fabric::FabricPortError::malformed(issue.code()))?;
    validate_bind(input.bind_addr)?;
    validate_request(&request, &input.protocol)?;
    if peer.remote_role != EndpointParticipantRole::Listener {
        return Err(crate::fabric::FabricPortError::malformed("direct client requires listener peer role"));
    }
    peer.endpoint_id.parse::<iroh::EndpointId>().map_err(port_error)?;
    let started = clock();
    let contract = contract_ref(&input.profile, &input.protocol, &bounds)?;
    let mut progress = Progress::default();
    let mut session = None;
    let descriptor_ref = input.endpoint.descriptor_ref;
    let mut state = DialState { connection: None, endpoint: None, actual_peer: None, path: DirectStreamPath::Unavailable };
    let runtime = dial(Dial {
        profile: input.profile, protocol: input.protocol, capability: input.capability,
        bind_addr: input.bind_addr, descriptor: input.endpoint.descriptor,
        expected: input.expected, admission: input.admission,
        peer: &peer, bounds: &bounds, request: &request, contract: &contract,
    }, service, &mut progress, &mut session, &mut state);
    let terminal = tokio::select! {
        biased;
        _ = cancellation(&mut cancel) => DirectStreamTerminal::Cancelled,
        result = tokio::time::timeout(std::time::Duration::from_millis(bounds.total_timeout_ms), runtime) => {
            match result { Ok(Ok(())) => DirectStreamTerminal::Clean, Ok(Err(terminal)) => terminal,
                Err(_) => DirectStreamTerminal::Timeout }
        }
    };
    let is_reset = terminal != DirectStreamTerminal::Clean && state.connection.is_some();
    let grace = std::time::Duration::from_millis(bounds.cleanup_timeout_ms);
    let is_cleanup = tokio::time::timeout(grace, async {
        if let Some(connection) = &state.connection {
            // On a clean stream the listener owns connection closure. Keeping
            // this endpoint alive lets its final stream ACK reach the listener
            // before that owner closes; simultaneous closes can discard it.
            if is_reset {
                connection.close(1_u8.into(), b"direct-stream-terminal");
            }
            let _closed = connection.closed().await;
        }
        if let Some(endpoint) = state.endpoint {
            endpoint.close().await;
        }
    })
    .await
    .is_ok();
    if is_cleanup {
        finish_session(&mut session, terminal, &request, &contract)?;
    }
    evidence::build(evidence::Input {
        descriptor_ref: &descriptor_ref,
        request: &request,
        peer: &peer,
        actual_peer: state.actual_peer.as_deref(),
        contract_ref: &contract,
        progress: &progress,
        terminal,
        path: state.path,
        duration_ms: elapsed_ms(started, clock())?,
        is_reset,
        is_cleanup,
    })
}

struct DialState {
    connection: Option<iroh::endpoint::Connection>,
    endpoint: Option<iroh::Endpoint>,
    actual_peer: Option<String>,
    path: DirectStreamPath,
}

struct Dial<'a> {
    profile: Profile,
    protocol: ProtocolDescriptor,
    capability: EndpointCapability,
    bind_addr: std::net::SocketAddr,
    descriptor: CrossProcessEndpointDescriptor,
    expected: ExpectedEndpointBinding,
    admission: EndpointAdmissionState,
    peer: &'a DirectPeerBinding,
    bounds: &'a DirectStreamBounds,
    request: &'a DirectStreamRequest,
    contract: &'a str,
}

async fn dial<S: ProtectedSocketService>(
    dial: Dial<'_>, service: &mut S, progress: &mut Progress,
    session: &mut Option<CrossProcessSessionState>, state: &mut DialState,
) -> Result<(), DirectStreamTerminal> {
    let Dial { profile, protocol, capability, bind_addr, descriptor, expected, admission,
        peer, bounds, request, contract } = dial;
        let plan = admit_endpoint_import(
            &profile.profile, &protocol, &descriptor, &expected, admission,
        )
        .map_err(|_| DirectStreamTerminal::Denied)?;
        if plan.locators.iter().any(|locator| locator.class != EndpointLocatorClass::Ip) {
            return Err(DirectStreamTerminal::Denied);
        }
        let address = endpoint_addr(&plan).map_err(|_| DirectStreamTerminal::Denied)?;
        let live = bind_endpoint(bind_addr, capability, protocol.alpn.as_bytes(), bounds)
            .await
            .map_err(|_| DirectStreamTerminal::Disconnected)?;
        state.endpoint = Some(live);
        let accepted = state.endpoint
            .as_ref()
            .ok_or(DirectStreamTerminal::Disconnected)?
            .connect(address, protocol.alpn.as_bytes())
            .await
            .map_err(|_| DirectStreamTerminal::Disconnected)?;
        state.actual_peer = Some(accepted.remote_id().to_string());
        state.connection = Some(accepted);
        state.path = observe_path(state.connection.as_ref().ok_or(DirectStreamTerminal::Disconnected)?)?;
        let connection = state.connection.as_ref().ok_or(DirectStreamTerminal::Disconnected)?;
        let source = connection
            .paths()
            .iter()
            .find_map(|path| match path.remote_addr() {
                iroh::TransportAddr::Ip(address) if path.is_selected() => Some(ReceiveSource::from(address.ip())),
                _ => None,
            })
            .ok_or(DirectStreamTerminal::Denied)?;
        // Outbound is already pinned to an exact imported endpoint; the same
        // private type still prevents bypassing the authority service contract.
        let mut gate = ReceiveGate::new(ReceiveBudget {
            burst: 1,
            refill_ticks: 1,
            max_sources: 1,
        })
        .map_err(|_| DirectStreamTerminal::Denied)?;
        let gated = gate
            .admit_inbound(
                source,
                observation(connection, peer, &protocol, EndpointParticipantRole::Listener),
                0,
            )
            .map_err(|deny| {
                progress.deny_code = Some(deny.code());
                DirectStreamTerminal::Denied
            })?;
        validate_direct_peer(&gated, peer, &protocol, true).map_err(|issue| {
            progress.deny_code = Some(issue.code());
            DirectStreamTerminal::Denied
        })?;
        let (mut send, mut receive) = connection.open_bi().await.map_err(connection_error)?;
        exchange(Exchange {
            send: &mut send, receive: &mut receive, gated, peer, protocol: &protocol,
            bounds, request, contract, descriptor: &descriptor,
        }, service, progress, session).await
}
