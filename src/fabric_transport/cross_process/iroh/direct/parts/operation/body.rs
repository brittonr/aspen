struct Exchange<'a> {
    send: &'a mut iroh::endpoint::SendStream,
    receive: &'a mut iroh::endpoint::RecvStream,
    gated: GatedInbound,
    peer: &'a DirectPeerBinding,
    protocol: &'a ProtocolDescriptor,
    bounds: &'a DirectStreamBounds,
    request: &'a DirectStreamRequest,
    contract: &'a str,
    descriptor: &'a CrossProcessEndpointDescriptor,
}

async fn exchange<S: ProtectedSocketService>(
    exchange: Exchange<'_>, service: &mut S, progress: &mut Progress,
    session: &mut Option<CrossProcessSessionState>,
) -> Result<(), DirectStreamTerminal> {
    let Exchange { send, receive, gated, peer, protocol, bounds, request, contract, descriptor } = exchange;
    if receive.is_0rtt() {
        progress.deny_code = Some(DirectStreamIssue::EarlyData.code());
        return Err(DirectStreamTerminal::Denied);
    }
    let binding = operation_binding(request, contract).map_err(|_| DirectStreamTerminal::Denied)?;
    let idle = std::time::Duration::from_millis(bounds.idle_timeout_ms);
    let exchange = async {
        send.write_all(&binding).await.map_err(write_error)?;
        let mut remote = [0_u8; 32];
        receive.read_exact(&mut remote).await.map_err(exact_read_error)?;
        if binding != remote {
            return Err(DirectStreamTerminal::Denied);
        }
        Ok(())
    };
    tokio::time::timeout(idle, exchange).await.map_err(|_| DirectStreamTerminal::Timeout)??;
    let role = match peer.remote_role {
        EndpointParticipantRole::Client => EndpointParticipantRole::Listener,
        EndpointParticipantRole::Listener => EndpointParticipantRole::Client,
    };
    let mut state = plan_cross_process_session(&dial_plan_from_descriptor(descriptor), &request.request_ref, role)
        .map_err(|_| DirectStreamTerminal::Denied)?;
    let begin = match role {
        EndpointParticipantRole::Client => CrossProcessSessionCommand::BeginDial {
            observed_descriptor_ref: descriptor.descriptor_ref.clone(),
            callback_generation: protocol.generation,
        },
        EndpointParticipantRole::Listener => CrossProcessSessionCommand::BeginAccept {
            observed_descriptor_ref: descriptor.descriptor_ref.clone(),
            callback_generation: protocol.generation,
        },
    };
    apply_session(&mut state, begin)?;
    apply_session(&mut state, CrossProcessSessionCommand::Established {
        observed_peer_context_ref: gated.observation().peer_context_ref.clone(),
        callback_generation: protocol.generation,
    })?;
    *session = Some(state);
    let authority_ref = service.authorize(gated, request).map_err(|_| DirectStreamTerminal::Denied)?;
    crate::preserves_rail::validate_content_ref(&authority_ref).map_err(|_| DirectStreamTerminal::Denied)?;
    progress.authority_ref = Some(authority_ref.clone());
    let admitted = AdmittedOperation {
        request: request.clone(),
        peer: peer.clone(),
        authority_ref,
    };
    service.check_current(&admitted).map_err(|_| DirectStreamTerminal::Denied)?;
    let socket = tokio::time::timeout(idle, service.open(&admitted))
        .await
        .map_err(|_| DirectStreamTerminal::Timeout)?
        .map_err(|error| match error {
            crate::fabric::FabricPortError::CapabilityUnavailable { .. } => DirectStreamTerminal::Denied,
            crate::fabric::FabricPortError::Timeout { .. } => DirectStreamTerminal::Timeout,
            crate::fabric::FabricPortError::Cancelled { .. } => DirectStreamTerminal::Cancelled,
            crate::fabric::FabricPortError::UncertainExternalOutcome { .. } => {
                progress.submitted = true;
                DirectStreamTerminal::Disconnected
            }
            _ => DirectStreamTerminal::Disconnected,
        })?;
    // The public API never accepts an early-data connection, so no mutation can
    // run before Iroh's authenticated handshake has completed.
    let state = session.as_mut().ok_or(DirectStreamTerminal::Malformed)?;
    pump::run(send, receive, socket, pump::Context { service, admitted: &admitted, bounds, progress, session: state }).await
}
