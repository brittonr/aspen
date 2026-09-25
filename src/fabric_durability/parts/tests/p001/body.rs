
// r[verify molten.fabric_durability.port_contracts]
// r[verify molten.fabric_durability.final_validation]
#[test]
fn registered_effect_port_routes_only_known_commands_to_the_exact_bound_profile() {
    use crate::system_extension::FabricEffectPort;

    let simulation = profile(DurableAdapterKind::DeterministicSimulation);
    let port_descriptor = fabric_durability_port_descriptors(&simulation)
        .into_iter()
        .find(|descriptor| descriptor.port_id == FABRIC_DURABLE_LOG_PORT_ID)
        .expect("durable log descriptor");
    let binding = crate::fabric::resolve_canonical_fabric_port_binding(
        std::slice::from_ref(&port_descriptor),
        &crate::fabric::FabricPortRequirement {
            port_id: port_descriptor.port_id.clone(),
            version: port_descriptor.version.clone(),
            class: port_descriptor.class,
            operation_classes: port_descriptor.operation_classes.clone(),
            input_schema_refs: port_descriptor.input_schema_refs.clone(),
            output_schema_refs: port_descriptor.output_schema_refs.clone(),
            allowed_authorities: port_descriptor.authority_requirements.clone(),
            available_resources: port_descriptor.resource_requirements.clone(),
            expected_determinism: port_descriptor.determinism,
            expected_replay: port_descriptor.replay,
            expected_profile: port_descriptor.implementation_profile.clone(),
        },
    )
    .expect("canonical durability binding");
    let adapter = SimulatedDurableStateAdapter::new(simulation, descriptor()).expect("simulation adapter");
    let mut port = RegisteredDurableEffectPort::new(adapter);
    port.register(OPERATION_REF.to_string(), DurablePortCommand::Append(append_request(DurabilityLevel::ProcessLoss)))
        .expect("register durable request");
    let effect = crate::system_extension::TypedEffectRequest {
        target: crate::system_extension::EffectTarget::FabricPort(binding.binding.key.clone()),
        operation: "append".to_string(),
        input_schema_ref: DURABLE_STATE_OPERATION_SCHEMA.to_string(),
        output_schema_ref: DURABLE_STATE_OUTCOME_SCHEMA.to_string(),
        request_ref: OPERATION_REF.to_string(),
        generation: GENERATION,
        accounted_bytes: 1,
    };
    let output = port.route(&binding, &effect).expect("route durable effect");
    assert!(output.output_ref.starts_with("blake3:"));
    assert_eq!(port.adapter().state().durable_log.len(), 1);

    let mut unknown = effect;
    unknown.request_ref = VALUE_REF.to_string();
    assert!(port.route(&binding, &unknown).is_err());
}

// r[verify molten.fabric_durability.port_contracts]
#[test]
fn extension_context_denies_stale_generation_unbound_port_and_profile_substitution() {
    let simulation = profile(DurableAdapterKind::DeterministicSimulation);
    let context = ExtensionDurabilityContext::from_test_snapshot("service-a", GENERATION, &simulation, vec![
        FABRIC_DURABLE_LOG_PORT_ID.to_string(),
    ]);
    assert!(context.admit_operation(&simulation, FABRIC_DURABLE_LOG_PORT_ID, "service-a", GENERATION, 1).is_ok());
    assert!(
        context
            .admit_operation(&simulation, FABRIC_DURABLE_LOG_PORT_ID, "service-a", STALE_GENERATION, 1)
            .is_err()
    );
    assert!(
        context
            .admit_operation(&simulation, FABRIC_ORDERED_STORE_PORT_ID, "service-a", GENERATION, 1)
            .is_err()
    );
    let live = profile(DurableAdapterKind::LiveRedb);
    assert!(context.admit_operation(&live, FABRIC_DURABLE_LOG_PORT_ID, "service-a", GENERATION, 1).is_err());
}
