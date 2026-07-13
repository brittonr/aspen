use super::*;

const NO_AUTOMATIC_RETRIES: u64 = 0;
const EVENT_SLOT: u64 = 1;
const FIRST_SEQUENCE: u64 = 0;

// r[impl molten.fabric_transport.port_contract]
// r[impl molten.fabric_transport.protocol_registration]
// r[impl molten.fabric_transport.session_streams]
// r[impl molten.fabric_transport.flow_control]
// r[impl molten.fabric_transport.identity_separation]
// r[impl molten.fabric_transport.failure_semantics]
pub fn apply_transport_command(
    profile: &TransportProfile,
    state: &TransportState,
    command: &TransportCommand,
) -> Result<TransportTransition, Vec<TransportIssue>> {
    let mut issues = validate_transport_profile(profile).err().unwrap_or_default();
    validate_operation_id(command.operation_id(), &mut issues);
    if !issues.is_empty() {
        return Err(issues);
    }

    match command {
        TransportCommand::Register {
            operation_id,
            descriptor,
        } => register_protocol(profile, state, operation_id, descriptor),
        TransportCommand::TransferOwnership {
            operation_id,
            descriptor,
            prior_generation,
            cleanup_evidence_ref,
        } => transfer_protocol(profile, state, operation_id, descriptor, *prior_generation, cleanup_evidence_ref),
        TransportCommand::BeginDrain {
            operation_id,
            alpn,
            service_id,
            generation,
        } => begin_drain(state, operation_id, alpn, service_id, *generation),
        TransportCommand::CleanupListener {
            operation_id,
            alpn,
            service_id,
            generation,
            cleanup_evidence_ref,
        } => cleanup_listener(state, operation_id, alpn, service_id, *generation, cleanup_evidence_ref),
        TransportCommand::OpenSession {
            operation_id,
            session_id,
            alpn,
            direction,
            peer,
            observed_tick,
            deadline_tick,
        } => open_session(
            profile,
            state,
            operation_id,
            session_id,
            alpn,
            *direction,
            peer,
            *observed_tick,
            *deadline_tick,
        ),
        TransportCommand::OpenStream {
            operation_id,
            session_id,
            stream_id,
            direction,
            initial_credit_bytes,
        } => open_stream(profile, state, operation_id, session_id, stream_id, *direction, *initial_credit_bytes),
        TransportCommand::SendFrame {
            operation_id,
            session_id,
            stream_id,
            payload_ref,
            payload_bytes,
            observed_tick,
        } => {
            send_frame(profile, state, operation_id, session_id, stream_id, payload_ref, *payload_bytes, *observed_tick)
        }
        TransportCommand::ReceiveFrame {
            operation_id,
            session_id,
            stream_id,
            payload_ref,
            payload_bytes,
            sequence,
            observed_tick,
        } => receive_frame(
            profile,
            state,
            operation_id,
            session_id,
            stream_id,
            payload_ref,
            *payload_bytes,
            *sequence,
            *observed_tick,
        ),
        TransportCommand::AcknowledgeFrame {
            operation_id,
            session_id,
            stream_id,
            payload_bytes,
        } => acknowledge_frame(profile, state, operation_id, session_id, stream_id, *payload_bytes),
        TransportCommand::SendDatagram {
            operation_id,
            session_id,
            payload_ref,
            payload_bytes,
            observed_tick,
        } => send_datagram(profile, state, operation_id, session_id, payload_ref, *payload_bytes, *observed_tick),
        TransportCommand::CompleteDatagram {
            operation_id,
            session_id,
            payload_bytes,
            delivered,
        } => complete_datagram(state, operation_id, session_id, *payload_bytes, *delivered),
        TransportCommand::GrantCredit {
            operation_id,
            session_id,
            stream_id,
            credit_bytes,
        } => grant_credit(profile, state, operation_id, session_id, stream_id, *credit_bytes),
        TransportCommand::HalfCloseStream {
            operation_id,
            session_id,
            stream_id,
            send_direction,
        } => half_close_stream(state, operation_id, session_id, stream_id, *send_direction),
        TransportCommand::CloseStream {
            operation_id,
            session_id,
            stream_id,
        } => close_stream(state, operation_id, session_id, stream_id),
        TransportCommand::CloseSession {
            operation_id,
            session_id,
        } => close_session(state, operation_id, session_id),
        TransportCommand::Cancel { operation_id, target } => cancel(state, operation_id, target),
        TransportCommand::FailSession {
            operation_id,
            session_id,
            class,
            delivery_definitive,
        } => fail_session(state, operation_id, session_id, *class, *delivery_definitive),
    }
}

fn register_protocol(
    profile: &TransportProfile,
    state: &TransportState,
    operation_id: &str,
    descriptor: &ProtocolDescriptor,
) -> Result<TransportTransition, Vec<TransportIssue>> {
    validate_protocol_descriptor(profile, descriptor)?;
    let mut issues = Vec::new();
    if state.protocols.contains_key(&descriptor.alpn) {
        issues.push(TransportIssue::DuplicateProtocol);
    }
    if state.protocols.values().any(|registered| {
        registered.descriptor.protocol_id == descriptor.protocol_id
            && registered.descriptor.version == descriptor.version
    }) {
        issues.push(TransportIssue::ConflictingProtocolIdentity);
    }
    let listener_count = count_active_protocols(state)?;
    if listener_count >= profile.limits.max_listeners {
        issues.push(TransportIssue::ListenerLimitExceeded);
    }
    if !issues.is_empty() {
        return Err(issues);
    }

    let mut next = state.clone();
    next.protocols.insert(descriptor.alpn.clone(), RegisteredProtocol {
        descriptor: descriptor.clone(),
        phase: ProtocolRegistrationPhase::Active,
        latest_evidence_ref: None,
    });
    next.counters.registrations = increment(next.counters.registrations)?;
    applied(
        next,
        event(
            TransportEventKind::ProtocolRegistered,
            operation_id,
            &descriptor.protocol_id,
            None,
            None,
            descriptor.generation,
        ),
    )
}

