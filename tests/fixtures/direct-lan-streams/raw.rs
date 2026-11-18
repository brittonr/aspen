use crate::*;

pub(crate) fn operation_digest(generation: u64, case: &str) -> TestResult<[u8; 32]> {
    operation_digest_for_bounds(generation, case, config::bounds_for(case))
}

pub(crate) fn operation_digest_for_bounds(
    generation: u64,
    case: &str,
    bounds: molten::fabric_transport::DirectStreamBounds,
) -> TestResult<[u8; 32]> {
    let profile = config::profile(case)?;
    let protocol = config::protocol(generation, case)?;
    let contract = molten::preserves_rail::record("fabric-direct-stream-contract-v1", vec![
        molten::preserves_rail::string(&profile.profile_ref),
        molten::preserves_rail::string(&protocol.protocol_id),
        molten::preserves_rail::string(&protocol.version),
        molten::preserves_rail::string(&protocol.alpn),
        molten::preserves_rail::string(&protocol.service_id),
        molten::preserves_rail::u64_value(protocol.generation),
        molten::preserves_rail::u64_value(bounds.max_connections),
        molten::preserves_rail::u64_value(bounds.max_frame_bytes),
        molten::preserves_rail::u64_value(bounds.max_total_bytes),
        molten::preserves_rail::u64_value(bounds.max_frames),
        molten::preserves_rail::u64_value(bounds.max_buffered_bytes),
        molten::preserves_rail::u64_value(bounds.idle_timeout_ms),
        molten::preserves_rail::u64_value(bounds.total_timeout_ms),
        molten::preserves_rail::u64_value(bounds.cleanup_timeout_ms),
    ]);
    let contract_ref = molten::preserves_rail::canonical_hash(&contract)?;
    let request = config::request(generation)?;
    let operation = molten::preserves_rail::record("fabric-direct-stream-operation-v1", vec![
        molten::preserves_rail::string(&request.request_ref),
        molten::preserves_rail::string(&request.operation_ref),
        molten::preserves_rail::string(&request.resource_ref),
        molten::preserves_rail::u64_value(request.generation),
        molten::preserves_rail::string(contract_ref),
    ]);
    Ok(*blake3::hash(&molten::preserves_rail::canonical_bytes(&operation)?).as_bytes())
}

pub(crate) fn endpoint_addr(handoff: &molten::fabric_transport::Endpoint) -> TestResult<iroh::EndpointAddr> {
    let identity = handoff
        .descriptor
        .public_endpoint_identity
        .strip_prefix("iroh:")
        .ok_or("endpoint identity has wrong transport prefix")?
        .parse::<iroh::EndpointId>()?;
    let mut addrs = Vec::with_capacity(handoff.descriptor.locators.len());
    for locator in &handoff.descriptor.locators {
        if locator.class != molten::fabric_transport::EndpointLocatorClass::Ip {
            return Err("direct test handoff contained non-IP locator".into());
        }
        let ip = locator.value.strip_prefix("ip:").ok_or("non-IP locator value")?.parse::<std::net::SocketAddr>()?;
        addrs.push(iroh::TransportAddr::Ip(ip));
    }
    Ok(iroh::EndpointAddr::from_parts(identity, addrs))
}

pub(crate) async fn header(send: &mut iroh::endpoint::SendStream, kind: u8, argument: u64) -> TestResult {
    let mut bytes = [0_u8; 9];
    bytes[0] = kind;
    bytes[1..].copy_from_slice(&argument.to_be_bytes());
    send.write_all(&bytes).await?;
    Ok(())
}

pub(crate) async fn discard_frame(receive: &mut iroh::endpoint::RecvStream) -> TestResult<(u8, u64)> {
    let mut bytes = [0_u8; 9];
    receive.read_exact(&mut bytes).await?;
    let mut argument = [0_u8; 8];
    argument.copy_from_slice(&bytes[1..]);
    let value = u64::from_be_bytes(argument);
    if bytes[0] == 1 {
        if value > config::FRAME_BYTES {
            return Err("live sender exceeded configured frame size".into());
        }
        let mut expected = [0_u8; 32];
        receive.read_exact(&mut expected).await?;
        let mut payload = vec![0_u8; usize::try_from(value)?];
        receive.read_exact(&mut payload).await?;
        assert_eq!(blake3::hash(&payload).as_bytes(), &expected, "lost ACK fixture must receive real hashed payload");
    }
    Ok((bytes[0], value))
}

