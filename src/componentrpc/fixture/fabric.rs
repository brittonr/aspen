//! Isolated fixture fabric configuration; no existing service registration is changed.

fn reference(label: &str) -> String {
    super::reference(label.as_bytes())
}
pub fn peer(byte: u8) -> String {
    super::reference(iroh::SecretKey::from_bytes(&[byte; 32]).public().to_string().as_bytes())
}
fn capability(byte: u8) -> super::Result<crate::fabric_transport::EndpointCapability> {
    crate::fabric_transport::EndpointCapability::from_secret_bytes(
        [byte; 32],
        reference(&format!("fixture-endpoint-{byte}")),
    )
    .map_err(super::failure)
}
fn address() -> std::net::SocketAddr {
    std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, 0))
}
fn profile() -> super::Result<crate::fabric_transport::Profile> {
    crate::fabric_transport::profile(&crate::fabric_transport::TransportProfile {
        schema: crate::fabric_transport::TRANSPORT_PROFILE_SCHEMA.into(),
        profile_id: super::PROFILE.into(),
        profile_ref: reference("component-rpc-loopback-transport-v1"),
        adapter_kind: crate::fabric_transport::TransportAdapterKind::IrohLive,
        capabilities: vec![crate::fabric_transport::TransportCapability::BidirectionalStreams],
        limits: crate::fabric_transport::TransportLimits {
            max_listeners: 2,
            max_sessions: 2,
            max_streams_per_session: 1,
            max_frame_bytes: super::MAX_BYTES as u64,
            max_datagram_bytes: 1,
            max_queued_events: 8,
            max_queued_bytes: 16384,
            max_inflight_bytes: 8192,
            operation_deadline_ticks: 64,
        },
        non_claims: crate::fabric_transport::REQUIRED_TRANSPORT_NON_CLAIMS.to_vec(),
    })
    .map_err(super::failure)
}
fn protocol(lease: &super::authority::Lease) -> crate::fabric_transport::ProtocolDescriptor {
    crate::fabric_transport::ProtocolDescriptor {
        schema: crate::fabric_transport::TRANSPORT_PROTOCOL_SCHEMA.into(),
        protocol_id: super::PROFILE.into(),
        version: "v1".into(),
        alpn: "molten/component-rpc/hello/1".into(),
        extension_id: "component-rpc-pilot".into(),
        service_id: "component-rpc-unary".into(),
        generation: 1,
        listener_limit: 2,
        requested_capabilities: vec![crate::fabric_transport::TransportCapability::BidirectionalStreams],
        framing: crate::fabric_transport::FramingProfile {
            profile_id: "length-prefixed-blake3-v1".into(),
            profile_ref: reference("component-rpc-length-prefixed-blake3-v1"),
            max_frame_bytes: super::MAX_BYTES as u64,
            length_prefix_bytes: 8,
            payload_hash_required: true,
        },
        cleanup_policy: crate::fabric_transport::ListenerCleanupPolicy::BoundedDrain { grace_ticks: 64 },
        registration_authority_ref: lease.receipt().into(),
        profile_ref: reference("component-rpc-loopback-transport-v1"),
    }
}

/// A listener keyed by `byte` that admits only the endpoint keyed by `caller`, under its own
/// context.
pub async fn listener(
    lease: &super::authority::Lease,
    byte: u8,
    caller: u8,
) -> super::Result<crate::fabric_transport::Listener> {
    lease.check(10, 1)?;
    let admitted_caller = crate::fabric_transport::AdmittedPeer {
        public_endpoint_identity: capability(caller)?.public_endpoint_identity(),
        peer_context_ref: peer(byte),
    };
    crate::fabric_transport::Listener::bind(crate::fabric_transport::ListenerInput {
        profile: profile()?,
        protocol: protocol(lease),
        capability: capability(byte)?,
        bind_addr: address(),
        listener_identity_ref: reference(&format!("rpc-listener-{byte}")),
        expected_peer_context_ref: peer(byte),
        locator_cohort_ref: reference("rpc-loopback-ip-locators-v1"),
        disclosure: crate::fabric_transport::EndpointDisclosurePolicy {
            explicit_handoff_classes: vec![crate::fabric_transport::EndpointLocatorClass::Ip],
            default_readback_redacted: true,
        },
        validity: crate::fabric_transport::EndpointValidityCohort {
            cohort_ref: reference("rpc-loopback-ticks-1-100"),
            not_before_tick: 1,
            expires_at_tick: 100,
        },
        admission: crate::fabric_transport::EndpointAdmissionState::fully_active(),
        observed_tick: 10,
        admitted_peers: std::collections::BTreeMap::from([(format!("rpc-caller-{caller}"), admitted_caller)]),
    })
    .await
    .map_err(super::failure)
}

pub fn client(
    lease: &super::authority::Lease,
    endpoint: crate::fabric_transport::Endpoint,
    byte: u8,
    session: String,
    request: String,
) -> super::Result<crate::fabric_transport::ClientInput> {
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
        observed_tick: 10,
    };
    Ok(crate::fabric_transport::ClientInput {
        profile: profile()?,
        protocol: protocol(lease),
        capability: capability(byte)?,
        bind_addr: address(),
        endpoint,
        expected,
        admission: crate::fabric_transport::EndpointAdmissionState::fully_active(),
        session_ref: session,
        request_ref: request,
    })
}