fn transfer_protocol(
    profile: &TransportProfile,
    state: &TransportState,
    operation_id: &str,
    descriptor: &ProtocolDescriptor,
    prior_generation: u64,
    cleanup_evidence_ref: &str,
) -> Result<TransportTransition, Vec<TransportIssue>> {
    validate_protocol_descriptor(profile, descriptor)?;
    let mut issues = Vec::new();
    if !crate::fabric::valid_blake3_ref(cleanup_evidence_ref) {
        issues.push(TransportIssue::CleanupEvidenceRequired);
    }
    let Some(current) = state.protocols.get(&descriptor.alpn) else {
        issues.push(TransportIssue::UnknownProtocol);
        return Err(issues);
    };
    if current.descriptor.generation != prior_generation {
        issues.push(TransportIssue::StaleGeneration {
            active: current.descriptor.generation,
            requested: prior_generation,
        });
    }
    if descriptor.generation <= current.descriptor.generation {
        issues.push(TransportIssue::GenerationDidNotAdvance);
    }
    if descriptor.protocol_id != current.descriptor.protocol_id || descriptor.version != current.descriptor.version {
        issues.push(TransportIssue::ConflictingProtocolIdentity);
    }
    if descriptor.extension_id != current.descriptor.extension_id {
        issues.push(TransportIssue::ExtensionIdentityMismatch);
    }
    if descriptor.service_id != current.descriptor.service_id {
        issues.push(TransportIssue::ServiceIdentityMismatch);
    }
    if descriptor.registration_authority_ref != current.descriptor.registration_authority_ref {
        issues.push(TransportIssue::RegistrationAuthorityMismatch);
    }
    if !issues.is_empty() {
        return Err(issues);
    }

    let mut next = state.clone();
    next.protocols.insert(descriptor.alpn.clone(), RegisteredProtocol {
        descriptor: descriptor.clone(),
        phase: ProtocolRegistrationPhase::Active,
        latest_evidence_ref: Some(cleanup_evidence_ref.to_string()),
    });
    next.counters.ownership_transfers = increment(next.counters.ownership_transfers)?;
    applied(
        next,
        event(
            TransportEventKind::ProtocolOwnershipTransferred,
            operation_id,
            &descriptor.protocol_id,
            None,
            None,
            descriptor.generation,
        ),
    )
}

fn begin_drain(
    state: &TransportState,
    operation_id: &str,
    alpn: &str,
    service_id: &str,
    generation: u64,
) -> Result<TransportTransition, Vec<TransportIssue>> {
    let registered = active_registration(state, alpn, service_id, generation)?;
    let protocol_id = registered.descriptor.protocol_id.clone();
    let mut next = state.clone();
    let next_registration = next.protocols.get_mut(alpn).ok_or_else(|| vec![TransportIssue::UnknownProtocol])?;
    next_registration.phase = ProtocolRegistrationPhase::Draining;
    for session in next
        .sessions
        .values_mut()
        .filter(|session| session.alpn == alpn && session.id.generation == generation && !session.phase.is_terminal())
    {
        session.phase = SessionPhase::Draining;
    }
    applied(
        next,
        event(TransportEventKind::ListenerDraining, operation_id, &protocol_id, None, None, generation),
    )
}