async fn bind_endpoint(config: &config::RoleConfig, alpn: &[u8]) -> TestResult<iroh::Endpoint> {
    let mut builder = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .relay_mode(iroh::RelayMode::Disabled)
        .clear_ip_transports()
        .bind_addr(std::net::SocketAddr::new(config.bind_ip, 0))?
        .secret_key(iroh::SecretKey::from_bytes(&config::secret(true)))
        .alpns(vec![alpn.to_vec()]);
    if config.case == "slow" {
        // Make the native QUIC deadline win without changing the admitted application bound.
        let idle = std::time::Duration::from_millis(config::bounds().idle_timeout_ms / 2);
        builder = builder.transport_config(
            iroh::endpoint::QuicTransportConfig::builder().max_idle_timeout(Some(idle.try_into()?)).build(),
        );
    }
    Ok(builder.bind().await?)
}

pub async fn run(config: &config::RoleConfig, handoff: molten::fabric_transport::Endpoint) -> TestResult {
    if config.case == "queue-probe" {
        return refuse_concurrent(config, &handoff, config::protocol(config.generation, &config.case)?.alpn.as_bytes())
            .await;
    }
    let protocol = config::protocol(config.client_generation, &config.case)?;
    let endpoint = bind_endpoint(config, protocol.alpn.as_bytes()).await?;
    let alpn = if config.case == "alpn" {
        b"molten/unregistered-direct-test/v1".as_slice()
    } else {
        protocol.alpn.as_bytes()
    };
    let connected = endpoint.connect(endpoint_addr(&handoff)?, alpn).await;
    if config.case == "alpn" {
        assert!(connected.is_err(), "unsupported ALPN reached an admitted transport connection");
        endpoint.close().await;
        return Ok(());
    }
    let connection = connected?;
    let (mut send, mut receive) = connection.open_bi().await?;
    let digest = operation_digest(config.client_generation, &config.case)?;
    send.write_all(&digest).await?;
    if matches!(config.case.as_str(), "stale" | "restart") {
        assert_ne!(
            digest,
            operation_digest(config.generation, &config.case)?,
            "stale generation matched the current contract"
        );
        // A rejected operation can close the connection before its unaccepted
        // preface is delivered. Require the remote terminal close, not payload
        // from a stream the listener has already denied. The parent separately
        // requires canonical Denied evidence and zero protected socket opens.
        let closed = tokio::time::timeout(std::time::Duration::from_secs(8), connection.closed()).await?;
        match closed {
            iroh::endpoint::ConnectionError::ApplicationClosed(close) => {
                assert_eq!(close.error_code, 1_u8.into(), "stale operation did not receive a terminal denial");
                assert_eq!(close.reason.as_ref(), b"direct-stream-terminal");
            }
            other => return Err(format!("stale operation lacked a remote terminal denial: {other}").into()),
        }
        endpoint.close().await;
        return Ok(());
    }
    let mut observed = [0_u8; 32];
    receive.read_exact(&mut observed).await?;
    assert_eq!(observed, digest, "native operation binding differs from test wire contract");
    if config.case == "concurrent" {
        refuse_concurrent(config, &handoff, protocol.alpn.as_bytes()).await?;
        connection.close(0_u8.into(), b"concurrent-bound-observed");
    } else if config.case == "disconnect" {
        connection.close(0_u8.into(), b"injected-disconnect-after-operation-binding");
    } else {
        inject_fault(&config.case, &mut send, &mut receive).await?;
        // Retain authentication until the live listener resets/closes or times out.
        let _closed = tokio::time::timeout(std::time::Duration::from_secs(8), connection.closed()).await?;
    }
    endpoint.close().await;
    Ok(())
}

async fn refuse_concurrent(
    config: &config::RoleConfig,
    handoff: &molten::fabric_transport::Endpoint,
    alpn: &[u8],
) -> TestResult {
    let excess = bind_endpoint(config, alpn).await?;
    let attempt =
        tokio::time::timeout(std::time::Duration::from_millis(500), excess.connect(endpoint_addr(handoff)?, alpn))
            .await?;
    assert!(attempt.is_err(), "second concurrent connection bypassed the admitted connection bound");
    excess.close().await;
    Ok(())
}

async fn inject_fault(
    case: &str,
    send: &mut iroh::endpoint::SendStream,
    receive: &mut iroh::endpoint::RecvStream,
) -> TestResult {
    match case {
        "over-bound" => {
            header(send, 1, config::FRAME_BYTES + 1).await?;
            // No payload allocation or successful output is supplied for the oversized claim.
        }
        "slow" => {
            send.write_all(&[1]).await?;
        }
        "ack-loss" => {
            // Real local service bytes must arrive. Read DATA but deliberately never ACK it.
            let frame = discard_frame(receive).await?;
            assert_eq!(frame.0, 1, "ack-loss requires actual submitted DATA, not merely idle EOF");
            assert!(frame.1 > 0);
        }
        _ => return Err("unsupported raw-peer case".into()),
    }
    Ok(())
}
