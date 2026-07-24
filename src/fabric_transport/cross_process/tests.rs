use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::time::Duration;

use preserves::ValueImpl;

use super::*;
use crate::fabric_transport::*;

const PROFILE_REF: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const FRAMING_REF: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const AUTHORITY_REF: &str = "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const LISTENER_REF: &str = "blake3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const PEER_CONTEXT_REF: &str = "blake3:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const LOCATOR_COHORT_REF: &str = "blake3:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
const VALIDITY_REF: &str = "blake3:1111111111111111111111111111111111111111111111111111111111111111";
const LISTENER_CAPABILITY_REF: &str = "blake3:2222222222222222222222222222222222222222222222222222222222222222";
const CLIENT_CAPABILITY_REF: &str = "blake3:3333333333333333333333333333333333333333333333333333333333333333";
const SESSION_REF: &str = "blake3:4444444444444444444444444444444444444444444444444444444444444444";
const REQUEST_REF: &str = "blake3:5555555555555555555555555555555555555555555555555555555555555555";
const WRONG_REF: &str = "blake3:6666666666666666666666666666666666666666666666666666666666666666";
const FAKE_ENDPOINT_ID: &str = "iroh:abcdefghijklmnopqrstuvwxyz234567";
const FAKE_IP_LOCATOR: &str = "ip:127.0.0.1:49152";
const PROFILE_LIMIT: u64 = 8;
const FRAME_LIMIT: u64 = 4_096;
const DATAGRAM_LIMIT: u64 = 1_024;
const QUEUE_LIMIT: u64 = 16_384;
const INFLIGHT_LIMIT: u64 = 8_192;
const DEADLINE_WINDOW: u64 = 64;
const LENGTH_PREFIX_BYTES: u64 = 8;
const GENERATION: u64 = 1;
const VALID_FROM_TICK: u64 = 1;
const VALID_UNTIL_TICK: u64 = 100;
const OBSERVED_TICK: u64 = 10;
pub(crate) const TEST_TIMEOUT_SECONDS: u64 = 10;
const ACCEPT_TIMEOUT_MILLISECONDS: u64 = 25;
const LISTENER_SECRET_BYTE: u8 = 7;
const CLIENT_SECRET_BYTE: u8 = 9;
const PAYLOAD: &[u8] = b"bounded-cross-process-iroh-frame";
const OUTER_BINDING_INDEX: usize = 2;
const OUTER_CHECKS_INDEX: usize = 3;
const OUTER_FIELD_COUNT: usize = 4;

fn profile() -> CanonicalTransportProfile {
    canonical_transport_profile(&TransportProfile {
        schema: TRANSPORT_PROFILE_SCHEMA.to_string(),
        profile_id: "iroh-cross-process-v1".to_string(),
        profile_ref: PROFILE_REF.to_string(),
        adapter_kind: TransportAdapterKind::IrohLive,
        capabilities: vec![
            TransportCapability::BidirectionalStreams,
            TransportCapability::UnidirectionalStreams,
        ],
        limits: TransportLimits {
            max_listeners: PROFILE_LIMIT,
            max_sessions: PROFILE_LIMIT,
            max_streams_per_session: PROFILE_LIMIT,
            max_frame_bytes: FRAME_LIMIT,
            max_datagram_bytes: DATAGRAM_LIMIT,
            max_queued_events: PROFILE_LIMIT,
            max_queued_bytes: QUEUE_LIMIT,
            max_inflight_bytes: INFLIGHT_LIMIT,
            operation_deadline_ticks: DEADLINE_WINDOW,
        },
        non_claims: REQUIRED_TRANSPORT_NON_CLAIMS.to_vec(),
    })
    .expect("canonical transport profile")
}