fn cleanup_listener(
    state: &TransportState,
    operation_id: &str,
    alpn: &str,
    service_id: &str,
    generation: u64,
    cleanup_evidence_ref: &str,
) -> Result<TransportTransition, Vec<TransportIssue>> {
    let mut issues = Vec::new();
    if !crate::fabric::valid_blake3_ref(cleanup_evidence_ref) {
        issues.push(TransportIssue::CleanupEvidenceRequired);
    }
    let Some(registered) = state.protocols.get(alpn) else {
        issues.push(TransportIssue::UnknownProtocol);
        return Err(issues);
    };
    validate_registration_scope(registered, service_id, generation, &mut issues);
    if registered.phase != ProtocolRegistrationPhase::Draining {
        issues.push(TransportIssue::ProtocolNotDraining);
    }
    if state
        .sessions
        .values()
        .any(|session| session.alpn == alpn && session.id.generation == generation && !session.phase.is_terminal())
    {
        issues.push(TransportIssue::ActiveSessionsRemain);
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    let protocol_id = registered.descriptor.protocol_id.clone();
    let mut next = state.clone();
    next.protocols.remove(alpn);
    applied(next, event(TransportEventKind::ListenerCleaned, operation_id, &protocol_id, None, None, generation))
}

#[allow(clippy::too_many_arguments)]
fn open_session(
    profile: &TransportProfile,
    state: &TransportState,
    operation_id: &str,
    session_id: &ScopedTransportId,
    alpn: &str,
    direction: SessionDirection,
    peer: &PeerIdentityRefs,
    observed_tick: u64,
    deadline_tick: u64,
) -> Result<TransportTransition, Vec<TransportIssue>> {
    let mut issues = Vec::new();
    validate_scoped_id(session_id, &mut issues);
    validate_peer(peer, &mut issues);
    let Some(registered) = state.protocols.get(alpn) else {
        issues.push(TransportIssue::UnknownProtocol);
        return Err(issues);
    };
    validate_registration_scope(registered, &session_id.service_id, session_id.generation, &mut issues);
    if registered.phase != ProtocolRegistrationPhase::Active {
        issues.push(TransportIssue::ProtocolDraining);
    }
    if state.sessions.contains_key(&session_id.opaque_ref) {
        issues.push(TransportIssue::DuplicateHandle);
    }
    let active_sessions = count_active_sessions(state)?;
    if active_sessions >= profile.limits.max_sessions {
        issues.push(TransportIssue::SessionLimitExceeded);
    }
    validate_deadline(profile, observed_tick, deadline_tick, &mut issues);
    if !issues.is_empty() {
        return Err(issues);
    }
    ensure_event_capacity(profile, 0, 0)?;

    let protocol_id = registered.descriptor.protocol_id.clone();
    let session = TransportSession {
        id: session_id.clone(),
        protocol_id: protocol_id.clone(),
        alpn: alpn.to_string(),
        direction,
        phase: SessionPhase::Active,
        peer: peer.clone(),
        streams: BTreeMap::new(),
        queued_events: 0,
        queued_bytes: 0,
        inflight_bytes: 0,
        deadline_tick,
    };
    let mut next = state.clone();
    next.sessions.insert(session_id.opaque_ref.clone(), session);
    next.counters.sessions_opened = increment(next.counters.sessions_opened)?;
    let mut opened = event(
        TransportEventKind::SessionEstablished,
        operation_id,
        &protocol_id,
        Some(&session_id.opaque_ref),
        None,
        session_id.generation,
    );
    opened.peer = Some(peer.clone());
    applied(next, opened)
}

#[allow(clippy::too_many_arguments)]
fn open_stream(
    profile: &TransportProfile,
    state: &TransportState,
    operation_id: &str,
    session_id: &ScopedTransportId,
    stream_id: &ScopedTransportId,
    direction: StreamDirection,
    initial_credit_bytes: u64,
) -> Result<TransportTransition, Vec<TransportIssue>> {
    let session = scoped_session(state, session_id)?;
    let mut issues = Vec::new();
    validate_scoped_id(stream_id, &mut issues);
    validate_stream_scope(session, stream_id, &mut issues);
    if session.phase != SessionPhase::Active {
        issues.push(TransportIssue::SessionTerminal(session.phase));
    }
    if session.streams.contains_key(&stream_id.opaque_ref) {
        issues.push(TransportIssue::DuplicateHandle);
    }
    let stream_count = count_open_streams(session)?;
    if stream_count >= profile.limits.max_streams_per_session {
        issues.push(TransportIssue::StreamLimitExceeded);
    }
    if initial_credit_bytes > profile.limits.max_inflight_bytes {
        issues.push(TransportIssue::CreditOverflow);
    }
    let registration = state.protocols.get(&session.alpn).ok_or_else(|| vec![TransportIssue::UnknownProtocol])?;
    let required = match direction {
        StreamDirection::Bidirectional => TransportCapability::BidirectionalStreams,
        StreamDirection::SendOnly | StreamDirection::ReceiveOnly => TransportCapability::UnidirectionalStreams,
    };
    if !registration.descriptor.requested_capabilities.contains(&required) {
        issues.push(TransportIssue::UnsupportedCapability(required));
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    ensure_event_capacity(profile, session.queued_events, session.queued_bytes)?;

    let protocol_id = session.protocol_id.clone();
    let mut next = state.clone();
    let next_session =
        next.sessions.get_mut(&session_id.opaque_ref).ok_or_else(|| vec![TransportIssue::UnknownSession])?;
    next_session.streams.insert(stream_id.opaque_ref.clone(), TransportStream {
        id: stream_id.clone(),
        direction,
        phase: StreamPhase::Open,
        send_credit_bytes: initial_credit_bytes,
        inflight_bytes: 0,
        next_send_sequence: FIRST_SEQUENCE,
        next_receive_sequence: FIRST_SEQUENCE,
    });
    next.counters.streams_opened = increment(next.counters.streams_opened)?;
    applied(
        next,
        event(
            TransportEventKind::StreamOpened,
            operation_id,
            &protocol_id,
            Some(&session_id.opaque_ref),
            Some(&stream_id.opaque_ref),
            session_id.generation,
        ),
    )
}

#[allow(clippy::too_many_arguments)]
fn send_frame(
    profile: &TransportProfile,
    state: &TransportState,
    operation_id: &str,
    session_id: &ScopedTransportId,
    stream_id: &ScopedTransportId,
    payload_ref: &str,
    payload_bytes: u64,
    observed_tick: u64,
) -> Result<TransportTransition, Vec<TransportIssue>> {
    let session = scoped_session(state, session_id)?;
    let stream = scoped_stream(session, stream_id)?;
    let mut issues = Vec::new();
    validate_payload(profile, state, session, payload_ref, payload_bytes, false, &mut issues);
    validate_session_active(session, &mut issues);
    validate_send_open(stream, &mut issues);
    validate_observed_deadline(session, observed_tick, &mut issues);
    if !issues.is_empty() {
        return Err(issues);
    }
    ensure_event_capacity(profile, session.queued_events, session.queued_bytes)?;

    let next_session_inflight = checked_add(session.inflight_bytes, payload_bytes)?;
    let next_stream_inflight = checked_add(stream.inflight_bytes, payload_bytes)?;
    let lacks_credit = payload_bytes > stream.send_credit_bytes;
    let exceeds_inflight = next_session_inflight > profile.limits.max_inflight_bytes
        || next_stream_inflight > profile.limits.max_inflight_bytes;
    if lacks_credit || exceeds_inflight {
        let mut blocked = event(
            TransportEventKind::Backpressured,
            operation_id,
            &session.protocol_id,
            Some(&session_id.opaque_ref),
            Some(&stream_id.opaque_ref),
            session_id.generation,
        );
        blocked.payload_ref = Some(payload_ref.to_string());
        blocked.payload_bytes = payload_bytes;
        blocked.delivery = DeliveryOutcome::NotAttempted;
        blocked.retry = RetryDisposition::HigherLevelPolicyRequired;
        return backpressured(state.clone(), blocked);
    }

    let sequence = stream.next_send_sequence;
    let next_sequence = increment(sequence)?;
    let mut next = state.clone();
    let next_session =
        next.sessions.get_mut(&session_id.opaque_ref).ok_or_else(|| vec![TransportIssue::UnknownSession])?;
    let next_stream = next_session
        .streams
        .get_mut(&stream_id.opaque_ref)
        .ok_or_else(|| vec![TransportIssue::UnknownStream])?;
    next_stream.send_credit_bytes -= payload_bytes;
    next_stream.inflight_bytes = next_stream_inflight;
    next_stream.next_send_sequence = next_sequence;
    next_session.inflight_bytes = next_session_inflight;
    next.counters.frames_submitted = increment(next.counters.frames_submitted)?;
    let mut submitted = event(
        TransportEventKind::FrameSubmitted,
        operation_id,
        &session.protocol_id,
        Some(&session_id.opaque_ref),
        Some(&stream_id.opaque_ref),
        session_id.generation,
    );
    submitted.sequence = Some(sequence);
    submitted.payload_ref = Some(payload_ref.to_string());
    submitted.payload_bytes = payload_bytes;
    submitted.delivery = DeliveryOutcome::Pending;
    submitted.retry = RetryDisposition::UnsafeWithoutReconciliation;
    applied(next, submitted)
}

#[allow(clippy::too_many_arguments)]
fn receive_frame(
    profile: &TransportProfile,
    state: &TransportState,
    operation_id: &str,
    session_id: &ScopedTransportId,
    stream_id: &ScopedTransportId,
    payload_ref: &str,
    payload_bytes: u64,
    sequence: u64,
    observed_tick: u64,
) -> Result<TransportTransition, Vec<TransportIssue>> {
    let session = scoped_session(state, session_id)?;
    let stream = scoped_stream(session, stream_id)?;
    let mut issues = Vec::new();
    validate_payload(profile, state, session, payload_ref, payload_bytes, false, &mut issues);
    validate_session_active(session, &mut issues);
    validate_receive_open(stream, &mut issues);
    validate_observed_deadline(session, observed_tick, &mut issues);
    if sequence != stream.next_receive_sequence {
        issues.push(TransportIssue::SequenceMismatch {
            expected: stream.next_receive_sequence,
            actual: sequence,
        });
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    ensure_event_capacity(profile, session.queued_events, session.queued_bytes)?;

    let next_sequence = increment(sequence)?;
    let mut next = state.clone();
    let next_session =
        next.sessions.get_mut(&session_id.opaque_ref).ok_or_else(|| vec![TransportIssue::UnknownSession])?;
    let next_stream = next_session
        .streams
        .get_mut(&stream_id.opaque_ref)
        .ok_or_else(|| vec![TransportIssue::UnknownStream])?;
    next_stream.next_receive_sequence = next_sequence;
    next.counters.frames_received = increment(next.counters.frames_received)?;
    let mut received = event(
        TransportEventKind::FrameReceived,
        operation_id,
        &session.protocol_id,
        Some(&session_id.opaque_ref),
        Some(&stream_id.opaque_ref),
        session_id.generation,
    );
    received.sequence = Some(sequence);
    received.payload_ref = Some(payload_ref.to_string());
    received.payload_bytes = payload_bytes;
    received.delivery = DeliveryOutcome::Delivered;
    applied(next, received)
}

fn acknowledge_frame(
    profile: &TransportProfile,
    state: &TransportState,
    operation_id: &str,
    session_id: &ScopedTransportId,
    stream_id: &ScopedTransportId,
    payload_bytes: u64,
) -> Result<TransportTransition, Vec<TransportIssue>> {
    let session = scoped_session(state, session_id)?;
    let stream = scoped_stream(session, stream_id)?;
    let mut issues = Vec::new();
    if payload_bytes == 0 || payload_bytes > stream.inflight_bytes || payload_bytes > session.inflight_bytes {
        issues.push(TransportIssue::InvalidAcknowledgement);
    }
    let next_credit = stream
        .send_credit_bytes
        .checked_add(payload_bytes)
        .ok_or_else(|| vec![TransportIssue::CreditOverflow])?;
    if next_credit > profile.limits.max_inflight_bytes {
        issues.push(TransportIssue::CreditOverflow);
    }
    if !issues.is_empty() {
        return Err(issues);
    }

    let mut next = state.clone();
    let next_session =
        next.sessions.get_mut(&session_id.opaque_ref).ok_or_else(|| vec![TransportIssue::UnknownSession])?;
    let next_stream = next_session
        .streams
        .get_mut(&stream_id.opaque_ref)
        .ok_or_else(|| vec![TransportIssue::UnknownStream])?;
    next_stream.inflight_bytes -= payload_bytes;
    next_stream.send_credit_bytes = next_credit;
    next_session.inflight_bytes -= payload_bytes;
    let mut acknowledged = event(
        TransportEventKind::FrameAcknowledged,
        operation_id,
        &session.protocol_id,
        Some(&session_id.opaque_ref),
        Some(&stream_id.opaque_ref),
        session_id.generation,
    );
    acknowledged.payload_bytes = payload_bytes;
    acknowledged.delivery = DeliveryOutcome::Delivered;
    applied(next, acknowledged)
}

fn send_datagram(
    profile: &TransportProfile,
    state: &TransportState,
    operation_id: &str,
    session_id: &ScopedTransportId,
    payload_ref: &str,
    payload_bytes: u64,
    observed_tick: u64,
) -> Result<TransportTransition, Vec<TransportIssue>> {
    let session = scoped_session(state, session_id)?;
    let mut issues = Vec::new();
    validate_payload(profile, state, session, payload_ref, payload_bytes, true, &mut issues);
    validate_session_active(session, &mut issues);
    validate_observed_deadline(session, observed_tick, &mut issues);
    if !profile.capabilities.contains(&TransportCapability::Datagrams) {
        issues.push(TransportIssue::UnsupportedCapability(TransportCapability::Datagrams));
    }
    let registration = state.protocols.get(&session.alpn).ok_or_else(|| vec![TransportIssue::UnknownProtocol])?;
    if !registration.descriptor.requested_capabilities.contains(&TransportCapability::Datagrams) {
        issues.push(TransportIssue::UnsupportedCapability(TransportCapability::Datagrams));
    }
    let next_inflight = checked_add(session.inflight_bytes, payload_bytes)?;
    if next_inflight > profile.limits.max_inflight_bytes {
        issues.push(TransportIssue::InflightByteLimitExceeded);
    }
    if !issues.is_empty() {
        return Err(issues);
    }

    let mut next = state.clone();
    let next_session =
        next.sessions.get_mut(&session_id.opaque_ref).ok_or_else(|| vec![TransportIssue::UnknownSession])?;
    next_session.inflight_bytes = next_inflight;
    next.counters.datagrams_submitted = increment(next.counters.datagrams_submitted)?;
    let mut submitted = event(
        TransportEventKind::DatagramSubmitted,
        operation_id,
        &session.protocol_id,
        Some(&session_id.opaque_ref),
        None,
        session_id.generation,
    );
    submitted.payload_ref = Some(payload_ref.to_string());
    submitted.payload_bytes = payload_bytes;
    submitted.delivery = DeliveryOutcome::Pending;
    submitted.retry = RetryDisposition::UnsafeWithoutReconciliation;
    applied(next, submitted)
}

fn complete_datagram(
    state: &TransportState,
    operation_id: &str,
    session_id: &ScopedTransportId,
    payload_bytes: u64,
    delivered: bool,
) -> Result<TransportTransition, Vec<TransportIssue>> {
    let session = scoped_session(state, session_id)?;
    if payload_bytes == 0 || payload_bytes > session.inflight_bytes {
        return Err(vec![TransportIssue::InvalidAcknowledgement]);
    }
    let mut next = state.clone();
    let next_session =
        next.sessions.get_mut(&session_id.opaque_ref).ok_or_else(|| vec![TransportIssue::UnknownSession])?;
    next_session.inflight_bytes -= payload_bytes;
    let mut completed = event(
        TransportEventKind::DatagramCompleted,
        operation_id,
        &session.protocol_id,
        Some(&session_id.opaque_ref),
        None,
        session_id.generation,
    );
    completed.payload_bytes = payload_bytes;
    completed.delivery = if delivered {
        DeliveryOutcome::Delivered
    } else {
        DeliveryOutcome::NotDelivered
    };
    completed.retry = if delivered {
        RetryDisposition::NotApplicable
    } else {
        RetryDisposition::HigherLevelPolicyRequired
    };
    completed.terminal = true;
    applied(next, completed)
}

fn grant_credit(
    profile: &TransportProfile,
    state: &TransportState,
    operation_id: &str,
    session_id: &ScopedTransportId,
    stream_id: &ScopedTransportId,
    credit_bytes: u64,
) -> Result<TransportTransition, Vec<TransportIssue>> {
    let session = scoped_session(state, session_id)?;
    let stream = scoped_stream(session, stream_id)?;
    let next_credit = stream
        .send_credit_bytes
        .checked_add(credit_bytes)
        .ok_or_else(|| vec![TransportIssue::CreditOverflow])?;
    if next_credit > profile.limits.max_inflight_bytes {
        return Err(vec![TransportIssue::CreditOverflow]);
    }
    let mut next = state.clone();
    let next_session =
        next.sessions.get_mut(&session_id.opaque_ref).ok_or_else(|| vec![TransportIssue::UnknownSession])?;
    let next_stream = next_session
        .streams
        .get_mut(&stream_id.opaque_ref)
        .ok_or_else(|| vec![TransportIssue::UnknownStream])?;
    next_stream.send_credit_bytes = next_credit;
    let mut granted = event(
        TransportEventKind::CreditGranted,
        operation_id,
        &session.protocol_id,
        Some(&session_id.opaque_ref),
        Some(&stream_id.opaque_ref),
        session_id.generation,
    );
    granted.payload_bytes = credit_bytes;
    applied(next, granted)
}

fn half_close_stream(
    state: &TransportState,
    operation_id: &str,
    session_id: &ScopedTransportId,
    stream_id: &ScopedTransportId,
    send_direction: bool,
) -> Result<TransportTransition, Vec<TransportIssue>> {
    let session = scoped_session(state, session_id)?;
    let stream = scoped_stream(session, stream_id)?;
    if stream.phase.is_terminal() {
        return Err(vec![TransportIssue::StreamTerminal(stream.phase)]);
    }
    let next_phase = match (stream.phase, send_direction) {
        (StreamPhase::Open, true) => StreamPhase::SendHalfClosed,
        (StreamPhase::Open, false) => StreamPhase::ReceiveHalfClosed,
        (StreamPhase::ReceiveHalfClosed, true) | (StreamPhase::SendHalfClosed, false) => StreamPhase::Closed,
        (StreamPhase::SendHalfClosed, true) => return Err(vec![TransportIssue::SendDirectionClosed]),
        (StreamPhase::ReceiveHalfClosed, false) => return Err(vec![TransportIssue::ReceiveDirectionClosed]),
        (StreamPhase::Closed | StreamPhase::Reset | StreamPhase::Cancelled, _) => {
            return Err(vec![TransportIssue::StreamTerminal(stream.phase)]);
        }
    };
    let terminal = next_phase.is_terminal();
    let mut next = state.clone();
    let next_session =
        next.sessions.get_mut(&session_id.opaque_ref).ok_or_else(|| vec![TransportIssue::UnknownSession])?;
    let next_stream = next_session
        .streams
        .get_mut(&stream_id.opaque_ref)
        .ok_or_else(|| vec![TransportIssue::UnknownStream])?;
    next_stream.phase = next_phase;
    let mut half_closed = event(
        if terminal {
            TransportEventKind::StreamClosed
        } else {
            TransportEventKind::StreamHalfClosed
        },
        operation_id,
        &session.protocol_id,
        Some(&session_id.opaque_ref),
        Some(&stream_id.opaque_ref),
        session_id.generation,
    );
    half_closed.terminal = terminal;
    applied(next, half_closed)
}

fn close_stream(
    state: &TransportState,
    operation_id: &str,
    session_id: &ScopedTransportId,
    stream_id: &ScopedTransportId,
) -> Result<TransportTransition, Vec<TransportIssue>> {
    let session = scoped_session(state, session_id)?;
    let stream = scoped_stream(session, stream_id)?;
    if stream.phase.is_terminal() {
        return Err(vec![TransportIssue::StreamTerminal(stream.phase)]);
    }
    let mut next = state.clone();
    let next_session =
        next.sessions.get_mut(&session_id.opaque_ref).ok_or_else(|| vec![TransportIssue::UnknownSession])?;
    let next_stream = next_session
        .streams
        .get_mut(&stream_id.opaque_ref)
        .ok_or_else(|| vec![TransportIssue::UnknownStream])?;
    next_stream.phase = StreamPhase::Closed;
    let mut closed = event(
        TransportEventKind::StreamClosed,
        operation_id,
        &session.protocol_id,
        Some(&session_id.opaque_ref),
        Some(&stream_id.opaque_ref),
        session_id.generation,
    );
    closed.terminal = true;
    applied(next, closed)
}

fn close_session(
    state: &TransportState,
    operation_id: &str,
    session_id: &ScopedTransportId,
) -> Result<TransportTransition, Vec<TransportIssue>> {
    let session = scoped_session(state, session_id)?;
    if session.phase.is_terminal() {
        return Err(vec![TransportIssue::SessionTerminal(session.phase)]);
    }
    let mut next = state.clone();
    let next_session =
        next.sessions.get_mut(&session_id.opaque_ref).ok_or_else(|| vec![TransportIssue::UnknownSession])?;
    next_session.phase = SessionPhase::Closed;
    for stream in next_session.streams.values_mut() {
        if !stream.phase.is_terminal() {
            stream.phase = StreamPhase::Closed;
        }
    }
    let mut closed = event(
        TransportEventKind::SessionClosed,
        operation_id,
        &session.protocol_id,
        Some(&session_id.opaque_ref),
        None,
        session_id.generation,
    );
    closed.terminal = true;
    applied(next, closed)
}

fn cancel(
    state: &TransportState,
    operation_id: &str,
    target: &CancelTarget,
) -> Result<TransportTransition, Vec<TransportIssue>> {
    let (session_id, stream_id) = match target {
        CancelTarget::Session(session_id) => (session_id, None),
        CancelTarget::Stream { session_id, stream_id } => (session_id, Some(stream_id)),
    };
    let session = scoped_session(state, session_id)?;
    if let Some(stream_id) = stream_id {
        let stream = scoped_stream(session, stream_id)?;
        if stream.phase.is_terminal() {
            return Err(vec![TransportIssue::StreamTerminal(stream.phase)]);
        }
    } else if session.phase.is_terminal() {
        return Err(vec![TransportIssue::SessionTerminal(session.phase)]);
    }

    let mut next = state.clone();
    let next_session =
        next.sessions.get_mut(&session_id.opaque_ref).ok_or_else(|| vec![TransportIssue::UnknownSession])?;
    if let Some(stream_id) = stream_id {
        let next_stream = next_session
            .streams
            .get_mut(&stream_id.opaque_ref)
            .ok_or_else(|| vec![TransportIssue::UnknownStream])?;
        next_stream.phase = StreamPhase::Cancelled;
    } else {
        next_session.phase = SessionPhase::Cancelled;
        for stream in next_session.streams.values_mut() {
            if !stream.phase.is_terminal() {
                stream.phase = StreamPhase::Cancelled;
            }
        }
    }
    next.counters.cancellations = increment(next.counters.cancellations)?;
    let mut cancelled = event(
        TransportEventKind::Cancelled,
        operation_id,
        &session.protocol_id,
        Some(&session_id.opaque_ref),
        stream_id.map(|id| id.opaque_ref.as_str()),
        session_id.generation,
    );
    cancelled.failure = Some(TransportFailureClass::Cancellation);
    cancelled.delivery = if session.inflight_bytes == 0 {
        DeliveryOutcome::NotAttempted
    } else {
        DeliveryOutcome::Uncertain
    };
    cancelled.retry = if cancelled.delivery == DeliveryOutcome::Uncertain {
        RetryDisposition::UnsafeWithoutReconciliation
    } else {
        RetryDisposition::HigherLevelPolicyRequired
    };
    cancelled.terminal = true;
    applied(next, cancelled)
}

fn fail_session(
    state: &TransportState,
    operation_id: &str,
    session_id: &ScopedTransportId,
    class: TransportFailureClass,
    delivery_definitive: bool,
) -> Result<TransportTransition, Vec<TransportIssue>> {
    let session = scoped_session(state, session_id)?;
    if session.phase.is_terminal() {
        return Err(vec![TransportIssue::SessionTerminal(session.phase)]);
    }
    let delivery = if delivery_definitive {
        DeliveryOutcome::NotDelivered
    } else if session.inflight_bytes > 0 {
        DeliveryOutcome::Uncertain
    } else {
        DeliveryOutcome::NotAttempted
    };
    let retry = if delivery == DeliveryOutcome::Uncertain {
        RetryDisposition::UnsafeWithoutReconciliation
    } else {
        RetryDisposition::HigherLevelPolicyRequired
    };
    let mut next = state.clone();
    let next_session =
        next.sessions.get_mut(&session_id.opaque_ref).ok_or_else(|| vec![TransportIssue::UnknownSession])?;
    next_session.phase = SessionPhase::Failed;
    for stream in next_session.streams.values_mut() {
        if !stream.phase.is_terminal() {
            stream.phase = StreamPhase::Reset;
        }
    }
    next.counters.failures = increment(next.counters.failures)?;
    let mut failed = event(
        TransportEventKind::Failed,
        operation_id,
        &session.protocol_id,
        Some(&session_id.opaque_ref),
        None,
        session_id.generation,
    );
    failed.failure = Some(class);
    failed.delivery = delivery;
    failed.retry = retry;
    failed.terminal = true;
    applied(next, failed)
}

fn active_registration<'a>(
    state: &'a TransportState,
    alpn: &str,
    service_id: &str,
    generation: u64,
) -> Result<&'a RegisteredProtocol, Vec<TransportIssue>> {
    let registered = state.protocols.get(alpn).ok_or_else(|| vec![TransportIssue::UnknownProtocol])?;
    let mut issues = Vec::new();
    validate_registration_scope(registered, service_id, generation, &mut issues);
    if registered.phase != ProtocolRegistrationPhase::Active {
        issues.push(TransportIssue::ProtocolDraining);
    }
    if issues.is_empty() { Ok(registered) } else { Err(issues) }
}

fn validate_registration_scope(
    registered: &RegisteredProtocol,
    service_id: &str,
    generation: u64,
    issues: &mut Vec<TransportIssue>,
) {
    if registered.descriptor.service_id != service_id {
        issues.push(TransportIssue::ServiceIdentityMismatch);
    }
    if registered.descriptor.generation != generation {
        issues.push(TransportIssue::StaleGeneration {
            active: registered.descriptor.generation,
            requested: generation,
        });
    }
}

fn scoped_session<'a>(
    state: &'a TransportState,
    id: &ScopedTransportId,
) -> Result<&'a TransportSession, Vec<TransportIssue>> {
    let mut issues = Vec::new();
    validate_scoped_id(id, &mut issues);
    let Some(session) = state.sessions.get(&id.opaque_ref) else {
        issues.push(TransportIssue::UnknownSession);
        return Err(issues);
    };
    if session.id.generation != id.generation {
        issues.push(TransportIssue::WrongGenerationHandle);
    }
    if session.id.service_id != id.service_id {
        issues.push(TransportIssue::HandleServiceMismatch);
    }
    if issues.is_empty() { Ok(session) } else { Err(issues) }
}

