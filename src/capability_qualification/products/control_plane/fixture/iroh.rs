//! The fixed Iroh transport of the qualification live Raft group: profile,
//! protocol, listener and client inputs with stable per-replica endpoint
//! secrets.

const TRANSPORT_LIMIT: u64 = 8;
const FRAME_LIMIT: u64 = 4_096;
const DATAGRAM_LIMIT: u64 = 1_024;
const QUEUE_BYTE_LIMIT: u64 = 16_384;
const INFLIGHT_BYTE_LIMIT: u64 = 8_192;
const DEADLINE_WINDOW_TICKS: u64 = 64;
const LENGTH_PREFIX_BYTES: u64 = 8;
const VALID_FROM_TICK: u64 = 1;
const VALID_UNTIL_TICK: u64 = 100;
const OBSERVED_TICK: u64 = 10;

fn transport_profile() -> crate::error::Result<crate::fabric_transport::Profile> {
    crate::fabric_transport::profile(&crate::fabric_transport::TransportProfile {
        schema: crate::fabric_transport::TRANSPORT_PROFILE_SCHEMA.to_string(),
        profile_id: "aspen-qualification-live-raft-iroh-v1".to_string(),
        profile_ref: super::reference("transport-profile"),
        adapter_kind: crate::fabric_transport::TransportAdapterKind::IrohLive,
        capabilities: vec![
            crate::fabric_transport::TransportCapability::BidirectionalStreams,
            crate::fabric_transport::TransportCapability::UnidirectionalStreams,
        ],
        limits: crate::fabric_transport::TransportLimits {
            max_listeners: TRANSPORT_LIMIT,
            max_sessions: TRANSPORT_LIMIT,
            max_streams_per_session: TRANSPORT_LIMIT,
            max_frame_bytes: FRAME_LIMIT,
            max_datagram_bytes: DATAGRAM_LIMIT,
            max_queued_events: TRANSPORT_LIMIT,
            max_queued_bytes: QUEUE_BYTE_LIMIT,
            max_inflight_bytes: INFLIGHT_BYTE_LIMIT,
            operation_deadline_ticks: DEADLINE_WINDOW_TICKS,
        },
        non_claims: crate::fabric_transport::REQUIRED_TRANSPORT_NON_CLAIMS.to_vec(),
    })
}

fn protocol(group: &crate::fabric_consistency::Group) -> crate::fabric_transport::ProtocolDescriptor {
    crate::fabric_transport::ProtocolDescriptor {
        schema: crate::fabric_transport::TRANSPORT_PROTOCOL_SCHEMA.to_string(),
        protocol_id: "qualification-live-raft".to_string(),
        version: "v1".to_string(),
        alpn: "molten/qualification-live-raft/1".to_string(),
        extension_id: group.extension_id.clone(),
        service_id: group.service_id.clone(),
        generation: group.service_generation,
        listener_limit: 1,
        requested_capabilities: vec![
            crate::fabric_transport::TransportCapability::BidirectionalStreams,
            crate::fabric_transport::TransportCapability::UnidirectionalStreams,
        ],
        framing: crate::fabric_transport::FramingProfile {
            profile_id: "length-prefixed-blake3-v1".to_string(),
            profile_ref: super::reference("framing-profile"),
            max_frame_bytes: FRAME_LIMIT,
            length_prefix_bytes: LENGTH_PREFIX_BYTES,
            payload_hash_required: true,
        },
        cleanup_policy: crate::fabric_transport::ListenerCleanupPolicy::BoundedDrain {
            grace_ticks: DEADLINE_WINDOW_TICKS,
        },
        registration_authority_ref: super::reference("registration-authority"),
        profile_ref: super::reference("transport-profile"),
    }
}

/// A fixed endpoint secret per replica and purpose: listener identities are
/// stable across the crash and recovered restart.
fn secret(node_id: &str, purpose: &str) -> [u8; crate::fabric_transport::IROH_SECRET_KEY_BYTES] {
    *blake3::hash(format!("aspen-qualification-live-raft/{purpose}-secret/{node_id}").as_bytes()).as_bytes()
}

fn listener_identity_ref(node_id: &str) -> String {
    super::reference(&format!("{node_id}/listener"))
}

