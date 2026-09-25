
// r[verify molten.fabric_transport.live_sim_parity]
// r[verify molten.fabric_transport.final_validation]
#[tokio::test]
async fn live_iroh_loopback_exchanges_a_bounded_frame_without_leaking_adapter_handles() {
    let mut adapter = IrohTransportAdapter::new(profile(TransportAdapterKind::IrohLive)).expect("Iroh adapter");
    let _events = apply_setup(&mut adapter);
    let result = adapter
        .live_loopback_frame(LoopbackFrameInput {
            session_id: &id(SESSION_REF, GENERATION),
            stream_id: &id(STREAM_REF, GENERATION),
            operation_id: OPERATION_REF,
            alpn: &descriptor().alpn,
            payload: PAYLOAD,
            observed_tick: INITIAL_TICK,
        })
        .await
        .expect("live Iroh loopback");
    let expected_ref = format!("blake3:{}", blake3::hash(PAYLOAD).to_hex());
    assert_eq!(result.echoed_payload_ref, expected_ref);
    assert!(result.remote_transport_identity_ref.starts_with("blake3:"));
    assert_eq!(result.submitted.events[0].event.kind, TransportEventKind::FrameSubmitted);
    assert_eq!(result.acknowledged.events[0].event.kind, TransportEventKind::FrameAcknowledged);
    assert_eq!(adapter.state().sessions[SESSION_REF].inflight_bytes, 0);
}

// r[verify molten.modularity.fabric_boundary.adapters.transport_error]
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

// r[verify molten.modularity.fabric_boundary.validation]
// r[verify molten.fabric_transport.session_streams]
// r[verify molten.fabric_transport.protocol_registration]
#[test]
fn registered_effect_port_routes_only_exact_profile_generation_and_known_request() {
    use crate::system_extension::FabricEffectPort;

    let profile = profile(TransportAdapterKind::DeterministicSimulation);
    let descriptor = fabric_transport_port_descriptor(&profile);
    let binding = crate::fabric::resolve_canonical_fabric_port_binding(
        std::slice::from_ref(&descriptor),
        &crate::fabric::FabricPortRequirement {
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
        },
    )
    .expect("transport binding");
    let context = ExtensionTransportContext::from_test_snapshot("echo-service", GENERATION, &profile);
    let adapter = DeterministicTransportAdapter::new(profile.clone()).expect("simulated adapter");
    let mut port = RegisteredTransportEffectPort::new(adapter, context, profile).expect("registered port");
    port.register(REQUEST_REF.to_string(), setup_commands()[0].clone())
        .expect("register transport request");
    let effect = crate::system_extension::TypedEffectRequest {
        target: crate::system_extension::EffectTarget::FabricPort(binding.binding.key.clone()),
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