fn protocol() -> ProtocolDescriptor {
    ProtocolDescriptor {
        schema: TRANSPORT_PROTOCOL_SCHEMA.to_string(),
        protocol_id: "cross-process-echo".to_string(),
        version: "v1".to_string(),
        alpn: "molten/cross-process-echo/1".to_string(),
        extension_id: "cross-process-extension".to_string(),
        service_id: "cross-process-service".to_string(),
        generation: GENERATION,
        listener_limit: 1,
        requested_capabilities: vec![
            TransportCapability::BidirectionalStreams,
            TransportCapability::UnidirectionalStreams,
        ],
        framing: FramingProfile {
            profile_id: "length-prefixed-blake3-v1".to_string(),
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

fn validity() -> EndpointValidityCohort {
    EndpointValidityCohort {
        cohort_ref: VALIDITY_REF.to_string(),
        not_before_tick: VALID_FROM_TICK,
        expires_at_tick: VALID_UNTIL_TICK,
    }
}

fn disclosure() -> EndpointDisclosurePolicy {
    EndpointDisclosurePolicy {
        explicit_handoff_classes: vec![EndpointLocatorClass::Ip],
        default_readback_redacted: true,
    }
}

fn fake_bindings() -> EndpointDescriptorBindings {
    EndpointDescriptorBindings {
        public_endpoint_identity: FAKE_ENDPOINT_ID.to_string(),
        listener_identity_ref: LISTENER_REF.to_string(),
        expected_peer_context_ref: PEER_CONTEXT_REF.to_string(),
        locator_cohort_ref: LOCATOR_COHORT_REF.to_string(),
        locators: vec![EndpointLocator {
            class: EndpointLocatorClass::Ip,
            value: FAKE_IP_LOCATOR.to_string(),
        }],
        disclosure: disclosure(),
        resources: EndpointResourceBounds {
            max_sessions: PROFILE_LIMIT,
            max_frame_bytes: FRAME_LIMIT,
            max_queued_bytes: QUEUE_LIMIT,
            max_inflight_bytes: INFLIGHT_LIMIT,
        },
        validity: validity(),
    }
}

fn capability(byte: u8, capability_ref: &str) -> IrohEndpointCapability {
    IrohEndpointCapability::from_secret_bytes([byte; IROH_SECRET_KEY_BYTES], capability_ref.to_string())
        .expect("endpoint capability")
}

fn bind_addr() -> SocketAddr {
    SocketAddr::from((Ipv4Addr::LOCALHOST, 0))
}

fn expected(endpoint: &CanonicalCrossProcessEndpoint) -> ExpectedEndpointBinding {
    let descriptor = &endpoint.descriptor;
    ExpectedEndpointBinding {
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
        observed_tick: OBSERVED_TICK,
    }
}

pub(crate) async fn listener() -> IrohCrossProcessListener {
    listener_with_secret(LISTENER_SECRET_BYTE).await
}

pub(crate) async fn listener_with_secret(secret_byte: u8) -> IrohCrossProcessListener {
    IrohCrossProcessListener::bind(IrohCrossProcessListenerInput {
        profile: profile(),
        protocol: protocol(),
        capability: capability(secret_byte, LISTENER_CAPABILITY_REF),
        bind_addr: bind_addr(),
        listener_identity_ref: LISTENER_REF.to_string(),
        expected_peer_context_ref: PEER_CONTEXT_REF.to_string(),
        locator_cohort_ref: LOCATOR_COHORT_REF.to_string(),
        disclosure: disclosure(),
        validity: validity(),
        admission: EndpointAdmissionState::fully_active(),
        observed_tick: OBSERVED_TICK,
    })
    .await
    .expect("cross-process listener")
}

pub(crate) fn client_input(endpoint: CanonicalCrossProcessEndpoint) -> IrohCrossProcessClientInput {
    IrohCrossProcessClientInput {
        profile: profile(),
        protocol: protocol(),
        capability: capability(CLIENT_SECRET_BYTE, CLIENT_CAPABILITY_REF),
        bind_addr: bind_addr(),
        expected: expected(&endpoint),
        endpoint,
        admission: EndpointAdmissionState::fully_active(),
        session_ref: SESSION_REF.to_string(),
        request_ref: REQUEST_REF.to_string(),
    }
}

fn effect_binding(profile: &CanonicalTransportProfile) -> crate::fabric::CanonicalFabricPortBinding {
    use crate::fabric::FabricPortRequirement;
    use crate::fabric::resolve_canonical_fabric_port_binding;

    let descriptor = fabric_transport_port_descriptor(profile);
    resolve_canonical_fabric_port_binding(std::slice::from_ref(&descriptor), &FabricPortRequirement {
        port_id: descriptor.port_id.clone(),
        version: descriptor.version.clone(),
        class: descriptor.class,
        operation_classes: descriptor.operation_classes.clone(),
        input_schema_refs: descriptor.input_schema_refs.clone(),
        output_schema_refs: descriptor.output_schema_refs.clone(),
        allowed_authorities: descriptor.authority_requirements.clone(),
        available_resources: descriptor.resource_requirements.clone(),
        expected_determinism: descriptor.determinism,
        expected_replay: descriptor.replay,
        expected_profile: descriptor.implementation_profile.clone(),
    })
    .expect("cross-process transport binding")
}

fn effect_request(
    binding: &crate::fabric::CanonicalFabricPortBinding,
    operation: &str,
    request_ref: &str,
    accounted_bytes: u64,
) -> crate::system_extension::TypedEffectRequest {
    crate::system_extension::TypedEffectRequest {
        target: crate::system_extension::EffectTarget::FabricPort(binding.binding.key.clone()),
        operation: operation.to_string(),
        input_schema_ref: TRANSPORT_COMMAND_SCHEMA.to_string(),
        output_schema_ref: TRANSPORT_EVENT_SCHEMA.to_string(),
        request_ref: request_ref.to_string(),
        generation: GENERATION,
        accounted_bytes,
    }
}

// r[verify molten.fabric_transport.cross_process_endpoint]
// r[verify molten.fabric_transport.cross_process_validation]
#[test]
fn canonical_endpoint_roundtrips_and_default_status_redacts_raw_locators() {
    let endpoint = canonical_cross_process_endpoint(&profile().profile, &protocol(), &fake_bindings())
        .expect("canonical endpoint");
    let parsed = parse_canonical_cross_process_endpoint(&endpoint.value).expect("parsed endpoint");
    assert_eq!(parsed, endpoint);
    let handoff_text = crate::preserves_rail::to_text(&endpoint.value).expect("handoff text");
    assert!(handoff_text.contains(FAKE_IP_LOCATOR));

    let status = canonical_endpoint_status(&endpoint.descriptor).expect("endpoint status");
    let status_text = crate::preserves_rail::to_text(&status.value).expect("status text");
    assert!(status_text.contains("locator-classes"));
    assert!(!status_text.contains(FAKE_IP_LOCATOR));
    assert!(!status_text.contains("private-key"));
    assert!(!status_text.contains("iroh::Endpoint"));
}

// r[verify molten.fabric_transport.cross_process_endpoint]
// r[verify molten.fabric_transport.cross_process_validation]
#[test]
fn canonical_endpoint_import_rejects_a_tampered_binding_reference() {
    let endpoint = canonical_cross_process_endpoint(&profile().profile, &protocol(), &fake_bindings())
        .expect("canonical endpoint");
    let fields = endpoint
        .value
        .collect_simple_record("fabric-transport-endpoint-descriptor-v1", Some(OUTER_FIELD_COUNT))
        .expect("endpoint outer record");
    let values = fields.iter().collect::<Vec<_>>();
    let tampered = crate::preserves_rail::record("fabric-transport-endpoint-descriptor-v1", vec![
        crate::preserves_rail::string(CROSS_PROCESS_ENDPOINT_HANDOFF_SCHEMA),
        crate::preserves_rail::string(WRONG_REF),
        crate::preserves_rail::value_to_iovalue(&values[OUTER_BINDING_INDEX]),
        crate::preserves_rail::value_to_iovalue(&values[OUTER_CHECKS_INDEX]),
    ]);
    let error = parse_canonical_cross_process_endpoint(&tampered).expect_err("tampered ref must deny");
    assert!(error.to_string().contains("descriptor ref mismatch"));
}

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
    let timeout = Duration::from_secs(TEST_TIMEOUT_SECONDS);
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
    let error = exchange_cross_process_frame(wrong_protocol, PAYLOAD, Duration::from_secs(TEST_TIMEOUT_SECONDS))
        .await
        .expect_err("wrong ALPN must deny");
    assert!(error.to_string().contains("AlpnMismatch"));

    let oversized_len = usize::try_from(FRAME_LIMIT + 1).expect("oversized payload length");
    let oversized = vec![0_u8; oversized_len];
    let error =
        exchange_cross_process_frame(client_input(endpoint), &oversized, Duration::from_secs(TEST_TIMEOUT_SECONDS))
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
        .accept_one(SESSION_REF, REQUEST_REF, Duration::from_millis(ACCEPT_TIMEOUT_MILLISECONDS))
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
    let source = include_str!("iroh_shell.rs");
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

    let mut listener = listener().await;
    let endpoint = listener.handoff().clone();
    let listener_task = tokio::spawn(async move {
        let frame = listener
            .accept_one(SESSION_REF, REQUEST_REF, Duration::from_secs(TEST_TIMEOUT_SECONDS))
            .await
            .expect("effect-port listener frame");
        let cleanup = listener
            .drain_and_close(ListenerDrainReason::OperatorRequest)
            .await
            .expect("effect-port listener cleanup");
        (frame, cleanup)
    });

    let live_profile = profile();
    let context = ExtensionTransportContext::from_test_snapshot(&protocol().service_id, GENERATION, &live_profile);
    let client = IrohCrossProcessEffectClientConfig {
        capability: capability(CLIENT_SECRET_BYTE, CLIENT_CAPABILITY_REF),
        bind_addr: bind_addr(),
        expected: expected(&endpoint),
        endpoint,
        admission: EndpointAdmissionState::fully_active(),
        timeout: Duration::from_secs(TEST_TIMEOUT_SECONDS),
    };
    let mut port = RegisteredCrossProcessTransportEffectPort::new(context, live_profile.clone(), protocol(), client)
        .expect("cross-process effect port");
    let binding = effect_binding(&live_profile);
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
    let setup = [
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
    ];
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