fn scoped_stream<'a>(
    session: &'a TransportSession,
    id: &ScopedTransportId,
) -> Result<&'a TransportStream, Vec<TransportIssue>> {
    let mut issues = Vec::new();
    validate_scoped_id(id, &mut issues);
    validate_stream_scope(session, id, &mut issues);
    let Some(stream) = session.streams.get(&id.opaque_ref) else {
        issues.push(TransportIssue::UnknownStream);
        return Err(issues);
    };
    if stream.id.generation != id.generation {
        issues.push(TransportIssue::WrongGenerationHandle);
    }
    if stream.id.service_id != id.service_id {
        issues.push(TransportIssue::HandleServiceMismatch);
    }
    if issues.is_empty() { Ok(stream) } else { Err(issues) }
}

fn validate_stream_scope(session: &TransportSession, id: &ScopedTransportId, issues: &mut Vec<TransportIssue>) {
    if session.id.generation != id.generation {
        issues.push(TransportIssue::WrongGenerationHandle);
    }
    if session.id.service_id != id.service_id {
        issues.push(TransportIssue::HandleServiceMismatch);
    }
}

fn validate_session_active(session: &TransportSession, issues: &mut Vec<TransportIssue>) {
    if session.phase != SessionPhase::Active {
        issues.push(TransportIssue::SessionTerminal(session.phase));
    }
}

