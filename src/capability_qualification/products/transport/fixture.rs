//! Fixed transport profile, protocol, endpoint bindings and the deterministic
//! projections of the runtime receipts both sides observe.

pub(super) fn exchange(
    input: crate::fabric_transport::ClientInput,
) -> crate::error::Result<crate::fabric_transport::FrameEvidence> {
    runtime()?.block_on(crate::fabric_transport::exchange_frame(input, super::PAYLOAD, super::EXCHANGE_TIMEOUT))
}

pub(super) fn runtime() -> crate::error::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(crate::error::Failure::from)
}

pub(super) fn client_input(
    endpoint: crate::fabric_transport::Endpoint,
    expected: crate::fabric_transport::ExpectedEndpointBinding,
) -> crate::error::Result<crate::fabric_transport::ClientInput> {
    Ok(crate::fabric_transport::ClientInput {
        profile: profile()?,
        protocol: protocol(),
        capability: crate::fabric_transport::EndpointCapability::from_secret_bytes(
            super::CLIENT_SECRET,
            reference("qualification client capability"),
        )?,
        bind_addr: std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, 0)),
        endpoint,
        expected,
        admission: crate::fabric_transport::EndpointAdmissionState::fully_active(),
        session_ref: session_ref(),
        request_ref: request_ref(),
    })
}

/// The binding the client independently expects. The listener identity comes
/// from the fixed listener key, never from the handed-off descriptor.
pub(super) fn expected(
    public_endpoint_identity: String,
) -> crate::error::Result<crate::fabric_transport::ExpectedEndpointBinding> {
    let profile = profile()?;
    let protocol = protocol();
    Ok(crate::fabric_transport::ExpectedEndpointBinding {
        profile_id: profile.profile.profile_id.clone(),
        profile_ref: profile.profile.profile_ref.clone(),
        protocol_id: protocol.protocol_id.clone(),
        protocol_version: protocol.version.clone(),
        alpn: protocol.alpn.clone(),
        extension_id: protocol.extension_id.clone(),
        service_id: protocol.service_id.clone(),
        generation: super::GENERATION,
        public_endpoint_identity,
        listener_identity_ref: reference("qualification listener identity"),
        peer_context_ref: reference("qualification expected peer context"),
        observed_tick: super::OBSERVED_TICK,
    })
}

pub(super) fn listener_id(secret: [u8; crate::fabric_transport::IROH_SECRET_KEY_BYTES]) -> String {
    iroh::SecretKey::from_bytes(&secret).public().to_string()
}

pub(super) fn public_identity(secret: [u8; crate::fabric_transport::IROH_SECRET_KEY_BYTES]) -> String {
    format!("iroh:{}", listener_id(secret))
}

/// Deterministic projection of a canonical endpoint descriptor: everything
/// except the OS-assigned loopback locator value and the refs that bind it.
pub(super) fn endpoint_projection(endpoint: &crate::fabric_transport::Endpoint) -> preserves::IOValue {
    let descriptor = &endpoint.descriptor;
    crate::preserves_rail::record("qualification-transport-endpoint-v1", vec![
        crate::preserves_rail::string(&descriptor.schema),
        crate::preserves_rail::string(&descriptor.profile_id),
        crate::preserves_rail::string(&descriptor.profile_ref),
        crate::preserves_rail::string(&descriptor.protocol_id),
        crate::preserves_rail::string(&descriptor.protocol_version),
        crate::preserves_rail::string(&descriptor.alpn),
        crate::preserves_rail::string(&descriptor.service_id),
        crate::preserves_rail::u64_value(descriptor.generation),
        crate::preserves_rail::string(&descriptor.public_endpoint_identity),
        crate::preserves_rail::string(&descriptor.listener_identity_ref),
        crate::preserves_rail::string(&descriptor.locator_cohort_ref),
        crate::preserves_rail::sequence(
            descriptor
                .locators
                .iter()
                .map(|locator| crate::preserves_rail::string(locator.class.as_str()))
                .collect(),
        ),
        crate::preserves_rail::string(&descriptor.framing_profile_ref),
        crate::preserves_rail::string(&descriptor.validity.cohort_ref),
    ])
}

