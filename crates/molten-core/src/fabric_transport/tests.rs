use super::*;

const PROFILE_REF: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const FRAMING_REF: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const AUTHORITY_REF: &str = "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const OPERATION_REF: &str = "blake3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const SESSION_REF: &str = "blake3:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const STREAM_REF: &str = "blake3:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
const PAYLOAD_REF: &str = "blake3:1111111111111111111111111111111111111111111111111111111111111111";
const PEER_REF: &str = "blake3:2222222222222222222222222222222222222222222222222222222222222222";
const MEMBERSHIP_REF: &str = "blake3:3333333333333333333333333333333333333333333333333333333333333333";
const PRINCIPAL_REF: &str = "blake3:4444444444444444444444444444444444444444444444444444444444444444";
const TRUST_REF: &str = "blake3:5555555555555555555555555555555555555555555555555555555555555555";
const CAPABILITY_REF: &str = "blake3:6666666666666666666666666666666666666666666666666666666666666666";
const CLEANUP_REF: &str = "blake3:7777777777777777777777777777777777777777777777777777777777777777";
const ACTUAL_REF: &str = "blake3:8888888888888888888888888888888888888888888888888888888888888888";
const GENERATION_ONE: u64 = 1;
const GENERATION_TWO: u64 = 2;
const LISTENER_LIMIT: u64 = 4;
const SESSION_LIMIT: u64 = 8;
const STREAM_LIMIT: u64 = 8;
const FRAME_LIMIT: u64 = 1_024;
const DATAGRAM_LIMIT: u64 = 512;
const QUEUE_EVENT_LIMIT: u64 = 16;
const QUEUE_BYTE_LIMIT: u64 = 4_096;
const INFLIGHT_LIMIT: u64 = 2_048;
const DEADLINE_WINDOW: u64 = 32;
const INITIAL_TICK: u64 = 10;
const DEADLINE_TICK: u64 = INITIAL_TICK + DEADLINE_WINDOW;
const INITIAL_CREDIT: u64 = 16;
const FRAME_BYTES: u64 = 8;
const LENGTH_PREFIX_BYTES: u64 = 4;
const OVERSIZED_FRAME_BYTES: u64 = FRAME_LIMIT + 1;

fn profile(adapter_kind: TransportAdapterKind, datagrams: bool) -> TransportProfile {
    let mut capabilities = vec![
        TransportCapability::BidirectionalStreams,
        TransportCapability::UnidirectionalStreams,
    ];
    if datagrams {
        capabilities.push(TransportCapability::Datagrams);
    }
    TransportProfile {
        schema: TRANSPORT_PROFILE_SCHEMA.to_string(),
        profile_id: match adapter_kind {
            TransportAdapterKind::IrohLive => "iroh-live-v1",
            TransportAdapterKind::DeterministicSimulation => "simulated-transport-v1",
        }
        .to_string(),
        profile_ref: PROFILE_REF.to_string(),
        adapter_kind,
        capabilities,
        limits: TransportLimits {
            max_listeners: LISTENER_LIMIT,
            max_sessions: SESSION_LIMIT,
            max_streams_per_session: STREAM_LIMIT,
            max_frame_bytes: FRAME_LIMIT,
            max_datagram_bytes: DATAGRAM_LIMIT,
            max_queued_events: QUEUE_EVENT_LIMIT,
            max_queued_bytes: QUEUE_BYTE_LIMIT,
            max_inflight_bytes: INFLIGHT_LIMIT,
            operation_deadline_ticks: DEADLINE_WINDOW,
        },
        non_claims: REQUIRED_TRANSPORT_NON_CLAIMS.to_vec(),
    }
}

