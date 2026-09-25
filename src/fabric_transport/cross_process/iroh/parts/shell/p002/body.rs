
// r[impl molten.fabric_transport.cross_process_endpoint]
// r[impl molten.fabric_transport.cross_process_session]
pub async fn exchange_cross_process_frame(
    input: IrohCrossProcessClientInput,
    payload: &[u8],
    timeout: std::time::Duration,
) -> crate::error::Result<CrossProcessFrameEvidence> {
    validate_client_shell_input(&input, payload)?;
    let dial_plan = admit_endpoint_import(
        &input.profile.profile,
        &input.protocol,
        &input.endpoint.descriptor,
        &input.expected,
        input.admission,
    )
    .map_err(|issues| shell_validation_error("cross-process endpoint import", &issues))?;
    let payload_bytes = u64::try_from(payload.len())
        .map_err(|_| crate::error::MoltenError::invalid_harness("cross-process payload size does not fit u64"))?;
    if payload_bytes == 0 || payload_bytes > dial_plan.resources.max_frame_bytes {
        return Err(crate::error::MoltenError::invalid_harness(
            "cross-process client payload exceeds the admitted frame bound",
        ));
    }
    let mut session = plan_cross_process_session(&dial_plan, &input.session_ref, EndpointParticipantRole::Client)
        .map_err(|issues| shell_validation_error("cross-process client session plan", &issues))?;
    session = apply_cross_process_session_command(&session, &CrossProcessSessionCommand::BeginDial {
        observed_descriptor_ref: input.endpoint.descriptor_ref.clone(),
        callback_generation: input.protocol.generation,
    })
    .map_err(|issues| shell_validation_error("cross-process client dial plan", &issues))?
    .next;

    let alpn = input.protocol.alpn.as_bytes().to_vec();
    let endpoint = bind_explicit_endpoint(input.bind_addr, input.capability, &alpn).await?;
    let endpoint_addr = iroh_endpoint_addr(&dial_plan)?;
    let network = run_client_exchange(ClientExchangeInput {
        endpoint: &endpoint,
        endpoint_addr,
        alpn: &alpn,
        session: &mut session,
        request_ref: &input.request_ref,
        payload,
        generation: input.protocol.generation,
        timeout,
    })
    .await;
    endpoint.close().await;
    let exchange = match network {
        Ok(exchange) => exchange,
        Err(error) => {
            let _failed = finalize_failed_session(session, SessionTerminalClass::AdapterFailure)?;
            return Err(error);
        }
    };
    let mut evidence = finalize_successful_session(SessionCloseInput {
        session,
        role: EndpointParticipantRole::Client,
        descriptor_ref: &input.endpoint.descriptor_ref,
        session_ref: &input.session_ref,
        request_ref: &input.request_ref,
        remote_transport_identity_ref: &exchange.remote_transport_identity_ref,
        frame: exchange.frame,
    })?;
    evidence.cleanup_evidence_ref =
        cleanup_ref(&evidence.cleanup_evidence_ref, &input.endpoint.descriptor_ref, input.protocol.generation);
    Ok(evidence)
}

struct NetworkFrame {
    payload_ref: String,
    acknowledgement_ref: String,
    payload_bytes: u64,
}

struct ClientNetworkFrame {
    frame: NetworkFrame,
    remote_transport_identity_ref: String,
}

struct ServerNetworkFrame {
    frame: NetworkFrame,
    request_ref: String,
    payload: Vec<u8>,
}