fn validate_send_open(stream: &TransportStream, issues: &mut Vec<TransportIssue>) {
    if stream.direction == StreamDirection::ReceiveOnly {
        issues.push(TransportIssue::StreamDirectionUnsupported);
    }
    if matches!(stream.phase, StreamPhase::SendHalfClosed) {
        issues.push(TransportIssue::SendDirectionClosed);
    } else if stream.phase.is_terminal() {
        issues.push(TransportIssue::StreamTerminal(stream.phase));
    }
}

fn validate_receive_open(stream: &TransportStream, issues: &mut Vec<TransportIssue>) {
    if stream.direction == StreamDirection::SendOnly {
        issues.push(TransportIssue::StreamDirectionUnsupported);
    }
    if matches!(stream.phase, StreamPhase::ReceiveHalfClosed) {
        issues.push(TransportIssue::ReceiveDirectionClosed);
    } else if stream.phase.is_terminal() {
        issues.push(TransportIssue::StreamTerminal(stream.phase));
    }
}

fn validate_payload(
    profile: &TransportProfile,
    state: &TransportState,
    session: &TransportSession,
    payload_ref: &str,
    payload_bytes: u64,
    datagram: bool,
    issues: &mut Vec<TransportIssue>,
) {
    if !crate::fabric::valid_blake3_ref(payload_ref) {
        issues.push(TransportIssue::MalformedContentRef("payload-ref"));
    }
    if payload_bytes == 0 {
        issues.push(TransportIssue::EmptyPayload);
    }
    let registration = state.protocols.get(&session.alpn);
    let frame_maximum = registration
        .map(|registered| registered.descriptor.framing.max_frame_bytes)
        .unwrap_or(profile.limits.max_frame_bytes);
    let maximum = if datagram {
        profile.limits.max_datagram_bytes
    } else {
        profile.limits.max_frame_bytes.min(frame_maximum)
    };
    if payload_bytes > maximum {
        if datagram {
            issues.push(TransportIssue::DatagramLimitExceeded {
                actual: payload_bytes,
                maximum,
            });
        } else {
            issues.push(TransportIssue::FrameLimitExceeded {
                actual: payload_bytes,
                maximum,
            });
        }
    }
}

