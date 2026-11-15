fn observation(
    connection: &iroh::endpoint::Connection,
    peer: &DirectPeerBinding,
    protocol: &ProtocolDescriptor,
    role: EndpointParticipantRole,
) -> ReceiveObservation {
    ReceiveObservation {
        peer_identity: connection.remote_id().to_string(),
        peer_context_ref: peer.peer_context_ref.clone(),
        service_id: protocol.service_id.clone(),
        alpn: String::from_utf8_lossy(connection.alpn()).into_owned(),
        generation: protocol.generation,
        role,
    }
}

fn validate_request(request: &DirectStreamRequest, protocol: &ProtocolDescriptor) -> crate::fabric::FabricPortResult<()> {
    for reference in [&request.request_ref, &request.operation_ref, &request.resource_ref] {
        crate::preserves_rail::validate_content_ref(reference).map_err(port_error)?;
    }
    if request.generation != protocol.generation {
        return Err(crate::fabric::FabricPortError::malformed("direct request generation mismatch"));
    }
    Ok(())
}

fn validate_bind(address: std::net::SocketAddr) -> crate::fabric::FabricPortResult<()> {
    if address.ip().is_unspecified() || address.ip().is_multicast() {
        return Err(crate::fabric::FabricPortError::malformed("direct profile requires one explicit unicast bind"));
    }
    Ok(())
}

fn observe_path(connection: &iroh::endpoint::Connection) -> Result<DirectStreamPath, DirectStreamTerminal> {
    connection
        .paths()
        .iter()
        .find_map(|path| {
            if !path.is_selected() {
                return None;
            }
            match path.remote_addr() {
                iroh::TransportAddr::Ip(address) => Some(Ok(if address.ip().is_loopback() {
                    DirectStreamPath::DirectLoopback
                } else {
                    DirectStreamPath::DirectIp
                })),
                _ => Some(Err(DirectStreamTerminal::Denied)),
            }
        })
        .unwrap_or(Err(DirectStreamTerminal::Denied))
}

pub(super) async fn bind_endpoint(
    address: std::net::SocketAddr,
    capability: EndpointCapability,
    alpn: &[u8],
    bounds: &DirectStreamBounds,
) -> crate::error::Result<iroh::Endpoint> {
    let window = iroh::endpoint::VarInt::from_u64(bounds.max_frame_bytes * 2 + 512).map_err(error)?;
    let config = iroh::endpoint::QuicTransportConfig::builder()
        .max_concurrent_bidi_streams(1_u8.into())
        .max_concurrent_uni_streams(0_u8.into())
        .stream_receive_window(window)
        .receive_window(window)
        .send_window(bounds.max_frame_bytes * 2 + 512)
        .datagram_receive_buffer_size(None)
        .datagram_send_buffer_size(0)
        .max_idle_timeout(Some(std::time::Duration::from_millis(bounds.idle_timeout_ms).try_into().map_err(error)?))
        .build();
    iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .relay_mode(iroh::RelayMode::Disabled)
        .clear_ip_transports()
        .bind_addr(address)
        .map_err(error)?
        .secret_key(capability.secret_key)
        .alpns(vec![alpn.to_vec()])
        .transport_config(config)
        .bind()
        .await
        .map_err(error)
}

fn port_error(error: impl std::fmt::Display) -> crate::fabric::FabricPortError {
    crate::fabric::FabricPortError::malformed(error.to_string())
}

fn finish_session(
    session: &mut Option<CrossProcessSessionState>,
    terminal: DirectStreamTerminal,
    request: &DirectStreamRequest,
    contract: &str,
) -> crate::fabric::FabricPortResult<()> {
    let Some(state) = session.as_mut() else {
        return Ok(());
    };
    let command = if terminal == DirectStreamTerminal::Clean {
        CrossProcessSessionCommand::Close
    } else {
        CrossProcessSessionCommand::Fail {
            class: match terminal {
                DirectStreamTerminal::Cancelled => SessionTerminalClass::Cancelled,
                DirectStreamTerminal::Timeout => SessionTerminalClass::Timeout,
                _ => SessionTerminalClass::AdapterFailure,
            },
            delivery_definitive: false,
        }
    };
    apply_session(state, command).map_err(|terminal| crate::fabric::FabricPortError::transport(terminal.as_str()))?;
    apply_session(state, CrossProcessSessionCommand::BeginCleanup)
        .map_err(|terminal| crate::fabric::FabricPortError::transport(terminal.as_str()))?;
    let value = crate::preserves_rail::record("fabric-direct-stream-cleanup-v1", vec![
        crate::preserves_rail::string(&request.request_ref),
        crate::preserves_rail::string(contract),
        crate::preserves_rail::string(terminal.as_str()),
        crate::preserves_rail::string("local-connection-close-observed"),
    ]);
    let cleanup_evidence_ref = crate::preserves_rail::canonical_hash(&value).map_err(port_error)?;
    apply_session(state, CrossProcessSessionCommand::CompleteCleanup { cleanup_evidence_ref })
        .map_err(|terminal| crate::fabric::FabricPortError::transport(terminal.as_str()))
}

async fn refuse_excess(endpoint: &iroh::Endpoint) {
    while let Some(incoming) = endpoint.accept().await {
        incoming.refuse();
        tokio::task::yield_now().await;
    }
}
