use crate::*;

pub const FRAME_BYTES: u64 = 128;
pub const REQUEST: &[u8] = include_bytes!("request.bin");
pub const RESPONSE: &[u8] = include_bytes!("response.bin");
pub const PROFILE_ID: &str = "molten-iroh-direct-lan-stream-v1";

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct RoleConfig {
    pub role: String,
    pub case: String,
    pub root: std::path::PathBuf,
    pub generation: u64,
    pub client_generation: u64,
    pub bind_ip: std::net::IpAddr,
    pub bind_port: u16,
}

pub fn bounds() -> molten::fabric_transport::DirectStreamBounds {
    molten::fabric_transport::DirectStreamBounds {
        max_connections: 1,
        max_frame_bytes: FRAME_BYTES,
        max_total_bytes: 4096,
        max_frames: 64,
        max_buffered_bytes: 6 * FRAME_BYTES + 1024,
        idle_timeout_ms: 750,
        total_timeout_ms: 5000,
        cleanup_timeout_ms: 5000,
    }
}

pub fn profile() -> TestResult<molten::fabric_transport::Profile> {
    Ok(molten::fabric_transport::profile(&molten::fabric_transport::TransportProfile {
        schema: molten::fabric_transport::TRANSPORT_PROFILE_SCHEMA.to_owned(),
        profile_id: PROFILE_ID.to_owned(),
        profile_ref: reference("direct-stream-test-profile")?,
        adapter_kind: molten::fabric_transport::TransportAdapterKind::IrohLive,
        capabilities: vec![molten::fabric_transport::TransportCapability::BidirectionalStreams],
        limits: molten::fabric_transport::TransportLimits {
            max_listeners: 1,
            max_sessions: 1,
            max_streams_per_session: 1,
            max_frame_bytes: FRAME_BYTES,
            max_datagram_bytes: 1,
            max_queued_events: 64,
            max_queued_bytes: 4096,
            max_inflight_bytes: 4096,
            operation_deadline_ticks: 64,
        },
        non_claims: molten::fabric_transport::REQUIRED_TRANSPORT_NON_CLAIMS.to_vec(),
    })?)
}

pub fn protocol(generation: u64) -> TestResult<molten::fabric_transport::ProtocolDescriptor> {
    Ok(molten::fabric_transport::ProtocolDescriptor {
        schema: molten::fabric_transport::TRANSPORT_PROTOCOL_SCHEMA.to_owned(),
        protocol_id: "protected-unix-consumer".to_owned(),
        version: "v1".to_owned(),
        alpn: "molten/direct-protected-unix/v1".to_owned(),
        extension_id: "direct-stream-test-extension".to_owned(),
        service_id: "direct-stream-test-service".to_owned(),
        generation,
        listener_limit: 1,
        requested_capabilities: vec![molten::fabric_transport::TransportCapability::BidirectionalStreams],
        framing: molten::fabric_transport::FramingProfile {
            profile_id: "direct-stream-test-framing".to_owned(),
            profile_ref: reference("direct-stream-test-framing")?,
            max_frame_bytes: FRAME_BYTES,
            length_prefix_bytes: 8,
            payload_hash_required: true,
        },
        cleanup_policy: molten::fabric_transport::ListenerCleanupPolicy::BoundedDrain { grace_ticks: 64 },
        registration_authority_ref: reference("direct-stream-registration-authority")?,
        profile_ref: profile()?.profile.profile_ref,
    })
}

pub fn identity(is_client: bool) -> String {
    iroh::SecretKey::from_bytes(&secret(is_client)).public().to_string()
}

pub fn secret(is_client: bool) -> [u8; 32] {
    [if is_client { 0x29 } else { 0x47 }; 32]
}

pub fn capability(is_client: bool) -> TestResult<molten::fabric_transport::EndpointCapability> {
    Ok(molten::fabric_transport::EndpointCapability::from_secret_bytes(
        secret(is_client),
        reference(if is_client {
            "direct-stream-client-capability"
        } else {
            "direct-stream-listener-capability"
        })?,
    )?)
}

pub fn request(generation: u64) -> molten::fabric::FabricPortResult<molten::fabric_transport::DirectStreamRequest> {
    Ok(molten::fabric_transport::DirectStreamRequest {
        request_ref: reference("protected-consumer-request")?,
        operation_ref: reference("protected-consumer-operation")?,
        resource_ref: reference("protected-consumer-resource")?,
        generation,
    })
}

pub fn peer(is_client: bool, generation: u64) -> TestResult<molten::fabric_transport::DirectPeerBinding> {
    let protocol = protocol(generation)?;
    Ok(molten::fabric_transport::DirectPeerBinding {
        endpoint_id: identity(!is_client),
        peer_context_ref: reference("direct-stream-peer-context")?,
        service_id: protocol.service_id,
        alpn: protocol.alpn,
        generation,
        remote_role: if is_client {
            molten::fabric_transport::EndpointParticipantRole::Listener
        } else {
            molten::fabric_transport::EndpointParticipantRole::Client
        },
    })
}

pub fn listener_input(config: &RoleConfig) -> TestResult<molten::fabric_transport::ListenerInput> {
    Ok(molten::fabric_transport::ListenerInput {
        profile: profile()?,
        protocol: protocol(config.generation)?,
        capability: capability(false)?,
        bind_addr: std::net::SocketAddr::new(config.bind_ip, config.bind_port),
        listener_identity_ref: reference("direct-stream-listener")?,
        expected_peer_context_ref: reference("direct-stream-peer-context")?,
        locator_cohort_ref: reference("direct-stream-locator-cohort")?,
        disclosure: molten::fabric_transport::EndpointDisclosurePolicy {
            explicit_handoff_classes: vec![molten::fabric_transport::EndpointLocatorClass::Ip],
            default_readback_redacted: true,
        },
        validity: molten::fabric_transport::EndpointValidityCohort {
            cohort_ref: reference("direct-stream-validity")?,
            not_before_tick: 1,
            expires_at_tick: 100,
        },
        admission: molten::fabric_transport::EndpointAdmissionState::fully_active(),
        observed_tick: 10,
    })
}

pub fn client_input(
    config: &RoleConfig,
    endpoint: molten::fabric_transport::Endpoint,
) -> TestResult<molten::fabric_transport::ClientInput> {
    let d = &endpoint.descriptor;
    Ok(molten::fabric_transport::ClientInput {
        profile: profile()?,
        protocol: protocol(config.client_generation)?,
        capability: capability(true)?,
        bind_addr: std::net::SocketAddr::new(config.bind_ip, 0),
        expected: molten::fabric_transport::ExpectedEndpointBinding {
            profile_id: d.profile_id.clone(),
            profile_ref: d.profile_ref.clone(),
            protocol_id: d.protocol_id.clone(),
            protocol_version: d.protocol_version.clone(),
            alpn: d.alpn.clone(),
            extension_id: d.extension_id.clone(),
            service_id: d.service_id.clone(),
            generation: d.generation,
            public_endpoint_identity: d.public_endpoint_identity.clone(),
            listener_identity_ref: d.listener_identity_ref.clone(),
            peer_context_ref: d.expected_peer_context_ref.clone(),
            observed_tick: 10,
        },
        endpoint,
        admission: molten::fabric_transport::EndpointAdmissionState::fully_active(),
        session_ref: reference("direct-stream-session")?,
        request_ref: reference("protected-consumer-request")?,
    })
}
