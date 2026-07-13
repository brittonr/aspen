use super::*;

const PROFILE_DECLARATION_REF: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const FRAMING_REF: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const AUTHORITY_REF: &str = "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const OPERATION_REF: &str = "blake3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const REQUEST_REF: &str = "blake3:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const SESSION_REF: &str = "blake3:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
const STREAM_REF: &str = "blake3:1111111111111111111111111111111111111111111111111111111111111111";
const PEER_REF: &str = "blake3:2222222222222222222222222222222222222222222222222222222222222222";
const MEMBERSHIP_REF: &str = "blake3:3333333333333333333333333333333333333333333333333333333333333333";
const PRINCIPAL_REF: &str = "blake3:4444444444444444444444444444444444444444444444444444444444444444";
const TRUST_REF: &str = "blake3:5555555555555555555555555555555555555555555555555555555555555555";
const CAPABILITY_REF: &str = "blake3:6666666666666666666666666666666666666666666666666666666666666666";
const GENERATION: u64 = 1;
const STALE_GENERATION: u64 = 2;
const PROFILE_LIMIT: u64 = 16;
const FRAME_LIMIT: u64 = 4_096;
const DATAGRAM_LIMIT: u64 = 1_024;
const QUEUE_BYTE_LIMIT: u64 = 16_384;
const INFLIGHT_LIMIT: u64 = 8_192;
const DEADLINE_WINDOW: u64 = 64;
const INITIAL_TICK: u64 = 10;
const DEADLINE_TICK: u64 = INITIAL_TICK + DEADLINE_WINDOW;
const INITIAL_CREDIT: u64 = 1_024;
const LENGTH_PREFIX_BYTES: u64 = 4;
const PAYLOAD: &[u8] = b"bounded-live-iroh-frame";
const EXPECTED_EVENT_KINDS: usize = 3;