fn validate_deadline(
    profile: &TransportProfile,
    observed_tick: u64,
    deadline_tick: u64,
    issues: &mut Vec<TransportIssue>,
) {
    if deadline_tick <= observed_tick {
        issues.push(TransportIssue::DeadlineExpired);
        return;
    }
    let Some(window) = deadline_tick.checked_sub(observed_tick) else {
        issues.push(TransportIssue::DeadlineExpired);
        return;
    };
    if window > profile.limits.operation_deadline_ticks {
        issues.push(TransportIssue::DeadlineTooLarge);
    }
}

fn validate_observed_deadline(session: &TransportSession, observed_tick: u64, issues: &mut Vec<TransportIssue>) {
    if observed_tick >= session.deadline_tick {
        issues.push(TransportIssue::DeadlineExpired);
    }
}

fn ensure_event_capacity(
    profile: &TransportProfile,
    queued_events: u64,
    queued_bytes: u64,
) -> Result<(), Vec<TransportIssue>> {
    let mut issues = Vec::new();
    let next_events = queued_events.checked_add(EVENT_SLOT).ok_or(TransportIssue::CounterOverflow);
    match next_events {
        Ok(next_events) if next_events > profile.limits.max_queued_events => {
            issues.push(TransportIssue::QueueEventLimitExceeded);
        }
        Err(issue) => issues.push(issue),
        Ok(_) => {}
    }
    if queued_bytes > profile.limits.max_queued_bytes {
        issues.push(TransportIssue::QueueByteLimitExceeded);
    }
    if issues.is_empty() { Ok(()) } else { Err(issues) }
}