/// Deterministic projection of shell frame evidence (the descriptor/cleanup
/// refs bind the OS-assigned port and are excluded).
pub(super) fn frame_projection(frame: &crate::fabric_transport::FrameEvidence) -> preserves::IOValue {
    crate::preserves_rail::record("qualification-transport-frame-v1", vec![
        crate::preserves_rail::string(format!("{:?}", frame.role)),
        crate::preserves_rail::string(&frame.session_ref),
        crate::preserves_rail::string(&frame.request_ref),
        crate::preserves_rail::string(&frame.payload_ref),
        crate::preserves_rail::string(&frame.acknowledgement_ref),
        crate::preserves_rail::string(&frame.remote_transport_identity_ref),
        crate::preserves_rail::u64_value(frame.payload_bytes),
        crate::preserves_rail::string(format!("{:?}", frame.delivery)),
        crate::preserves_rail::string(format!("{:?}", frame.retry)),
        crate::preserves_rail::u64_value(frame.automatic_retry_count),
        crate::preserves_rail::string(format!("{:?}", frame.terminal_class)),
    ])
}

pub(super) fn profile() -> crate::error::Result<crate::fabric_transport::Profile> {
    crate::fabric_transport::profile(&crate::fabric_transport::TransportProfile {
        schema: crate::fabric_transport::TRANSPORT_PROFILE_SCHEMA.to_string(),
        profile_id: "iroh-cross-process-v1".to_string(),
        profile_ref: reference("qualification transport profile declaration"),
        adapter_kind: crate::fabric_transport::TransportAdapterKind::IrohLive,
        capabilities: vec![
            crate::fabric_transport::TransportCapability::BidirectionalStreams,
            crate::fabric_transport::TransportCapability::UnidirectionalStreams,
        ],
        limits: crate::fabric_transport::TransportLimits {
            max_listeners: super::PROFILE_LIMIT,
            max_sessions: super::PROFILE_LIMIT,
            max_streams_per_session: super::PROFILE_LIMIT,
            max_frame_bytes: super::FRAME_LIMIT,
            max_datagram_bytes: super::DATAGRAM_LIMIT,
            max_queued_events: super::PROFILE_LIMIT,
            max_queued_bytes: super::QUEUE_LIMIT,
            max_inflight_bytes: super::INFLIGHT_LIMIT,
            operation_deadline_ticks: super::DEADLINE_WINDOW,
        },
        non_claims: crate::fabric_transport::REQUIRED_TRANSPORT_NON_CLAIMS.to_vec(),
    })
}

pub(super) fn protocol() -> crate::fabric_transport::ProtocolDescriptor {
    crate::fabric_transport::ProtocolDescriptor {
        schema: crate::fabric_transport::TRANSPORT_PROTOCOL_SCHEMA.to_string(),
        protocol_id: "qualification-echo".to_string(),
        version: "v1".to_string(),
        alpn: "aspen/qualification-echo/1".to_string(),
        extension_id: "qualification-transport-extension".to_string(),
        service_id: "qualification-transport-service".to_string(),
        generation: super::GENERATION,
        listener_limit: 1,
        requested_capabilities: vec![
            crate::fabric_transport::TransportCapability::BidirectionalStreams,
            crate::fabric_transport::TransportCapability::UnidirectionalStreams,
        ],
        framing: crate::fabric_transport::FramingProfile {
            profile_id: "length-prefixed-blake3-v1".to_string(),
            profile_ref: reference("qualification transport framing"),
            max_frame_bytes: super::FRAME_LIMIT,
            length_prefix_bytes: super::LENGTH_PREFIX_BYTES,
            payload_hash_required: true,
        },
        cleanup_policy: crate::fabric_transport::ListenerCleanupPolicy::BoundedDrain {
            grace_ticks: super::DEADLINE_WINDOW,
        },
        registration_authority_ref: reference("qualification transport registration authority"),
        profile_ref: reference("qualification transport profile declaration"),
    }
}

pub(super) fn validity() -> crate::fabric_transport::EndpointValidityCohort {
    crate::fabric_transport::EndpointValidityCohort {
        cohort_ref: reference("qualification endpoint validity cohort"),
        not_before_tick: super::VALID_FROM_TICK,
        expires_at_tick: super::VALID_UNTIL_TICK,
    }
}

pub(super) fn disclosure() -> crate::fabric_transport::EndpointDisclosurePolicy {
    crate::fabric_transport::EndpointDisclosurePolicy {
        explicit_handoff_classes: vec![crate::fabric_transport::EndpointLocatorClass::Ip],
        default_readback_redacted: true,
    }
}

pub(super) fn session_ref() -> String {
    reference("qualification transport session")
}

pub(super) fn request_ref() -> String {
    reference("qualification transport request")
}

pub(super) fn reference(label: &str) -> String {
    crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
}
