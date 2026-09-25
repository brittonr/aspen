
// r[verify molten.fabric_transport.cross_process_listener]
// r[verify molten.fabric_transport.cross_process_session]
// r[verify molten.fabric_transport.cross_process_validation]
#[tokio::test]
async fn live_listener_and_client_exchange_one_bounded_frame_and_clean_up() {
    let mut listener = listener().await;
    assert!(listener.state().is_ready());
    assert_eq!(listener.profile().profile.adapter_kind, TransportAdapterKind::IrohLive);
    assert_eq!(listener.admission(), EndpointAdmissionState::fully_active());
    let endpoint = listener.handoff().clone();
    let timeout = std::time::Duration::from_secs(TEST_TIMEOUT_SECONDS);
    let server = listener.accept_one_frame(SESSION_REF, REQUEST_REF, timeout);
    let client = exchange_cross_process_frame(client_input(endpoint), PAYLOAD, timeout);
    let (server, client) = tokio::join!(server, client);
    let received = server.expect("server exchange");
    let client = client.expect("client exchange");
    assert_eq!(received.payload, PAYLOAD);
    let received_debug = format!("{received:?}");
    assert!(!received_debug.contains(std::str::from_utf8(PAYLOAD).expect("UTF-8 payload")));
    let server = received.evidence;

    assert_eq!(server.role, EndpointParticipantRole::Listener);
    assert_eq!(client.role, EndpointParticipantRole::Client);
    assert_eq!(server.descriptor_ref, client.descriptor_ref);
    assert_eq!(server.payload_ref, client.payload_ref);
    assert_eq!(server.acknowledgement_ref, client.acknowledgement_ref);
    assert_eq!(server.delivery, DeliveryOutcome::Delivered);
    assert_eq!(client.delivery, DeliveryOutcome::Delivered);
    assert_eq!(server.automatic_retry_count, 0);
    assert_eq!(client.automatic_retry_count, 0);
    assert_eq!(listener.state().active_sessions, 0);
    let evidence_text = format!("{server:?}{client:?}");
    assert!(!evidence_text.contains(std::str::from_utf8(PAYLOAD).expect("UTF-8 payload")));
    assert!(!evidence_text.contains("iroh::Connection"));

    let cleanup = listener.drain_and_close(ListenerDrainReason::OperatorRequest).await.expect("listener cleanup");
    assert_eq!(cleanup.drain_reason, ListenerDrainReason::OperatorRequest);
    assert_eq!(cleanup.terminal_class, ListenerTerminalClass::Clean);
    assert!(cleanup.cleanup_evidence_ref.starts_with("blake3:"));
}

// r[verify molten.fabric_transport.cross_process_endpoint]
// r[verify molten.fabric_transport.cross_process_session]
// r[verify molten.fabric_transport.cross_process_validation]
#[tokio::test]
async fn client_preflight_denies_wrong_protocol_and_oversized_payload_before_dial() {
    let listener = listener().await;
    let endpoint = listener.handoff().clone();
    let mut wrong_protocol = client_input(endpoint.clone());
    wrong_protocol.expected.alpn = "molten/wrong/1".to_string();
    let error =
        exchange_cross_process_frame(wrong_protocol, PAYLOAD, std::time::Duration::from_secs(TEST_TIMEOUT_SECONDS))
            .await
            .expect_err("wrong ALPN must deny");
    assert!(error.to_string().contains("AlpnMismatch"));

    let oversized_len = usize::try_from(FRAME_LIMIT + 1).expect("oversized payload length");
    let oversized = vec![0_u8; oversized_len];
    let error = exchange_cross_process_frame(
        client_input(endpoint),
        &oversized,
        std::time::Duration::from_secs(TEST_TIMEOUT_SECONDS),
    )
    .await
    .expect_err("oversized payload must deny");
    assert!(error.to_string().contains("exceeds the admitted frame bound"));

    let cleanup = listener.drain_and_close(ListenerDrainReason::Cancellation).await.expect("cancel listener");
    assert_eq!(cleanup.drain_reason, ListenerDrainReason::Cancellation);
    assert_eq!(cleanup.terminal_class, ListenerTerminalClass::Cancelled);
}