async fn run_server_exchange<F>(
    connection: &iroh::endpoint::Connection,
    session: &mut CrossProcessSessionState,
    derive_request_ref: F,
    generation: u64,
    timeout: std::time::Duration,
) -> crate::error::Result<ServerNetworkFrame>
where
    F: FnOnce(&[u8]) -> crate::error::Result<String>,
{
    let (mut send, mut receive) = tokio::time::timeout(timeout, connection.accept_bi())
        .await
        .map_err(|_| crate::error::MoltenError::invalid_harness("cross-process stream accept timed out"))?
        .map_err(iroh_error)?;
    let payload = read_bounded_frame(&mut receive, session.resources.max_frame_bytes, timeout).await?;
    let payload_bytes = u64::try_from(payload.len())
        .map_err(|_| crate::error::MoltenError::invalid_harness("cross-process payload size does not fit u64"))?;
    *session = apply_cross_process_session_command(session, &CrossProcessSessionCommand::ReceiveFrame {
        payload_bytes,
        callback_generation: generation,
    })
    .map_err(|issues| shell_validation_error("cross-process server receive", &issues))?
    .next;
    let request_ref = derive_request_ref(&payload)?;
    crate::preserves_rail::validate_content_ref(&request_ref)?;
    write_bounded_frame(&mut send, &payload, session.resources.max_frame_bytes, timeout).await?;
    tokio::time::timeout(timeout, connection.closed())
        .await
        .map_err(|_| crate::error::MoltenError::invalid_harness("cross-process peer close timed out"))?;
    let payload_ref = cross_process_frame_ref(&request_ref, &payload);
    Ok(ServerNetworkFrame {
        frame: NetworkFrame {
            acknowledgement_ref: payload_ref.clone(),
            payload_ref,
            payload_bytes,
        },
        request_ref,
        payload,
    })
}

struct ClientExchangeInput<'a> {
    endpoint: &'a iroh::Endpoint,
    endpoint_addr: iroh::EndpointAddr,
    alpn: &'a [u8],
    session: &'a mut CrossProcessSessionState,
    request_ref: &'a str,
    payload: &'a [u8],
    generation: u64,
    timeout: std::time::Duration,
}

async fn run_client_exchange(input: ClientExchangeInput<'_>) -> crate::error::Result<ClientNetworkFrame> {
    let ClientExchangeInput {
        endpoint,
        endpoint_addr,
        alpn,
        session,
        request_ref,
        payload,
        generation,
        timeout,
    } = input;
    let connection = tokio::time::timeout(timeout, endpoint.connect(endpoint_addr, alpn))
        .await
        .map_err(|_| crate::error::MoltenError::invalid_harness("cross-process connect timed out"))?
        .map_err(iroh_error)?;
    let remote_transport_identity_ref = blake3_ref(connection.remote_id().to_string().as_bytes());
    *session = apply_cross_process_session_command(session, &CrossProcessSessionCommand::Established {
        observed_peer_context_ref: session.identity.expected_peer_context_ref.clone(),
        callback_generation: generation,
    })
    .map_err(|issues| shell_validation_error("cross-process client establishment", &issues))?
    .next;
    let payload_bytes = u64::try_from(payload.len())
        .map_err(|_| crate::error::MoltenError::invalid_harness("cross-process payload size does not fit u64"))?;
    *session = apply_cross_process_session_command(session, &CrossProcessSessionCommand::QueueFrame {
        payload_bytes,
        callback_generation: generation,
    })
    .map_err(|issues| shell_validation_error("cross-process client queue", &issues))?
    .next;
    *session = apply_cross_process_session_command(session, &CrossProcessSessionCommand::FrameSubmitted {
        payload_bytes,
        callback_generation: generation,
    })
    .map_err(|issues| shell_validation_error("cross-process client submission", &issues))?
    .next;

    let (mut send, mut receive) = tokio::time::timeout(timeout, connection.open_bi())
        .await
        .map_err(|_| crate::error::MoltenError::invalid_harness("cross-process stream open timed out"))?
        .map_err(iroh_error)?;
    write_bounded_frame(&mut send, payload, session.resources.max_frame_bytes, timeout).await?;
    let acknowledgement = read_bounded_frame(&mut receive, session.resources.max_frame_bytes, timeout).await?;
    if acknowledgement != payload {
        return Err(crate::error::MoltenError::invalid_harness("cross-process acknowledgement payload mismatch"));
    }
    *session = apply_cross_process_session_command(session, &CrossProcessSessionCommand::AcknowledgeFrame {
        payload_bytes,
        callback_generation: generation,
    })
    .map_err(|issues| shell_validation_error("cross-process acknowledgement", &issues))?
    .next;
    connection.close(IROH_CLOSE_CODE.into(), CLIENT_CLOSE_REASON);
    let payload_ref = cross_process_frame_ref(request_ref, payload);
    Ok(ClientNetworkFrame {
        frame: NetworkFrame {
            acknowledgement_ref: payload_ref.clone(),
            payload_ref,
            payload_bytes,
        },
        remote_transport_identity_ref,
    })
}

