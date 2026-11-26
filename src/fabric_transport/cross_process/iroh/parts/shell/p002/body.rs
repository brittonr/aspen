
// r[impl molten.fabric_transport.cross_process_endpoint]
// r[impl molten.fabric_transport.cross_process_session]
pub async fn exchange_frame(
    input: ClientInput,
    payload: &[u8],
    timeout: std::time::Duration,
) -> crate::error::Result<FrameEvidence> {
    validate_client_input(&input, payload)?;
    let dial_plan = admit_endpoint_import(
        &input.profile.profile,
        &input.protocol,
        &input.endpoint.descriptor,
        &input.expected,
        input.admission,
    )
    .map_err(|issues| validation_error("cross-process endpoint import", &issues))?;
    let payload_bytes = u64::try_from(payload.len())
        .map_err(|_| crate::error::Failure::invalid_harness("cross-process payload size does not fit u64"))?;
    if payload_bytes == 0 || payload_bytes > dial_plan.resources.max_frame_bytes {
        return Err(crate::error::Failure::invalid_harness(
            "cross-process client payload exceeds the admitted frame bound",
        ));
    }
    let mut session = plan_cross_process_session(&dial_plan, &input.session_ref, EndpointParticipantRole::Client)
        .map_err(|issues| validation_error("cross-process client session plan", &issues))?;
    session = apply_cross_process_session_command(&session, &CrossProcessSessionCommand::BeginDial {
        observed_descriptor_ref: input.endpoint.descriptor_ref.clone(),
        callback_generation: input.protocol.generation,
    })
    .map_err(|issues| validation_error("cross-process client dial plan", &issues))?
    .next;

    let alpn = input.protocol.alpn.as_bytes().to_vec();
    let endpoint = bind_explicit_endpoint(input.bind_addr, input.capability, &alpn).await?;
    let endpoint_addr = endpoint_addr(&dial_plan)?;
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

/// Test support: dial `input`'s admitted endpoint offering `alpn` and return the connection without
/// opening a stream, like a peer that stalls after its handshake. The endpoint must outlive it.
#[cfg(test)]
pub(crate) async fn connect_without_exchange(
    input: ClientInput,
    alpn: &[u8],
    timeout: std::time::Duration,
) -> crate::error::Result<(iroh::Endpoint, iroh::endpoint::Connection)> {
    let dial_plan = admit_endpoint_import(
        &input.profile.profile,
        &input.protocol,
        &input.endpoint.descriptor,
        &input.expected,
        input.admission,
    )
    .map_err(|issues| validation_error("cross-process endpoint import", &issues))?;
    let endpoint = bind_explicit_endpoint(input.bind_addr, input.capability, alpn).await?;
    let connection = tokio::time::timeout(timeout, endpoint.connect(endpoint_addr(&dial_plan)?, alpn))
        .await
        .map_err(|_| crate::error::Failure::invalid_harness("cross-process connect timed out"))?
        .map_err(error)?;
    Ok((endpoint, connection))
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
        .map_err(|_| crate::error::Failure::invalid_harness("cross-process stream accept timed out"))?
        .map_err(error)?;
    let payload = read_bounded_frame(&mut receive, session.resources.max_frame_bytes, timeout).await?;
    let payload_bytes = u64::try_from(payload.len())
        .map_err(|_| crate::error::Failure::invalid_harness("cross-process payload size does not fit u64"))?;
    *session = apply_cross_process_session_command(session, &CrossProcessSessionCommand::ReceiveFrame {
        payload_bytes,
        callback_generation: generation,
    })
    .map_err(|issues| validation_error("cross-process server receive", &issues))?
    .next;
    let request_ref = derive_request_ref(&payload)?;
    crate::preserves_rail::validate_content_ref(&request_ref)?;
    write_bounded_frame(&mut send, &payload, session.resources.max_frame_bytes, timeout).await?;
    tokio::time::timeout(timeout, connection.closed())
        .await
        .map_err(|_| crate::error::Failure::invalid_harness("cross-process peer close timed out"))?;
    let payload_ref = frame_ref(&request_ref, &payload);
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
    let expected_remote = endpoint_addr.id;
    let connection = tokio::time::timeout(timeout, endpoint.connect(endpoint_addr, alpn))
        .await
        .map_err(|_| crate::error::Failure::invalid_harness("cross-process connect timed out"))?
        .map_err(error)?;
    // The handshake authenticates the dialed key; the admitted import bound that key to the
    // descriptor's peer context, so the session observes the context of the authenticated endpoint.
    if connection.remote_id() != expected_remote {
        connection.close(IROH_REFUSED_CLOSE_CODE.into(), UNADMITTED_PEER_CLOSE_REASON);
        return Err(crate::error::Failure::invalid_harness(
            "cross-process client reached an endpoint other than the admitted descriptor identity",
        ));
    }
    let remote_transport_identity_ref = blake3_ref(connection.remote_id().to_string().as_bytes());
    *session = apply_cross_process_session_command(session, &CrossProcessSessionCommand::Established {
        observed_peer_context_ref: session.identity.expected_peer_context_ref.clone(),
        callback_generation: generation,
    })
    .map_err(|issues| validation_error("cross-process client establishment", &issues))?
    .next;
    let payload_bytes = u64::try_from(payload.len())
        .map_err(|_| crate::error::Failure::invalid_harness("cross-process payload size does not fit u64"))?;
    *session = apply_cross_process_session_command(session, &CrossProcessSessionCommand::QueueFrame {
        payload_bytes,
        callback_generation: generation,
    })
    .map_err(|issues| validation_error("cross-process client queue", &issues))?
    .next;
    *session = apply_cross_process_session_command(session, &CrossProcessSessionCommand::FrameSubmitted {
        payload_bytes,
        callback_generation: generation,
    })
    .map_err(|issues| validation_error("cross-process client submission", &issues))?
    .next;

    let (mut send, mut receive) = tokio::time::timeout(timeout, connection.open_bi())
        .await
        .map_err(|_| crate::error::Failure::invalid_harness("cross-process stream open timed out"))?
        .map_err(error)?;
    write_bounded_frame(&mut send, payload, session.resources.max_frame_bytes, timeout).await?;
    let acknowledgement = read_bounded_frame(&mut receive, session.resources.max_frame_bytes, timeout).await?;
    if acknowledgement != payload {
        return Err(crate::error::Failure::invalid_harness("cross-process acknowledgement payload mismatch"));
    }
    *session = apply_cross_process_session_command(session, &CrossProcessSessionCommand::AcknowledgeFrame {
        payload_bytes,
        callback_generation: generation,
    })
    .map_err(|issues| validation_error("cross-process acknowledgement", &issues))?
    .next;
    connection.close(IROH_CLOSE_CODE.into(), CLIENT_CLOSE_REASON);
    let payload_ref = frame_ref(request_ref, payload);
    Ok(ClientNetworkFrame {
        frame: NetworkFrame {
            acknowledgement_ref: payload_ref.clone(),
            payload_ref,
            payload_bytes,
        },
        remote_transport_identity_ref,
    })
}
