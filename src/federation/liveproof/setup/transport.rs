pub const LIMIT: u64 = 1_048_576;
pub const NOW: u64 = 10;
pub const UNTIL: u64 = 1_000;
pub const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

pub fn reference(label: &str) -> String {
    crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
}
pub fn profile() -> crate::error::Result<crate::fabric_transport::Profile> {
    crate::fabric_transport::profile(&crate::fabric_transport::TransportProfile {
        schema: crate::fabric_transport::TRANSPORT_PROFILE_SCHEMA.to_string(),
        profile_id: "federated-pull-live-v1".to_string(),
        profile_ref: reference("federated-pull-live-transport-v1"),
        adapter_kind: crate::fabric_transport::TransportAdapterKind::IrohLive,
        capabilities: vec![
            crate::fabric_transport::TransportCapability::BidirectionalStreams,
            crate::fabric_transport::TransportCapability::UnidirectionalStreams,
        ],
        limits: crate::fabric_transport::TransportLimits {
            max_listeners: 2,
            max_sessions: 8,
            max_streams_per_session: 2,
            max_frame_bytes: LIMIT,
            max_datagram_bytes: 1024,
            max_queued_events: 8,
            max_queued_bytes: LIMIT * 2,
            max_inflight_bytes: LIMIT * 2,
            operation_deadline_ticks: 100,
        },
        non_claims: crate::fabric_transport::REQUIRED_TRANSPORT_NON_CLAIMS.to_vec(),
    })
}
pub fn protocol() -> crate::fabric_transport::ProtocolDescriptor {
    crate::fabric_transport::ProtocolDescriptor {
        schema: crate::fabric_transport::TRANSPORT_PROTOCOL_SCHEMA.to_string(),
        protocol_id: "federated-pull".to_string(),
        version: "v1".to_string(),
        alpn: "molten/federated-pull/1".to_string(),
        extension_id: "federated-pull-live".to_string(),
        service_id: "federated-pull-live".to_string(),
        generation: 1,
        listener_limit: 2,
        requested_capabilities: vec![
            crate::fabric_transport::TransportCapability::BidirectionalStreams,
            crate::fabric_transport::TransportCapability::UnidirectionalStreams,
        ],
        framing: crate::fabric_transport::FramingProfile {
            profile_id: "length-prefixed-blake3-v1".to_string(),
            profile_ref: reference("federated-pull-live-framing-v1"),
            max_frame_bytes: LIMIT,
            length_prefix_bytes: 8,
            payload_hash_required: true,
        },
        cleanup_policy: crate::fabric_transport::ListenerCleanupPolicy::BoundedDrain { grace_ticks: 100 },
        registration_authority_ref: reference("federated-pull-live-registration-v1"),
        profile_ref: reference("federated-pull-live-transport-v1"),
    }
}
fn capability(
    adapter: &crate::fabric_crypto_identity::IrohEd25519<'_>,
) -> crate::error::Result<crate::fabric_transport::cross_process::EndpointCapability> {
    let key = adapter.resolve_or_generate(
        crate::fabric_crypto_identity::KeyPurpose::TransportEndpoint,
        &reference("federated-pull-live-transport-key-policy"),
        true,
    )?;
    let secret = adapter.load_transport_secret(&key.handle.handle)?;
    crate::fabric_transport::cross_process::EndpointCapability::from_secret_bytes(
        secret.to_bytes(),
        key.handle.handle.handle_ref,
    )
}
pub async fn listen(
    role: &str,
    adapter: &crate::fabric_crypto_identity::IrohEd25519<'_>,
) -> crate::error::Result<crate::fabric_transport::cross_process::Listener> {
    crate::fabric_transport::cross_process::Listener::bind(crate::fabric_transport::cross_process::ListenerInput {
        profile: profile()?,
        protocol: protocol(),
        capability: capability(adapter)?,
        bind_addr: std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, 0)),
        listener_identity_ref: reference(role),
        expected_peer_context_ref: reference("federation-live-peer-context"),
        locator_cohort_ref: reference("federation-live-loopback-locators"),
        disclosure: crate::fabric_transport::EndpointDisclosurePolicy {
            explicit_handoff_classes: vec![crate::fabric_transport::EndpointLocatorClass::Ip],
            default_readback_redacted: true,
        },
        validity: crate::fabric_transport::EndpointValidityCohort {
            cohort_ref: reference("federation-live-validity"),
            not_before_tick: 1,
            expires_at_tick: UNTIL,
        },
        admission: crate::fabric_transport::EndpointAdmissionState::fully_active(),
        observed_tick: NOW,
    })
    .await
}
pub fn client(
    endpoint: crate::fabric_transport::cross_process::Endpoint,
    session: &str,
    adapter: &crate::fabric_crypto_identity::IrohEd25519<'_>,
) -> crate::error::Result<crate::fabric_transport::cross_process::ClientInput> {
    let descriptor = &endpoint.descriptor;
    let expected = crate::fabric_transport::ExpectedEndpointBinding {
        profile_id: descriptor.profile_id.clone(),
        profile_ref: descriptor.profile_ref.clone(),
        protocol_id: descriptor.protocol_id.clone(),
        protocol_version: descriptor.protocol_version.clone(),
        alpn: descriptor.alpn.clone(),
        extension_id: descriptor.extension_id.clone(),
        service_id: descriptor.service_id.clone(),
        generation: descriptor.generation,
        public_endpoint_identity: descriptor.public_endpoint_identity.clone(),
        listener_identity_ref: descriptor.listener_identity_ref.clone(),
        peer_context_ref: descriptor.expected_peer_context_ref.clone(),
        observed_tick: NOW,
    };
    Ok(crate::fabric_transport::cross_process::ClientInput {
        profile: profile()?,
        protocol: protocol(),
        capability: capability(adapter)?,
        bind_addr: std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, 0)),
        endpoint,
        expected,
        admission: crate::fabric_transport::EndpointAdmissionState::fully_active(),
        session_ref: session.to_string(),
        request_ref: session.to_string(),
    })
}
pub fn evidence(frame: &crate::fabric_transport::cross_process::FrameEvidence) -> preserves::IOValue {
    crate::preserves_rail::record("federation-live-transport-v1", vec![
        crate::preserves_rail::string(&frame.descriptor_ref),
        crate::preserves_rail::string(&frame.session_ref),
        crate::preserves_rail::string(&frame.request_ref),
        crate::preserves_rail::string(&frame.payload_ref),
        crate::preserves_rail::string(&frame.acknowledgement_ref),
        crate::preserves_rail::string(&frame.remote_transport_identity_ref),
        crate::preserves_rail::u64_value(frame.payload_bytes),
        crate::preserves_rail::u64_value(frame.automatic_retry_count),
        crate::preserves_rail::string(&frame.cleanup_evidence_ref),
    ])
}
