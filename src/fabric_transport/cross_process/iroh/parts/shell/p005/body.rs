
async fn bind_explicit_endpoint(
    bind_addr: std::net::SocketAddr,
    capability: EndpointCapability,
    alpn: &[u8],
) -> crate::error::Result<iroh::Endpoint> {
    iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .relay_mode(iroh::RelayMode::Disabled)
        .clear_ip_transports()
        .bind_addr(bind_addr)
        .map_err(error)?
        .secret_key(capability.secret_key)
        .alpns(vec![alpn.to_vec()])
        .bind()
        .await
        .map_err(error)
}

async fn write_bounded_frame(
    send: &mut iroh::endpoint::SendStream,
    payload: &[u8],
    max_frame_bytes: u64,
    timeout: std::time::Duration,
) -> crate::error::Result<()> {
    let payload_bytes = u64::try_from(payload.len())
        .map_err(|_| crate::error::Failure::invalid_harness("cross-process payload size does not fit u64"))?;
    if payload_bytes == 0 || payload_bytes > max_frame_bytes {
        return Err(crate::error::Failure::invalid_harness("cross-process outbound frame exceeds bound"));
    }
    let prefix = payload_bytes.to_be_bytes();
    tokio::time::timeout(timeout, send.write_all(&prefix))
        .await
        .map_err(|_| crate::error::Failure::invalid_harness("cross-process frame prefix write timed out"))?
        .map_err(error)?;
    tokio::time::timeout(timeout, send.write_all(payload))
        .await
        .map_err(|_| crate::error::Failure::invalid_harness("cross-process frame payload write timed out"))?
        .map_err(error)?;
    send.finish().map_err(error)
}

async fn read_bounded_frame(
    receive: &mut iroh::endpoint::RecvStream,
    max_frame_bytes: u64,
    timeout: std::time::Duration,
) -> crate::error::Result<Vec<u8>> {
    let mut prefix = [0_u8; CROSS_PROCESS_FRAME_PREFIX_BYTES];
    tokio::time::timeout(timeout, receive.read_exact(&mut prefix))
        .await
        .map_err(|_| crate::error::Failure::invalid_harness("cross-process frame prefix read timed out"))?
        .map_err(error)?;
    let payload_bytes = u64::from_be_bytes(prefix);
    if payload_bytes == 0 || payload_bytes > max_frame_bytes {
        return Err(crate::error::Failure::invalid_harness("cross-process inbound frame exceeds bound"));
    }
    let payload_len = usize::try_from(payload_bytes)
        .map_err(|_| crate::error::Failure::invalid_harness("cross-process frame size does not fit usize"))?;
    let mut payload = vec![0_u8; payload_len];
    tokio::time::timeout(timeout, receive.read_exact(&mut payload))
        .await
        .map_err(|_| crate::error::Failure::invalid_harness("cross-process frame payload read timed out"))?
        .map_err(error)?;
    let trailing = tokio::time::timeout(timeout, receive.read_to_end(0))
        .await
        .map_err(|_| crate::error::Failure::invalid_harness("cross-process frame terminal read timed out"))?
        .map_err(error)?;
    if !trailing.is_empty() {
        return Err(crate::error::Failure::invalid_harness("cross-process frame has trailing bytes"));
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