// r[verify molten.fabric_transport.cross_process_listener]
// r[verify molten.fabric_transport.cross_process_validation]
#[tokio::test]
async fn listener_accept_timeout_is_bounded_and_does_not_publish_a_false_session() {
    let mut listener = listener().await;
    let error = listener
        .accept_one(SESSION_REF, REQUEST_REF, std::time::Duration::from_millis(ACCEPT_TIMEOUT_MILLISECONDS))
        .await
        .expect_err("accept without a client must time out");
    assert!(error.to_string().contains("accept timed out"));
    assert!(listener.state().is_ready());
    assert_eq!(listener.state().active_sessions, 0);
    assert_eq!(listener.state().accepted_sessions, 0);
    let cleanup = listener
        .drain_and_close(ListenerDrainReason::OperatorRequest)
        .await
        .expect("listener cleanup after timeout");
    assert_eq!(cleanup.drain_reason, ListenerDrainReason::OperatorRequest);
    assert_eq!(cleanup.terminal_class, ListenerTerminalClass::Clean);
}

// r[verify molten.fabric_transport.cross_process_endpoint]
// r[verify molten.fabric_transport.cross_process_validation]
#[test]
fn public_shell_surface_contains_no_runtime_handle_accessor_or_ambient_fallback() {
    let source = concat!(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/fabric_transport/cross_process/iroh/parts/shell/p000/body.rs")),
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/fabric_transport/cross_process/iroh/parts/shell/p001/body.rs")),
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/fabric_transport/cross_process/iroh/parts/shell/p002/body.rs")),
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/fabric_transport/cross_process/iroh/parts/shell/p003/body.rs")),
    );
    for forbidden in [
        "pub fn endpoint(",
        "pub fn connection(",
        "pub fn socket(",
        "pub fn executor(",
        "pub endpoint: iroh::Endpoint",
        "pub connection: iroh::endpoint::Connection",
        "std::env::",
        "UdpSocket::bind",
        "TcpListener::bind",
    ] {
        assert!(!source.contains(forbidden), "forbidden shell surface: {forbidden}");
    }
    assert!(source.contains(".bind_addr(bind_addr)"));
    assert!(source.contains("RelayMode::Disabled"));
    assert!(!source.contains("automatic_retry_count +="));
}

// r[verify molten.fabric_transport.cross_process_session]
// r[verify molten.fabric_transport.distinct_process_evidence]
// r[verify molten.fabric_transport.cross_process_validation]
#[tokio::test(flavor = "multi_thread")]
async fn registered_effect_port_routes_a_live_cross_process_frame_without_consumer_iroh_branches() {
    use crate::system_extension::FabricEffectPort;

    let listener = listener().await;
    let endpoint = listener.handoff().clone();
    let listener_task = tokio::spawn(accept_one_and_drain(listener));

    let live_profile = profile();
    let context = ExtensionTransportContext::from_test_snapshot(&protocol().service_id, GENERATION, &live_profile);
    let client = IrohCrossProcessEffectClientConfig {
        capability: capability(CLIENT_SECRET_BYTE, CLIENT_CAPABILITY_REF),
        bind_addr: bind_addr(),
        expected: expected(&endpoint),
        endpoint,
        admission: EndpointAdmissionState::fully_active(),
        timeout: std::time::Duration::from_secs(TEST_TIMEOUT_SECONDS),
    };
    let mut port = RegisteredCrossProcessTransportEffectPort::new(context, live_profile.clone(), protocol(), client)
        .expect("cross-process effect port");
    let binding = effect_binding(&live_profile);
    let (session_id, stream_id) = effect_port_ids();
    let setup = setup_commands(&session_id, &stream_id);
    for (request_ref, operation, command) in setup {
        port.register(request_ref.to_string(), command, None).expect("register setup effect");
        let effect = effect_request(&binding, operation, request_ref, 0);
        let output = port.route(&binding, &effect).expect("route setup effect");
        assert!(output.output_ref.starts_with("blake3:"));
    }

    let payload_bytes = u64::try_from(PAYLOAD.len()).expect("payload length");
    let payload_ref = cross_process_frame_ref(REQUEST_REF, PAYLOAD);
    let send = TransportCommand::SendFrame {
        operation_id: REQUEST_REF.to_string(),
        session_id,
        stream_id,
        payload_ref,
        payload_bytes,
        observed_tick: OBSERVED_TICK,
    };
    assert!(
        port.register(VALIDITY_REF.to_string(), send.clone(), Some(PAYLOAD.to_vec())).is_err(),
        "request-bound payload ref substitution must deny"
    );
    assert!(
        port.register(REQUEST_REF.to_string(), send.clone(), None).is_err(),
        "missing live payload must deny"
    );
    port.register(REQUEST_REF.to_string(), send, Some(PAYLOAD.to_vec()))
        .expect("register live send effect");
    let effect = effect_request(&binding, "send-frame", REQUEST_REF, payload_bytes);
    let output = port.route(&binding, &effect).expect("route live send effect");
    assert!(output.output_ref.starts_with("blake3:"));
    assert_eq!(port.adapter().state().sessions[SESSION_REF].inflight_bytes, 0);
    let client_frame = port.latest_frame_evidence().cloned().expect("client frame evidence");
    assert_eq!(client_frame.delivery, DeliveryOutcome::Delivered);
    assert_eq!(client_frame.automatic_retry_count, 0);
    assert!(port.route(&binding, &effect).is_err(), "effect replay must deny");

    let (listener_frame, cleanup) = listener_task.await.expect("listener task");
    assert_eq!(listener_frame.payload_ref, client_frame.payload_ref);
    assert_eq!(cleanup.terminal_class, ListenerTerminalClass::Clean);
}

