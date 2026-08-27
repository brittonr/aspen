use super::*;

pub(super) fn transport_profile(kind: TransportAdapterKind) -> Result<CanonicalTransportProfile> {
    canonical_transport_profile(&TransportProfile {
        schema: TRANSPORT_PROFILE_SCHEMA.to_string(),
        profile_id: format!("content-replication-{}", kind.as_str()),
        profile_ref: PROFILE_REF.to_string(),
        adapter_kind: kind,
        capabilities: vec![TransportCapability::BidirectionalStreams],
        limits: TransportLimits {
            max_listeners: PROFILE_LIMIT,
            max_sessions: PROFILE_LIMIT,
            max_streams_per_session: PROFILE_LIMIT,
            max_frame_bytes: FRAME_LIMIT,
            max_datagram_bytes: DATAGRAM_LIMIT,
            max_queued_events: PROFILE_LIMIT,
            max_queued_bytes: QUEUE_BYTE_LIMIT,
            max_inflight_bytes: INFLIGHT_LIMIT,
            operation_deadline_ticks: DEADLINE_WINDOW,
        },
        non_claims: REQUIRED_TRANSPORT_NON_CLAIMS.to_vec(),
    })
}

pub(super) fn setup_commands(generation: u64) -> Vec<TransportCommand> {
    let protocol = protocol(generation);
    vec![
        TransportCommand::Register {
            operation_id: OPERATION_REF.to_string(),
            descriptor: protocol.clone(),
        },
        TransportCommand::OpenSession {
            operation_id: OPERATION_REF.to_string(),
            session_id: scoped_id(SESSION_REF, generation),
            alpn: protocol.alpn,
            direction: SessionDirection::Outbound,
            peer: peer_refs(),
            observed_tick: INITIAL_TICK,
            deadline_tick: INITIAL_TICK.saturating_add(DEADLINE_WINDOW),
        },
        TransportCommand::OpenStream {
            operation_id: OPERATION_REF.to_string(),
            session_id: scoped_id(SESSION_REF, generation),
            stream_id: scoped_id(STREAM_REF, generation),
            direction: StreamDirection::Bidirectional,
            initial_credit_bytes: INFLIGHT_LIMIT,
        },
    ]
}

pub(super) fn protocol(generation: u64) -> ProtocolDescriptor {
    ProtocolDescriptor {
        schema: TRANSPORT_PROTOCOL_SCHEMA.to_string(),
        protocol_id: PROTOCOL_ID.to_string(),
        version: "v1".to_string(),
        alpn: ALPN.to_string(),
        extension_id: EXTENSION_ID.to_string(),
        service_id: SERVICE_ID.to_string(),
        generation,
        listener_limit: 1,
        requested_capabilities: vec![TransportCapability::BidirectionalStreams],
        framing: FramingProfile {
            profile_id: "content-replication-length-delimited-blake3-v1".to_string(),
            profile_ref: FRAMING_REF.to_string(),
            max_frame_bytes: FRAME_LIMIT,
            length_prefix_bytes: LENGTH_PREFIX_BYTES,
            payload_hash_required: true,
        },
        cleanup_policy: ListenerCleanupPolicy::BoundedDrain {
            grace_ticks: DEADLINE_WINDOW,
        },
        registration_authority_ref: AUTHORITY_REF.to_string(),
        profile_ref: PROFILE_REF.to_string(),
    }
}

pub(super) fn peer_refs() -> PeerIdentityRefs {
    PeerIdentityRefs {
        transport_identity_ref: PEER_REF.to_string(),
        membership_ref: Some(MEMBERSHIP_REF.to_string()),
        application_principal_ref: Some(PRINCIPAL_REF.to_string()),
        trust_decision_ref: Some(TRUST_REF.to_string()),
        capability_authority_ref: Some(CAPABILITY_REF.to_string()),
        bootstrap_policy_ref: None,
    }
}

pub(super) fn scoped_id(reference: &str, generation: u64) -> ScopedTransportId {
    ScopedTransportId {
        opaque_ref: reference.to_string(),
        service_id: SERVICE_ID.to_string(),
        generation,
    }
}

pub(super) fn send_command(
    session_id: &ScopedTransportId,
    stream_id: &ScopedTransportId,
    action: &Action,
    payload: &[u8],
) -> Result<TransportCommand> {
    let payload_bytes = u64::try_from(payload.len())
        .map_err(|_| MoltenError::invalid_harness("replication payload length exceeds u64"))?;
    let attempt = usize::try_from(action.attempt)
        .map_err(|_| MoltenError::invalid_harness("replication attempt exceeds usize"))?;
    Ok(TransportCommand::SendFrame {
        operation_id: action.operation_id.clone(),
        session_id: session_id.clone(),
        stream_id: stream_id.clone(),
        payload_ref: format!("blake3:{}", blake3::hash(payload).to_hex()),
        payload_bytes,
        observed_tick: observed_tick(attempt)?,
    })
}

pub(super) fn observed_tick(position: usize) -> Result<u64> {
    let position =
        u64::try_from(position).map_err(|_| MoltenError::invalid_harness("replication position exceeds u64"))?;
    Ok(INITIAL_TICK.saturating_add(position))
}

pub(super) fn action_payload(action: &Action) -> Vec<u8> {
    format!(
        "{}:{}:{}:{}:{}:{}",
        action.operation_id,
        action.kind.as_str(),
        action.attempt,
        action.content_ref,
        action.source_peer.as_deref().unwrap_or("none"),
        action.target_peer
    )
    .into_bytes()
}