async fn bind_explicit_endpoint(
    bind_addr: std::net::SocketAddr,
    capability: IrohEndpointCapability,
    alpn: &[u8],
) -> crate::error::Result<iroh::Endpoint> {
    iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .relay_mode(iroh::RelayMode::Disabled)
        .clear_ip_transports()
        .bind_addr(bind_addr)
        .map_err(iroh_error)?
        .secret_key(capability.secret_key)
        .alpns(vec![alpn.to_vec()])
        .bind()
        .await
        .map_err(iroh_error)
}

async fn write_bounded_frame(
    send: &mut iroh::endpoint::SendStream,
    payload: &[u8],
    max_frame_bytes: u64,
    timeout: std::time::Duration,
) -> crate::error::Result<()> {
    let payload_bytes = u64::try_from(payload.len())
        .map_err(|_| crate::error::MoltenError::invalid_harness("cross-process payload size does not fit u64"))?;
    if payload_bytes == 0 || payload_bytes > max_frame_bytes {
        return Err(crate::error::MoltenError::invalid_harness("cross-process outbound frame exceeds bound"));
    }
    let prefix = payload_bytes.to_be_bytes();
    tokio::time::timeout(timeout, send.write_all(&prefix))
        .await
        .map_err(|_| crate::error::MoltenError::invalid_harness("cross-process frame prefix write timed out"))?
        .map_err(iroh_error)?;
    tokio::time::timeout(timeout, send.write_all(payload))
        .await
        .map_err(|_| crate::error::MoltenError::invalid_harness("cross-process frame payload write timed out"))?
        .map_err(iroh_error)?;
    send.finish().map_err(iroh_error)
}

async fn read_bounded_frame(
    receive: &mut iroh::endpoint::RecvStream,
    max_frame_bytes: u64,
    timeout: std::time::Duration,
) -> crate::error::Result<Vec<u8>> {
    let mut prefix = [0_u8; CROSS_PROCESS_FRAME_PREFIX_BYTES];
    tokio::time::timeout(timeout, receive.read_exact(&mut prefix))
        .await
        .map_err(|_| crate::error::MoltenError::invalid_harness("cross-process frame prefix read timed out"))?
        .map_err(iroh_error)?;
    let payload_bytes = u64::from_be_bytes(prefix);
    if payload_bytes == 0 || payload_bytes > max_frame_bytes {
        return Err(crate::error::MoltenError::invalid_harness("cross-process inbound frame exceeds bound"));
    }
    let payload_len = usize::try_from(payload_bytes)
        .map_err(|_| crate::error::MoltenError::invalid_harness("cross-process frame size does not fit usize"))?;
    let mut payload = vec![0_u8; payload_len];
    tokio::time::timeout(timeout, receive.read_exact(&mut payload))
        .await
        .map_err(|_| crate::error::MoltenError::invalid_harness("cross-process frame payload read timed out"))?
        .map_err(iroh_error)?;
    let trailing = tokio::time::timeout(timeout, receive.read_to_end(0))
        .await
        .map_err(|_| crate::error::MoltenError::invalid_harness("cross-process frame terminal read timed out"))?
        .map_err(iroh_error)?;
    if !trailing.is_empty() {
        return Err(crate::error::MoltenError::invalid_harness("cross-process frame has trailing bytes"));
    }
    Ok(payload)
}

struct SessionCloseInput<'a> {
    session: CrossProcessSessionState,
    role: EndpointParticipantRole,
    descriptor_ref: &'a str,
    session_ref: &'a str,
    request_ref: &'a str,
    remote_transport_identity_ref: &'a str,
    frame: NetworkFrame,
}