fn descriptor(generation: u64, datagrams: bool) -> ProtocolDescriptor {
    let mut requested_capabilities = vec![
        TransportCapability::BidirectionalStreams,
        TransportCapability::UnidirectionalStreams,
    ];
    if datagrams {
        requested_capabilities.push(TransportCapability::Datagrams);
    }
    ProtocolDescriptor {
        schema: TRANSPORT_PROTOCOL_SCHEMA.to_string(),
        protocol_id: "echo-protocol".to_string(),
        version: "v1".to_string(),
        alpn: "molten/echo/1".to_string(),
        extension_id: "echo-extension".to_string(),
        service_id: "echo-service".to_string(),
        generation,
        listener_limit: 1,
        requested_capabilities,
        framing: FramingProfile {
            profile_id: "length-delimited-blake3-v1".to_string(),
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

fn scoped_id(reference: &str, generation: u64) -> ScopedTransportId {
    ScopedTransportId {
        opaque_ref: reference.to_string(),
        service_id: "echo-service".to_string(),
        generation,
    }
}

fn admitted_peer() -> PeerIdentityRefs {
    PeerIdentityRefs {
        transport_identity_ref: PEER_REF.to_string(),
        membership_ref: Some(MEMBERSHIP_REF.to_string()),
        application_principal_ref: Some(PRINCIPAL_REF.to_string()),
        trust_decision_ref: Some(TRUST_REF.to_string()),
        capability_authority_ref: Some(CAPABILITY_REF.to_string()),
        bootstrap_policy_ref: None,
    }
}

fn registered_state(profile: &TransportProfile, descriptor: &ProtocolDescriptor) -> TransportState {
    apply_transport_command(profile, &TransportState::default(), &TransportCommand::Register {
        operation_id: OPERATION_REF.to_string(),
        descriptor: descriptor.clone(),
    })
    .expect("register protocol")
    .next
}

fn session_state(profile: &TransportProfile, descriptor: &ProtocolDescriptor) -> TransportState {
    let registered = registered_state(profile, descriptor);
    apply_transport_command(profile, &registered, &TransportCommand::OpenSession {
        operation_id: OPERATION_REF.to_string(),
        session_id: scoped_id(SESSION_REF, descriptor.generation),
        alpn: descriptor.alpn.clone(),
        direction: SessionDirection::Outbound,
        peer: admitted_peer(),
        observed_tick: INITIAL_TICK,
        deadline_tick: DEADLINE_TICK,
    })
    .expect("open session")
    .next
}

fn stream_state(profile: &TransportProfile, descriptor: &ProtocolDescriptor, credit: u64) -> TransportState {
    let session = session_state(profile, descriptor);
    apply_transport_command(profile, &session, &TransportCommand::OpenStream {
        operation_id: OPERATION_REF.to_string(),
        session_id: scoped_id(SESSION_REF, descriptor.generation),
        stream_id: scoped_id(STREAM_REF, descriptor.generation),
        direction: StreamDirection::Bidirectional,
        initial_credit_bytes: credit,
    })
    .expect("open stream")
    .next
}

// r[verify molten.fabric_transport.port_contract]
// r[verify molten.fabric_transport.protocol_registration]
#[test]
fn unique_registration_and_atomic_generation_transfer_route_only_to_replacement() {
    let profile = profile(TransportAdapterKind::DeterministicSimulation, false);
    let first = descriptor(GENERATION_ONE, false);
    let registered = registered_state(&profile, &first);
    assert_eq!(registered.protocols[&first.alpn].descriptor.generation, GENERATION_ONE);

    let duplicate = apply_transport_command(&profile, &registered, &TransportCommand::Register {
        operation_id: OPERATION_REF.to_string(),
        descriptor: first.clone(),
    })
    .expect_err("duplicate registration must deny");
    assert!(duplicate.contains(&TransportIssue::DuplicateProtocol));

    let replacement = descriptor(GENERATION_TWO, false);
    let transferred = apply_transport_command(&profile, &registered, &TransportCommand::TransferOwnership {
        operation_id: OPERATION_REF.to_string(),
        descriptor: replacement.clone(),
        prior_generation: GENERATION_ONE,
        cleanup_evidence_ref: CLEANUP_REF.to_string(),
    })
    .expect("transfer ownership");
    assert_eq!(transferred.next.protocols[&replacement.alpn].descriptor.generation, GENERATION_TWO);
    assert_eq!(transferred.events[0].kind, TransportEventKind::ProtocolOwnershipTransferred);

    let stale_session = apply_transport_command(&profile, &transferred.next, &TransportCommand::OpenSession {
        operation_id: OPERATION_REF.to_string(),
        session_id: scoped_id(SESSION_REF, GENERATION_ONE),
        alpn: replacement.alpn,
        direction: SessionDirection::Inbound,
        peer: admitted_peer(),
        observed_tick: INITIAL_TICK,
        deadline_tick: DEADLINE_TICK,
    })
    .expect_err("stale generation must not accept");
    assert!(stale_session.iter().any(|issue| matches!(issue, TransportIssue::StaleGeneration { .. })));
}

// r[verify molten.fabric_transport.identity_separation]
#[test]
fn authenticated_transport_identity_without_service_authority_is_denied() {
    let profile = profile(TransportAdapterKind::DeterministicSimulation, false);
    let descriptor = descriptor(GENERATION_ONE, false);
    let registered = registered_state(&profile, &descriptor);
    let transport_only = PeerIdentityRefs {
        transport_identity_ref: PEER_REF.to_string(),
        membership_ref: None,
        application_principal_ref: None,
        trust_decision_ref: None,
        capability_authority_ref: None,
        bootstrap_policy_ref: None,
    };
    let issues = apply_transport_command(&profile, &registered, &TransportCommand::OpenSession {
        operation_id: OPERATION_REF.to_string(),
        session_id: scoped_id(SESSION_REF, GENERATION_ONE),
        alpn: descriptor.alpn,
        direction: SessionDirection::Inbound,
        peer: transport_only,
        observed_tick: INITIAL_TICK,
        deadline_tick: DEADLINE_TICK,
    })
    .expect_err("transport identity alone must deny");
    assert!(issues.contains(&TransportIssue::TransportIdentityWithoutServiceAuthority));
}

// r[verify molten.fabric_transport.session_streams]
// r[verify molten.fabric_transport.flow_control]
#[test]
fn bounded_stream_reports_backpressure_then_progresses_through_explicit_credit_and_ack() {
    let profile = profile(TransportAdapterKind::DeterministicSimulation, false);
    let descriptor = descriptor(GENERATION_ONE, false);
    let state = stream_state(&profile, &descriptor, 0);
    let send = TransportCommand::SendFrame {
        operation_id: OPERATION_REF.to_string(),
        session_id: scoped_id(SESSION_REF, GENERATION_ONE),
        stream_id: scoped_id(STREAM_REF, GENERATION_ONE),
        payload_ref: PAYLOAD_REF.to_string(),
        payload_bytes: FRAME_BYTES,
        observed_tick: INITIAL_TICK,
    };
    let blocked = apply_transport_command(&profile, &state, &send).expect("backpressure is an explicit event");
    assert_eq!(blocked.decision, TransportTransitionDecision::Backpressured);
    assert_eq!(blocked.events[0].kind, TransportEventKind::Backpressured);
    assert_eq!(blocked.next, state);

    let credited = apply_transport_command(&profile, &state, &TransportCommand::GrantCredit {
        operation_id: OPERATION_REF.to_string(),
        session_id: scoped_id(SESSION_REF, GENERATION_ONE),
        stream_id: scoped_id(STREAM_REF, GENERATION_ONE),
        credit_bytes: INITIAL_CREDIT,
    })
    .expect("grant credit");
    let submitted = apply_transport_command(&profile, &credited.next, &send).expect("submit frame");
    assert_eq!(submitted.events[0].delivery, DeliveryOutcome::Pending);
    assert_eq!(submitted.automatic_retry_count, 0);

    let acknowledged = apply_transport_command(&profile, &submitted.next, &TransportCommand::AcknowledgeFrame {
        operation_id: OPERATION_REF.to_string(),
        session_id: scoped_id(SESSION_REF, GENERATION_ONE),
        stream_id: scoped_id(STREAM_REF, GENERATION_ONE),
        payload_bytes: FRAME_BYTES,
    })
    .expect("acknowledge frame");
    assert_eq!(acknowledged.events[0].delivery, DeliveryOutcome::Delivered);
    assert_eq!(acknowledged.next.sessions[SESSION_REF].inflight_bytes, 0);
}

// r[verify molten.fabric_transport.failure_semantics]
#[test]
fn disconnect_after_submission_is_uncertain_and_never_retried_implicitly() {
    let profile = profile(TransportAdapterKind::DeterministicSimulation, false);
    let descriptor = descriptor(GENERATION_ONE, false);
    let state = stream_state(&profile, &descriptor, INITIAL_CREDIT);
    let submitted = apply_transport_command(&profile, &state, &TransportCommand::SendFrame {
        operation_id: OPERATION_REF.to_string(),
        session_id: scoped_id(SESSION_REF, GENERATION_ONE),
        stream_id: scoped_id(STREAM_REF, GENERATION_ONE),
        payload_ref: PAYLOAD_REF.to_string(),
        payload_bytes: FRAME_BYTES,
        observed_tick: INITIAL_TICK,
    })
    .expect("submit frame");
    let failed = apply_transport_command(&profile, &submitted.next, &TransportCommand::FailSession {
        operation_id: OPERATION_REF.to_string(),
        session_id: scoped_id(SESSION_REF, GENERATION_ONE),
        class: TransportFailureClass::Disconnect,
        delivery_definitive: false,
    })
    .expect("classify disconnect");
    assert_eq!(failed.events[0].delivery, DeliveryOutcome::Uncertain);
    assert_eq!(failed.events[0].retry, RetryDisposition::UnsafeWithoutReconciliation);
    assert_eq!(failed.automatic_retry_count, 0);
}

// r[verify molten.fabric_transport.session_streams]
// r[verify molten.fabric_transport.failure_semantics]
#[test]
fn optional_datagram_has_an_explicit_terminal_outcome_and_releases_inflight_bytes() {
    let profile = profile(TransportAdapterKind::DeterministicSimulation, true);
    let descriptor = descriptor(GENERATION_ONE, true);
    let state = session_state(&profile, &descriptor);
    let submitted = apply_transport_command(&profile, &state, &TransportCommand::SendDatagram {
        operation_id: OPERATION_REF.to_string(),
        session_id: scoped_id(SESSION_REF, GENERATION_ONE),
        payload_ref: PAYLOAD_REF.to_string(),
        payload_bytes: FRAME_BYTES,
        observed_tick: INITIAL_TICK,
    })
    .expect("submit datagram");
    assert_eq!(submitted.events[0].kind, TransportEventKind::DatagramSubmitted);
    assert_eq!(submitted.events[0].delivery, DeliveryOutcome::Pending);

    let invalid = apply_transport_command(&profile, &submitted.next, &TransportCommand::CompleteDatagram {
        operation_id: OPERATION_REF.to_string(),
        session_id: scoped_id(SESSION_REF, GENERATION_ONE),
        payload_bytes: FRAME_LIMIT,
        delivered: false,
    })
    .expect_err("invalid datagram completion must deny");
    assert!(invalid.contains(&TransportIssue::InvalidAcknowledgement));

    let completed = apply_transport_command(&profile, &submitted.next, &TransportCommand::CompleteDatagram {
        operation_id: OPERATION_REF.to_string(),
        session_id: scoped_id(SESSION_REF, GENERATION_ONE),
        payload_bytes: FRAME_BYTES,
        delivered: true,
    })
    .expect("complete datagram");
    assert_eq!(completed.events[0].kind, TransportEventKind::DatagramCompleted);
    assert_eq!(completed.events[0].delivery, DeliveryOutcome::Delivered);
    assert!(completed.events[0].terminal);
    assert_eq!(completed.next.sessions[SESSION_REF].inflight_bytes, 0);
}

// r[verify molten.fabric_transport.port_contract]
// r[verify molten.fabric_transport.flow_control]
#[test]
fn malformed_oversized_unknown_and_unsupported_inputs_fail_before_delivery() {
    let profile = profile(TransportAdapterKind::DeterministicSimulation, false);
    let mismatch =
        validate_outer_frame(&profile, PAYLOAD_REF, ACTUAL_REF, FRAME_BYTES).expect_err("payload mismatch must deny");
    assert!(mismatch.contains(&TransportIssue::PayloadRefMismatch));

    let oversized = validate_outer_frame(&profile, PAYLOAD_REF, PAYLOAD_REF, OVERSIZED_FRAME_BYTES)
        .expect_err("oversized frame must deny");
    assert!(oversized.iter().any(|issue| matches!(issue, TransportIssue::FrameLimitExceeded { .. })));

    let descriptor = descriptor(GENERATION_ONE, false);
    let state = session_state(&profile, &descriptor);
    let unknown_stream = apply_transport_command(&profile, &state, &TransportCommand::ReceiveFrame {
        operation_id: OPERATION_REF.to_string(),
        session_id: scoped_id(SESSION_REF, GENERATION_ONE),
        stream_id: scoped_id(STREAM_REF, GENERATION_ONE),
        payload_ref: PAYLOAD_REF.to_string(),
        payload_bytes: FRAME_BYTES,
        sequence: 0,
        observed_tick: INITIAL_TICK,
    })
    .expect_err("unknown stream event must deny");
    assert!(unknown_stream.contains(&TransportIssue::UnknownStream));

    let datagram = apply_transport_command(&profile, &state, &TransportCommand::SendDatagram {
        operation_id: OPERATION_REF.to_string(),
        session_id: scoped_id(SESSION_REF, GENERATION_ONE),
        payload_ref: PAYLOAD_REF.to_string(),
        payload_bytes: FRAME_BYTES,
        observed_tick: INITIAL_TICK,
    })
    .expect_err("unsupported datagram must deny");
    assert!(datagram.contains(&TransportIssue::UnsupportedCapability(TransportCapability::Datagrams)));
}

// r[verify molten.fabric_transport.protocol_registration]
// r[verify molten.fabric_transport.final_validation]
#[test]
fn drain_stops_new_sessions_and_cleanup_waits_for_terminal_sessions() {
    let profile = profile(TransportAdapterKind::DeterministicSimulation, false);
    let descriptor = descriptor(GENERATION_ONE, false);
    let state = session_state(&profile, &descriptor);
    let draining = apply_transport_command(&profile, &state, &TransportCommand::BeginDrain {
        operation_id: OPERATION_REF.to_string(),
        alpn: descriptor.alpn.clone(),
        service_id: descriptor.service_id.clone(),
        generation: GENERATION_ONE,
    })
    .expect("begin drain");

    let denied = apply_transport_command(&profile, &draining.next, &TransportCommand::OpenSession {
        operation_id: OPERATION_REF.to_string(),
        session_id: scoped_id(STREAM_REF, GENERATION_ONE),
        alpn: descriptor.alpn.clone(),
        direction: SessionDirection::Inbound,
        peer: admitted_peer(),
        observed_tick: INITIAL_TICK,
        deadline_tick: DEADLINE_TICK,
    })
    .expect_err("draining listener must refuse new session");
    assert!(denied.contains(&TransportIssue::ProtocolDraining));

    let premature = apply_transport_command(&profile, &draining.next, &TransportCommand::CleanupListener {
        operation_id: OPERATION_REF.to_string(),
        alpn: descriptor.alpn.clone(),
        service_id: descriptor.service_id.clone(),
        generation: GENERATION_ONE,
        cleanup_evidence_ref: CLEANUP_REF.to_string(),
    })
    .expect_err("active session blocks cleanup");
    assert!(premature.contains(&TransportIssue::ActiveSessionsRemain));

    let closed = apply_transport_command(&profile, &draining.next, &TransportCommand::CloseSession {
        operation_id: OPERATION_REF.to_string(),
        session_id: scoped_id(SESSION_REF, GENERATION_ONE),
    })
    .expect("close draining session");
    let cleaned = apply_transport_command(&profile, &closed.next, &TransportCommand::CleanupListener {
        operation_id: OPERATION_REF.to_string(),
        alpn: descriptor.alpn.clone(),
        service_id: descriptor.service_id,
        generation: GENERATION_ONE,
        cleanup_evidence_ref: CLEANUP_REF.to_string(),
    })
    .expect("cleanup listener");
    assert!(!cleaned.next.protocols.contains_key(&descriptor.alpn));
}