fn count_active_protocols(state: &TransportState) -> Result<u64, Vec<TransportIssue>> {
    count(
        state
            .protocols
            .values()
            .filter(|registered| registered.phase == ProtocolRegistrationPhase::Active)
            .count(),
    )
}

fn count_active_sessions(state: &TransportState) -> Result<u64, Vec<TransportIssue>> {
    count(state.sessions.values().filter(|session| !session.phase.is_terminal()).count())
}

fn count_open_streams(session: &TransportSession) -> Result<u64, Vec<TransportIssue>> {
    count(session.streams.values().filter(|stream| !stream.phase.is_terminal()).count())
}

fn count(value: usize) -> Result<u64, Vec<TransportIssue>> {
    u64::try_from(value).map_err(|_| vec![TransportIssue::CounterOverflow])
}

fn increment(value: u64) -> Result<u64, Vec<TransportIssue>> {
    value.checked_add(1).ok_or_else(|| vec![TransportIssue::CounterOverflow])
}

fn checked_add(left: u64, right: u64) -> Result<u64, Vec<TransportIssue>> {
    left.checked_add(right).ok_or_else(|| vec![TransportIssue::CounterOverflow])
}

fn event(
    kind: TransportEventKind,
    operation_id: &str,
    protocol_id: &str,
    session_id: Option<&str>,
    stream_id: Option<&str>,
    generation: u64,
) -> TransportEvent {
    TransportEvent {
        kind,
        operation_id: operation_id.to_string(),
        protocol_id: protocol_id.to_string(),
        session_id: session_id.map(str::to_string),
        stream_id: stream_id.map(str::to_string),
        generation,
        sequence: None,
        payload_ref: None,
        payload_bytes: 0,
        peer: None,
        failure: None,
        delivery: DeliveryOutcome::NotAttempted,
        retry: RetryDisposition::NotApplicable,
        terminal: false,
    }
}

fn applied(next: TransportState, event: TransportEvent) -> Result<TransportTransition, Vec<TransportIssue>> {
    Ok(TransportTransition {
        next,
        decision: TransportTransitionDecision::Applied,
        events: vec![event],
        automatic_retry_count: NO_AUTOMATIC_RETRIES,
    })
}

fn backpressured(next: TransportState, event: TransportEvent) -> Result<TransportTransition, Vec<TransportIssue>> {
    Ok(TransportTransition {
        next,
        decision: TransportTransitionDecision::Backpressured,
        events: vec![event],
        automatic_retry_count: NO_AUTOMATIC_RETRIES,
    })
}
