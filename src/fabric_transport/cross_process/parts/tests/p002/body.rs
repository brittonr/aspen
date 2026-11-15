
fn registered_direct_stream_fixture() -> (Registered, crate::fabric::CanonicalPortBinding, RegisteredStreamRequest) {
    let mut profile_input = profile().profile;
    profile_input.profile_id = DIRECT_LAN_STREAM_PROFILE.to_string();
    profile_input.limits.max_sessions = 1;
    profile_input.limits.max_streams_per_session = 1;
    let live_profile = crate::fabric_transport::profile(&profile_input).expect("direct profile");
    let mut bindings = fake_bindings();
    let remote_id = iroh::SecretKey::from_bytes(&[LISTENER_SECRET_BYTE; IROH_SECRET_KEY_BYTES]).public();
    bindings.public_endpoint_identity = format!("iroh:{remote_id}");
    bindings.resources.max_sessions = 1;
    let endpoint = endpoint(&live_profile.profile, &protocol(), &bindings).expect("direct endpoint");
    let client = IrohClientConfig {
        capability: capability(CLIENT_SECRET_BYTE, CLIENT_CAPABILITY_REF),
        bind_addr: bind_addr(),
        expected: expected(&endpoint),
        endpoint,
        admission: EndpointAdmissionState::fully_active(),
        timeout: std::time::Duration::from_secs(TEST_TIMEOUT_SECONDS),
    };
    let context = ExtensionContext::from_test_snapshot(&protocol().service_id, GENERATION, &live_profile);
    let port = Registered::new(context, live_profile.clone(), protocol(), client).expect("direct effect port");
    let request = RegisteredStreamRequest {
        peer: DirectPeerBinding {
            endpoint_id: remote_id.to_string(),
            peer_context_ref: PEER_CONTEXT_REF.to_string(),
            service_id: protocol().service_id,
            alpn: protocol().alpn,
            generation: GENERATION,
            remote_role: EndpointParticipantRole::Listener,
        },
        bounds: DirectStreamBounds {
            max_connections: 1,
            max_frame_bytes: FRAME_LIMIT,
            max_total_bytes: FRAME_LIMIT,
            max_frames: DEADLINE_WINDOW,
            max_buffered_bytes: 6 * FRAME_LIMIT + 1024,
            idle_timeout_ms: 100,
            total_timeout_ms: 1000,
            cleanup_timeout_ms: 1000,
        },
        operation: DirectStreamRequest {
            request_ref: AUTHORITY_REF.to_string(),
            operation_ref: AUTHORITY_REF.to_string(),
            resource_ref: VALIDITY_REF.to_string(),
            generation: GENERATION,
        },
        clock: std::time::Instant::now,
    };
    (port, effect_binding(&live_profile), request)
}

// r[verify molten.fabric_transport.direct_lan]
#[test]
fn registered_direct_stream_ordinary_route_cannot_report_socket_completion() {
    use crate::system_extension::FabricEffectPort;

    let (mut port, binding, _) = registered_direct_stream_fixture();
    let (session_id, stream_id) = effect_port_ids();
    for (request_ref, operation, command) in setup_commands(&session_id, &stream_id).into_iter().take(2) {
        port.register(request_ref.to_string(), command, None).expect("register control setup");
        port.route(&binding, &effect_request(&binding, operation, request_ref, 0)).expect("route control setup");
    }
    let (request_ref, operation, command) = setup_commands(&session_id, &stream_id)[2].clone();
    port.register(request_ref.to_string(), command, None).expect("register direct stream");
    let before = port.adapter().state().clone();
    let error = port.route(&binding, &effect_request(&binding, operation, request_ref, 0))
        .expect_err("ordinary routing cannot complete a protected stream without a service");
    assert_eq!(error, crate::fabric::FabricPortError::malformed("direct stream effect requires proxy_stream"));
    assert_eq!(port.adapter().state(), &before, "denial must precede the OpenStream transition");
    assert!(port.latest_frame_evidence().is_none());
}

struct UnreachableProtectedService;

impl ProtectedSocketService for UnreachableProtectedService {
    fn authorize(&mut self, _: GatedInbound, _: &DirectStreamRequest) -> crate::fabric::FabricPortResult<String> {
        panic!("crossed operation reached authority dispatch");
    }

    fn check_current(&mut self, _: &direct::AdmittedOperation) -> crate::fabric::FabricPortResult<()> {
        panic!("crossed operation reached current-authority dispatch");
    }

    async fn open(&mut self, _: &direct::AdmittedOperation) -> crate::fabric::FabricPortResult<tokio::net::UnixStream> {
        panic!("crossed operation opened a protected socket");
    }
}

// r[verify molten.fabric_transport.direct_lan]
#[tokio::test]
async fn registered_direct_stream_crossed_operation_denies_before_control_apply() {
    use crate::system_extension::FabricEffectPort;

    let (mut port, binding, mut request) = registered_direct_stream_fixture();
    let (session_id, stream_id) = effect_port_ids();
    for (setup_ref, setup_operation, setup_command) in setup_commands(&session_id, &stream_id).into_iter().take(2) {
        port.register(setup_ref.to_string(), setup_command, None).expect("register control setup");
        port.route(&binding, &effect_request(&binding, setup_operation, setup_ref, 0)).expect("route control setup");
    }
    let (request_ref, operation, command) = setup_commands(&session_id, &stream_id)[2].clone();
    port.register(request_ref.to_string(), command, None).expect("register direct stream");
    request.operation.operation_ref = WRONG_REF.to_string();
    let before = port.adapter().state().clone();
    let (_cancel, cancellation) = tokio::sync::watch::channel(false);
    let error = port.proxy_stream(
        &binding,
        &effect_request(&binding, operation, request_ref, 0),
        request,
        &mut UnreachableProtectedService,
        cancellation,
    ).await.expect_err("crossed operation must deny before opening the control stream");
    assert_eq!(error, crate::fabric::FabricPortError::malformed("direct stream operation mismatch"));
    assert_eq!(port.adapter().state(), &before, "crossed operation must not apply any transport transition");
}