/// Accepts the one effect-port frame, then drains and closes the listener on operator request.
async fn accept_one_and_drain(
    mut listener: IrohCrossProcessListener,
) -> (CrossProcessFrameEvidence, CrossProcessListenerCleanup) {
    let frame = listener
        .accept_one(SESSION_REF, REQUEST_REF, std::time::Duration::from_secs(TEST_TIMEOUT_SECONDS))
        .await
        .expect("effect-port listener frame");
    let cleanup = listener
        .drain_and_close(ListenerDrainReason::OperatorRequest)
        .await
        .expect("effect-port listener cleanup");
    (frame, cleanup)
}

fn effect_port_ids() -> (ScopedTransportId, ScopedTransportId) {
    let session_id = ScopedTransportId {
        opaque_ref: SESSION_REF.to_string(),
        service_id: protocol().service_id,
        generation: GENERATION,
    };
    let stream_id = ScopedTransportId {
        opaque_ref: WRONG_REF.to_string(),
        service_id: session_id.service_id.clone(),
        generation: GENERATION,
    };
    (session_id, stream_id)
}

/// Register-protocol, open-session, and open-stream setup effects with their request refs and
/// operations.
fn setup_commands(
    session_id: &ScopedTransportId,
    stream_id: &ScopedTransportId,
) -> [(&'static str, &'static str, TransportCommand); 3] {
    [
        (PROFILE_REF, "register-protocol", TransportCommand::Register {
            operation_id: AUTHORITY_REF.to_string(),
            descriptor: protocol(),
        }),
        (FRAMING_REF, "open-session", TransportCommand::OpenSession {
            operation_id: AUTHORITY_REF.to_string(),
            session_id: session_id.clone(),
            alpn: protocol().alpn,
            direction: SessionDirection::Outbound,
            peer: PeerIdentityRefs {
                transport_identity_ref: PEER_CONTEXT_REF.to_string(),
                membership_ref: Some(LOCATOR_COHORT_REF.to_string()),
                application_principal_ref: Some(VALIDITY_REF.to_string()),
                trust_decision_ref: Some(LISTENER_CAPABILITY_REF.to_string()),
                capability_authority_ref: Some(CLIENT_CAPABILITY_REF.to_string()),
                bootstrap_policy_ref: None,
            },
            observed_tick: OBSERVED_TICK,
            deadline_tick: OBSERVED_TICK + DEADLINE_WINDOW,
        }),
        (AUTHORITY_REF, "open-stream", TransportCommand::OpenStream {
            operation_id: AUTHORITY_REF.to_string(),
            session_id: session_id.clone(),
            stream_id: stream_id.clone(),
            direction: StreamDirection::Bidirectional,
            initial_credit_bytes: FRAME_LIMIT,
        }),
    ]
}