fn profile(kind: TransportAdapterKind) -> CanonicalTransportProfile {
    canonical_transport_profile(&TransportProfile {
        schema: TRANSPORT_PROFILE_SCHEMA.to_string(),
        profile_id: kind.as_str().to_string(),
        profile_ref: PROFILE_DECLARATION_REF.to_string(),
        adapter_kind: kind,
        capabilities: vec![
            TransportCapability::BidirectionalStreams,
            TransportCapability::UnidirectionalStreams,
            TransportCapability::Datagrams,
        ],
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
    .expect("canonical transport profile")
}

fn descriptor() -> ProtocolDescriptor {
    ProtocolDescriptor {
        schema: TRANSPORT_PROTOCOL_SCHEMA.to_string(),
        protocol_id: "echo-protocol".to_string(),
        version: "v1".to_string(),
        alpn: "molten/extension-echo/1".to_string(),
        extension_id: "echo-extension".to_string(),
        service_id: "echo-service".to_string(),
        generation: GENERATION,
        listener_limit: 1,
        requested_capabilities: vec![
            TransportCapability::BidirectionalStreams,
            TransportCapability::UnidirectionalStreams,
            TransportCapability::Datagrams,
        ],
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
        profile_ref: PROFILE_DECLARATION_REF.to_string(),
    }
}

fn id(reference: &str, generation: u64) -> ScopedTransportId {
    ScopedTransportId {
        opaque_ref: reference.to_string(),
        service_id: "echo-service".to_string(),
        generation,
    }
}

fn peer() -> PeerIdentityRefs {
    PeerIdentityRefs {
        transport_identity_ref: PEER_REF.to_string(),
        membership_ref: Some(MEMBERSHIP_REF.to_string()),
        application_principal_ref: Some(PRINCIPAL_REF.to_string()),
        trust_decision_ref: Some(TRUST_REF.to_string()),
        capability_authority_ref: Some(CAPABILITY_REF.to_string()),
        bootstrap_policy_ref: None,
    }
}

fn setup_commands() -> [TransportCommand; EXPECTED_EVENT_KINDS] {
    let descriptor = descriptor();
    [
        TransportCommand::Register {
            operation_id: OPERATION_REF.to_string(),
            descriptor: descriptor.clone(),
        },
        TransportCommand::OpenSession {
            operation_id: OPERATION_REF.to_string(),
            session_id: id(SESSION_REF, GENERATION),
            alpn: descriptor.alpn,
            direction: SessionDirection::Outbound,
            peer: peer(),
            observed_tick: INITIAL_TICK,
            deadline_tick: DEADLINE_TICK,
        },
        TransportCommand::OpenStream {
            operation_id: OPERATION_REF.to_string(),
            session_id: id(SESSION_REF, GENERATION),
            stream_id: id(STREAM_REF, GENERATION),
            direction: StreamDirection::Bidirectional,
            initial_credit_bytes: INITIAL_CREDIT,
        },
    ]
}

fn apply_setup<A: TransportCommandShell>(adapter: &mut A) -> Vec<TransportEventKind> {
    setup_commands()
        .iter()
        .map(|command| {
            adapter
                .execute_command(command)
                .expect("setup command")
                .events
                .first()
                .expect("canonical event")
                .event
                .kind
        })
        .collect()
}

// r[verify molten.fabric_transport.port_contract]
// r[verify molten.fabric_transport.evidence]
#[test]
fn canonical_profile_descriptor_and_readback_are_bounded_and_payload_free() {
    let profile = profile(TransportAdapterKind::DeterministicSimulation);
    let descriptor = fabric_transport_port_descriptor(&profile);
    assert_eq!(descriptor.class, crate::fabric::FabricPortClass::Transport);
    assert_eq!(descriptor.port_id, FABRIC_TRANSPORT_PORT_ID);
    assert!(descriptor.authority_requirements.contains(&crate::fabric::FabricAuthority::ProtocolOwnership));
    let profile_text = crate::preserves_rail::to_text(&profile.value).expect("profile text");
    assert!(profile_text.contains("does-not-prove-durable-delivery"));
    assert!(!profile_text.contains("iroh::Connection"));

    let mut adapter = DeterministicTransportAdapter::new(profile).expect("simulated adapter");
    let _events = apply_setup(&mut adapter);
    let status = adapter.status().expect("transport status");
    let status_text = crate::preserves_rail::to_text(&status.value).expect("status text");
    assert_eq!(status.active_protocols, 1);
    assert_eq!(status.active_sessions, 1);
    assert_eq!(status.active_streams, 1);
    assert!(!status_text.contains("bounded-live-iroh-frame"));
    assert!(!status_text.contains("private-key"));
}

// r[verify molten.fabric_transport.live_sim_parity]
// r[verify molten.fabric_transport.final_validation]
#[test]
fn live_and_simulated_shells_emit_the_same_adapter_neutral_setup_trace() {
    let mut live = IrohTransportAdapter::new(profile(TransportAdapterKind::IrohLive)).expect("Iroh adapter");
    let mut simulated = DeterministicTransportAdapter::new(profile(TransportAdapterKind::DeterministicSimulation))
        .expect("simulated adapter");
    let live_trace = apply_setup(&mut live);
    let simulated_trace = apply_setup(&mut simulated);
    assert_eq!(live_trace, simulated_trace);
    assert_eq!(live_trace, vec![
        TransportEventKind::ProtocolRegistered,
        TransportEventKind::SessionEstablished,
        TransportEventKind::StreamOpened,
    ]);
}

// r[verify molten.fabric_transport.live_sim_parity]
// r[verify molten.fabric_transport.final_validation]
#[test]
fn shared_adapter_conformance_covers_flow_control_datagrams_cancellation_drain_and_cleanup() {
    fn run<A: TransportCommandShell>(adapter: &mut A) -> (Vec<TransportEventKind>, TransportState) {
        let _setup = apply_setup(adapter);
        let payload_ref = format!("blake3:{}", blake3::hash(PAYLOAD).to_hex());
        let payload_bytes = u64::try_from(PAYLOAD.len()).expect("payload length");
        let descriptor = descriptor();
        let commands = [
            TransportCommand::SendFrame {
                operation_id: OPERATION_REF.to_string(),
                session_id: id(SESSION_REF, GENERATION),
                stream_id: id(STREAM_REF, GENERATION),
                payload_ref: payload_ref.clone(),
                payload_bytes,
                observed_tick: INITIAL_TICK,
            },
            TransportCommand::AcknowledgeFrame {
                operation_id: OPERATION_REF.to_string(),
                session_id: id(SESSION_REF, GENERATION),
                stream_id: id(STREAM_REF, GENERATION),
                payload_bytes,
            },
            TransportCommand::SendDatagram {
                operation_id: OPERATION_REF.to_string(),
                session_id: id(SESSION_REF, GENERATION),
                payload_ref,
                payload_bytes,
                observed_tick: INITIAL_TICK,
            },
            TransportCommand::CompleteDatagram {
                operation_id: OPERATION_REF.to_string(),
                session_id: id(SESSION_REF, GENERATION),
                payload_bytes,
                delivered: true,
            },
            TransportCommand::Cancel {
                operation_id: OPERATION_REF.to_string(),
                target: CancelTarget::Stream {
                    session_id: id(SESSION_REF, GENERATION),
                    stream_id: id(STREAM_REF, GENERATION),
                },
            },
            TransportCommand::BeginDrain {
                operation_id: OPERATION_REF.to_string(),
                alpn: descriptor.alpn.clone(),
                service_id: descriptor.service_id.clone(),
                generation: GENERATION,
            },
            TransportCommand::CloseSession {
                operation_id: OPERATION_REF.to_string(),
                session_id: id(SESSION_REF, GENERATION),
            },
            TransportCommand::CleanupListener {
                operation_id: OPERATION_REF.to_string(),
                alpn: descriptor.alpn,
                service_id: descriptor.service_id,
                generation: GENERATION,
                cleanup_evidence_ref: REQUEST_REF.to_string(),
            },
        ];
        let mut kinds = Vec::new();
        let mut state = TransportState::default();
        for command in commands {
            let transition = adapter.execute_command(&command).expect("shared conformance command");
            kinds.push(transition.events[0].event.kind);
            state = transition.state;
        }
        (kinds, state)
    }

    let mut live = IrohTransportAdapter::new(profile(TransportAdapterKind::IrohLive)).expect("Iroh adapter");
    let mut simulated = DeterministicTransportAdapter::new(profile(TransportAdapterKind::DeterministicSimulation))
        .expect("simulated adapter");
    let live_result = run(&mut live);
    let simulated_result = run(&mut simulated);
    assert_eq!(live_result, simulated_result);
    assert!(live_result.1.protocols.is_empty());
    assert_eq!(live_result.1.counters.cancellations, 1);
}

// r[verify molten.fabric_transport.live_sim_parity]
// r[verify molten.fabric_transport.final_validation]
#[tokio::test]
async fn live_iroh_loopback_exchanges_a_bounded_frame_without_leaking_adapter_handles() {
    let mut adapter = IrohTransportAdapter::new(profile(TransportAdapterKind::IrohLive)).expect("Iroh adapter");
    let _events = apply_setup(&mut adapter);
    let result = adapter
        .live_loopback_frame(
            &id(SESSION_REF, GENERATION),
            &id(STREAM_REF, GENERATION),
            OPERATION_REF,
            &descriptor().alpn,
            PAYLOAD,
            INITIAL_TICK,
        )
        .await
        .expect("live Iroh loopback");
    let expected_ref = format!("blake3:{}", blake3::hash(PAYLOAD).to_hex());
    assert_eq!(result.echoed_payload_ref, expected_ref);
    assert!(result.remote_transport_identity_ref.starts_with("blake3:"));
    assert_eq!(result.submitted.events[0].event.kind, TransportEventKind::FrameSubmitted);
    assert_eq!(result.acknowledged.events[0].event.kind, TransportEventKind::FrameAcknowledged);
    assert_eq!(adapter.state().sessions[SESSION_REF].inflight_bytes, 0);
}

// r[verify molten.fabric_transport.failure_semantics]
// r[verify molten.fabric_transport.final_validation]
#[test]
fn deterministic_partition_after_submission_reports_uncertainty_without_retry() {
    let mut adapter = DeterministicTransportAdapter::new(profile(TransportAdapterKind::DeterministicSimulation))
        .expect("simulated adapter");
    let _events = apply_setup(&mut adapter);
    let payload_ref = format!("blake3:{}", blake3::hash(PAYLOAD).to_hex());
    let send = TransportCommand::SendFrame {
        operation_id: OPERATION_REF.to_string(),
        session_id: id(SESSION_REF, GENERATION),
        stream_id: id(STREAM_REF, GENERATION),
        payload_ref,
        payload_bytes: u64::try_from(PAYLOAD.len()).expect("payload length"),
        observed_tick: INITIAL_TICK,
    };
    let submitted = adapter.execute_command(&send).expect("submit frame");
    assert_eq!(submitted.events[0].event.delivery, DeliveryOutcome::Pending);
    let failed = adapter
        .execute_with_fault(&send, Some(SimulatedTransportFault::Partition))
        .expect("partition evidence");
    assert_eq!(failed.events[0].event.delivery, DeliveryOutcome::Uncertain);
    assert_eq!(failed.events[0].event.retry, RetryDisposition::UnsafeWithoutReconciliation);
    assert_eq!(failed.state.counters.failures, 1);
}

// r[verify molten.fabric_transport.session_streams]
// r[verify molten.fabric_transport.protocol_registration]
#[test]
fn registered_effect_port_routes_only_exact_profile_generation_and_known_request() {
    use crate::fabric::FabricPortRequirement;
    use crate::fabric::resolve_canonical_fabric_port_binding;
    use crate::system_extension::EffectTarget;
    use crate::system_extension::FabricEffectPort;
    use crate::system_extension::TypedEffectRequest;

    let profile = profile(TransportAdapterKind::DeterministicSimulation);
    let descriptor = fabric_transport_port_descriptor(&profile);
    let binding = resolve_canonical_fabric_port_binding(std::slice::from_ref(&descriptor), &FabricPortRequirement {
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
    .expect("transport binding");
    let context = ExtensionTransportContext::from_test_snapshot("echo-service", GENERATION, &profile);
    let adapter = DeterministicTransportAdapter::new(profile.clone()).expect("simulated adapter");
    let mut port = RegisteredTransportEffectPort::new(adapter, context, profile).expect("registered port");
    port.register(REQUEST_REF.to_string(), setup_commands()[0].clone())
        .expect("register transport request");
    let effect = TypedEffectRequest {
        target: EffectTarget::FabricPort(binding.binding.key.clone()),
        operation: "register-protocol".to_string(),
        input_schema_ref: TRANSPORT_COMMAND_SCHEMA.to_string(),
        output_schema_ref: TRANSPORT_EVENT_SCHEMA.to_string(),
        request_ref: REQUEST_REF.to_string(),
        generation: GENERATION,
        accounted_bytes: 0,
    };
    let output = port.route(&binding, &effect).expect("route transport effect");
    assert!(output.output_ref.starts_with("blake3:"));
    assert_eq!(port.adapter().state().protocols.len(), 1);

    let mut stale = effect.clone();
    stale.generation = STALE_GENERATION;
    assert!(port.route(&binding, &stale).is_err());
    let mut unknown = effect;
    unknown.request_ref = OPERATION_REF.to_string();
    assert!(port.route(&binding, &unknown).is_err());
}

// r[verify molten.fabric_transport.flow_control]
// r[verify molten.fabric_transport.identity_separation]
#[test]
fn extension_context_and_outer_frame_validation_fail_closed() {
    let profile = profile(TransportAdapterKind::DeterministicSimulation);
    let context = ExtensionTransportContext::from_test_snapshot("echo-service", GENERATION, &profile);
    let command = setup_commands()[0].clone();
    assert!(context.admit_command(&profile, &command, 0).is_ok());

    let stale = TransportCommand::Register {
        operation_id: OPERATION_REF.to_string(),
        descriptor: ProtocolDescriptor {
            generation: STALE_GENERATION,
            ..descriptor()
        },
    };
    assert!(context.admit_command(&profile, &stale, 0).is_err());

    let declared_ref = format!("blake3:{}", blake3::hash(PAYLOAD).to_hex());
    let wrong_ref = OPERATION_REF;
    let issues = validate_outer_frame(
        &profile.profile,
        &declared_ref,
        wrong_ref,
        u64::try_from(PAYLOAD.len()).expect("payload length"),
    )
    .expect_err("mismatched frame identity must deny");
    assert!(issues.contains(&TransportIssue::PayloadRefMismatch));
}
