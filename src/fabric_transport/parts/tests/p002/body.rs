// r[verify molten.fabric_transport.receive_gate]
#[test]
fn receive_gate_preserves_in_budget_deterministic_adapter_transition_bytes() {
    let selected = profile(TransportAdapterKind::DeterministicSimulation);
    let mut baseline = DeterministicAdapter::new(selected.clone()).expect("baseline adapter");
    let mut gated = DeterministicAdapter::new(selected).expect("gated adapter");
    let mut gate = ReceiveGate::new(ReceiveBudget { burst: 1, refill_ticks: 10, max_sources: 1 }).expect("budget");
    let source = ReceiveSource::from(std::net::Ipv4Addr::LOCALHOST);
    let observation = ReceiveObservation {
        peer_identity: "authenticated-test-endpoint".to_owned(), peer_context_ref: PEER_REF.to_owned(),
        service_id: descriptor().service_id, alpn: descriptor().alpn, generation: GENERATION,
        role: EndpointParticipantRole::Client,
    };
    let _admitted = gate.admit_inbound(source, observation.clone(), INITIAL_TICK).expect("receive gate");
    let commands = setup_commands().into_iter().chain([
        TransportCommand::SendFrame { operation_id: OPERATION_REF.to_owned(),
            session_id: id(SESSION_REF, GENERATION), stream_id: id(STREAM_REF, GENERATION),
            payload_ref: REQUEST_REF.to_owned(), payload_bytes: PAYLOAD.len() as u64, observed_tick: INITIAL_TICK },
        TransportCommand::AcknowledgeFrame { operation_id: OPERATION_REF.to_owned(),
            session_id: id(SESSION_REF, GENERATION), stream_id: id(STREAM_REF, GENERATION),
            payload_bytes: PAYLOAD.len() as u64 },
        TransportCommand::HalfCloseStream { operation_id: OPERATION_REF.to_owned(),
            session_id: id(SESSION_REF, GENERATION), stream_id: id(STREAM_REF, GENERATION), send_direction: true },
        TransportCommand::CloseStream { operation_id: OPERATION_REF.to_owned(),
            session_id: id(SESSION_REF, GENERATION), stream_id: id(STREAM_REF, GENERATION) },
        TransportCommand::CloseSession { operation_id: OPERATION_REF.to_owned(), session_id: id(SESSION_REF, GENERATION) },
    ]);
    for command in commands {
        let before = baseline.execute_command(&command).expect("baseline transition");
        let after = gated.execute_command(&command).expect("gated transition");
        assert_eq!(before.transition_ref, after.transition_ref);
        assert_eq!(before.value, after.value);
    }
    let state = gated.state().clone();
    let denied = gate.admit_inbound(source, observation, INITIAL_TICK).expect_err("source is over budget");
    assert_eq!(denied.reason, ReceiveDenyReason::RateLimited);
    assert_eq!(gated.state(), &state, "over-budget receive never enters costly command admission");
}

// r[verify molten.fabric_transport.direct_lan]
#[test]
fn direct_peer_binding_denies_early_data_and_every_authenticated_binding_mismatch() {
    let protocol = descriptor();
    let expected = DirectPeerBinding {
        endpoint_id: "authenticated-test-endpoint".to_owned(), peer_context_ref: PEER_REF.to_owned(),
        service_id: protocol.service_id.clone(), alpn: protocol.alpn.clone(), generation: GENERATION,
        remote_role: EndpointParticipantRole::Client,
    };
    let observed = ReceiveObservation {
        peer_identity: expected.endpoint_id.clone(), peer_context_ref: expected.peer_context_ref.clone(),
        service_id: expected.service_id.clone(), alpn: expected.alpn.clone(), generation: GENERATION,
        role: EndpointParticipantRole::Client,
    };
    let mut gate = ReceiveGate::new(ReceiveBudget { burst: 8, refill_ticks: 10, max_sources: 1 }).expect("budget");
    let source = ReceiveSource::from(std::net::Ipv4Addr::LOCALHOST);
    let admitted = gate.admit_inbound(source, observed.clone(), 0).expect("admitted");
    assert_eq!(validate_direct_peer(&admitted, &expected, &protocol, true), Ok(()));
    assert_eq!(validate_direct_peer(&admitted, &expected, &protocol, false), Err(DirectStreamIssue::EarlyData));
    for issue in [DirectStreamIssue::WrongPeer, DirectStreamIssue::WrongContext, DirectStreamIssue::WrongService,
        DirectStreamIssue::WrongAlpn, DirectStreamIssue::StaleGeneration, DirectStreamIssue::WrongRole] {
        let mut changed = observed.clone();
        match issue {
            DirectStreamIssue::WrongPeer => changed.peer_identity = "different-authenticated-peer".to_owned(),
            DirectStreamIssue::WrongContext => changed.peer_context_ref = AUTHORITY_REF.to_owned(),
            DirectStreamIssue::WrongService => changed.service_id = "other-service".to_owned(),
            DirectStreamIssue::WrongAlpn => changed.alpn = "other/alpn".to_owned(),
            DirectStreamIssue::StaleGeneration => changed.generation += 1,
            DirectStreamIssue::WrongRole => changed.role = EndpointParticipantRole::Listener,
            _ => unreachable!("closed test case inventory"),
        }
        let admitted = gate.admit_inbound(source, changed, 0).expect("cheap gate admits bounded observation");
        assert_eq!(validate_direct_peer(&admitted, &expected, &protocol, true), Err(issue));
    }
}

// r[verify molten.fabric_transport.direct_stream_bounds]
#[test]
fn direct_stream_payload_memory_and_connection_limits_are_explicit() {
    let mut selected = profile(TransportAdapterKind::IrohLive).profile;
    selected.profile_id = DIRECT_LAN_STREAM_PROFILE.to_owned();
    selected.limits.max_sessions = 1;
    selected.limits.max_streams_per_session = 1;
    let protocol = descriptor();
    let bounds = DirectStreamBounds { max_connections: 1, max_frame_bytes: 1024, max_total_bytes: 8192,
        max_frames: 16, max_buffered_bytes: 7168, idle_timeout_ms: 100, total_timeout_ms: 1000,
        cleanup_timeout_ms: 1000 };
    assert_eq!(validate_direct_stream_bounds(&selected, &protocol, &bounds), Ok(()));
    let mut changed = bounds.clone();
    changed.max_buffered_bytes -= 1;
    assert_eq!(validate_direct_stream_bounds(&selected, &protocol, &changed), Err(DirectStreamIssue::InvalidBounds));
    changed = bounds;
    changed.max_connections = 2;
    assert_eq!(validate_direct_stream_bounds(&selected, &protocol, &changed), Err(DirectStreamIssue::InvalidBounds));
}