pub(in crate::capability_qualification::products::control_plane) fn listener_input(
    group: &crate::fabric_consistency::Group,
    node_id: &str,
) -> crate::error::Result<crate::fabric_transport::ListenerInput> {
    Ok(crate::fabric_transport::ListenerInput {
        profile: transport_profile()?,
        protocol: protocol(group),
        capability: crate::fabric_transport::EndpointCapability::from_secret_bytes(
            secret(node_id, "listener"),
            super::reference(&format!("{node_id}/listener-capability")),
        )?,
        bind_addr: loopback(),
        listener_identity_ref: listener_identity_ref(node_id),
        expected_peer_context_ref: super::reference("peer-context"),
        locator_cohort_ref: super::reference("locator-cohort"),
        disclosure: disclosure(),
        validity: crate::fabric_transport::EndpointValidityCohort {
            cohort_ref: super::reference("validity-cohort"),
            not_before_tick: VALID_FROM_TICK,
            expires_at_tick: VALID_UNTIL_TICK,
        },
        admission: crate::fabric_transport::EndpointAdmissionState::fully_active(),
        observed_tick: OBSERVED_TICK,
        admitted_peers: admitted_peers(node_id)?,
    })
}

/// The listener of `node_id` admits exactly the fixed client endpoints of the
/// other static voters, under this listener's own peer context.
fn admitted_peers(
    node_id: &str,
) -> crate::error::Result<std::collections::BTreeMap<String, crate::fabric_transport::AdmittedPeer>> {
    super::NODES
        .into_iter()
        .filter(|peer_id| *peer_id != node_id)
        .map(|peer_id| {
            let client = crate::fabric_transport::EndpointCapability::from_secret_bytes(
                secret(peer_id, "client"),
                super::reference(&format!("{peer_id}/client-capability")),
            )?;
            Ok((peer_id.to_string(), crate::fabric_transport::AdmittedPeer {
                public_endpoint_identity: client.public_endpoint_identity(),
                peer_context_ref: super::reference("peer-context"),
            }))
        })
        .collect()
}

/// The admitted client of `node_id` towards one peer's published endpoint.
/// Everything but the peer's public identity and descriptor profile ref is
/// bound from the fixed declarations, so a substituted listener is refused.
pub(in crate::capability_qualification::products::control_plane) fn client_input(
    group: &crate::fabric_consistency::Group,
    node_id: &str,
    peer_id: &str,
    endpoint: crate::fabric_transport::Endpoint,
) -> crate::error::Result<crate::fabric_transport::ClientInput> {
    let protocol = protocol(group);
    let expected = crate::fabric_transport::ExpectedEndpointBinding {
        profile_id: "aspen-qualification-live-raft-iroh-v1".to_string(),
        profile_ref: endpoint.descriptor.profile_ref.clone(),
        protocol_id: protocol.protocol_id.clone(),
        protocol_version: protocol.version.clone(),
        alpn: protocol.alpn.clone(),
        extension_id: protocol.extension_id.clone(),
        service_id: protocol.service_id.clone(),
        generation: protocol.generation,
        public_endpoint_identity: endpoint.descriptor.public_endpoint_identity.clone(),
        listener_identity_ref: listener_identity_ref(peer_id),
        peer_context_ref: super::reference("peer-context"),
        observed_tick: OBSERVED_TICK,
    };
    Ok(crate::fabric_transport::ClientInput {
        profile: transport_profile()?,
        protocol,
        capability: crate::fabric_transport::EndpointCapability::from_secret_bytes(
            secret(node_id, "client"),
            super::reference(&format!("{node_id}/client-capability")),
        )?,
        bind_addr: loopback(),
        endpoint,
        expected,
        admission: crate::fabric_transport::EndpointAdmissionState::fully_active(),
        session_ref: session_ref(),
        request_ref: super::reference("client-request"),
    })
}

pub(in crate::capability_qualification::products::control_plane) fn session_ref() -> String {
    super::reference("replica-session")
}

fn disclosure() -> crate::fabric_transport::EndpointDisclosurePolicy {
    crate::fabric_transport::EndpointDisclosurePolicy {
        explicit_handoff_classes: vec![crate::fabric_transport::EndpointLocatorClass::Ip],
        default_readback_redacted: true,
    }
}

fn loopback() -> std::net::SocketAddr {
    std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, 0))
}
