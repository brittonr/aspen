
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
    let binding = crate::fabric::resolve_canonical_port_binding(
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
    let context = ExtensionContext::from_test_snapshot("service-a", GENERATION, &simulation, vec![
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

fn persisted_legacy_log(root: &std::path::Path, values: &[&[u8]]) {
    let mut adapter = RedbDurableStateAdapter::open(root, profile(DurableAdapterKind::LiveRedb), descriptor())
        .expect("open legacy Redb log");
    for value in values {
        let mut request = append_request(DurabilityLevel::MachineLoss);
        request.expected_sequence = adapter.state().next_log_sequence().expect("next persisted sequence");
        request.value = value.to_vec();
        adapter.append(&request).expect("persist legacy log record");
    }
}

fn current_restore_profile(max_log_records: u64, max_operation_bytes: u64, max_namespace_bytes: u64) -> DurableProfile {
    let mut current = profile(DurableAdapterKind::LiveRedb).profile;
    current.max_log_records = max_log_records;
    current.max_operation_bytes = max_operation_bytes;
    current.max_namespace_bytes = max_namespace_bytes;
    durable_profile(&current).expect("current canonical restore profile")
}

// r[verify molten.fabric_durability.redb_restore_profile_admission]
#[test]
fn redb_restore_preflights_persisted_record_count() {
    let root = crate::test_support::process_workspace("fabric-durability-redb-restore-record-count").expect("workspace");
    let exact_root = root.join("exact");
    let excess_root = root.join("excess");
    persisted_legacy_log(&exact_root, &[b"first".as_slice(), b"second".as_slice()]);
    persisted_legacy_log(&excess_root, &[b"first".as_slice(), b"second".as_slice(), b"third".as_slice()]);

    let current = current_restore_profile(2, OPERATION_BYTE_LIMIT, NAMESPACE_BYTE_LIMIT);
    let exact = RedbDurableStateAdapter::open(&exact_root, current.clone(), descriptor())
        .expect("exactly the current profile log-record limit must restore");
    assert_eq!(exact.state().durable_log.len(), 2);
    assert_eq!(exact.state().durable_log[1].value, b"second");
    drop(exact);

    let error = RedbDurableStateAdapter::open(&excess_root, current, descriptor())
        .expect_err("one excess persisted record must deny before record loading");
    assert!(
        error.to_string().contains("OperationLimitExceeded { actual: 3, maximum: 2 }"),
        "expected current profile log-record denial, got {error}"
    );
    let legacy = RedbDurableStateAdapter::open(&excess_root, profile(DurableAdapterKind::LiveRedb), descriptor())
        .expect("denied current-profile restore must leave the legacy Redb log unchanged");
    assert_eq!(legacy.state().durable_log.len(), 3);
}

// r[verify molten.fabric_durability.redb_restore_profile_admission]
#[test]
fn redb_restore_preflights_persisted_record_bytes() {
    let root = crate::test_support::process_workspace("fabric-durability-redb-restore-record-bytes").expect("workspace");
    let exact_root = root.join("exact");
    let excess_root = root.join("excess");
    persisted_legacy_log(&exact_root, &[b"12345678".as_slice()]);
    persisted_legacy_log(&excess_root, &[b"123456789".as_slice()]);

    let current = current_restore_profile(PROFILE_LIMIT, 8, NAMESPACE_BYTE_LIMIT);
    let exact = RedbDurableStateAdapter::open(&exact_root, current.clone(), descriptor())
        .expect("exactly the current operation byte limit must restore");
    assert_eq!(exact.state().durable_log[0].value, b"12345678");
    let error = RedbDurableStateAdapter::open(&excess_root, current, descriptor())
        .expect_err("one excess persisted value byte must deny before cloning the payload");
    assert!(
        error.to_string().contains("ByteLimitExceeded { actual: 9, maximum: 8 }"),
        "expected current profile operation byte denial, got {error}"
    );

    let quota_exact_root = root.join("quota-exact");
    let quota_excess_root = root.join("quota-excess");
    persisted_legacy_log(&quota_exact_root, &[b"12345678".as_slice(), b"abcdefgh".as_slice()]);
    persisted_legacy_log(&quota_excess_root, &[b"12345678".as_slice(), b"abcdefghi".as_slice()]);
    let mut bounded_descriptor = descriptor();
    bounded_descriptor.atomicity_domain.max_bytes = 16;
    bounded_descriptor.quota_bytes = NAMESPACE_BYTE_LIMIT;
    let current_quota = current_restore_profile(PROFILE_LIMIT, OPERATION_BYTE_LIMIT, 16);
    let exact_quota = RedbDurableStateAdapter::open(&quota_exact_root, current_quota.clone(), bounded_descriptor.clone())
        .expect("exactly the current namespace byte limit must restore");
    assert_eq!(exact_quota.state().durable_bytes, 16);
    drop(exact_quota);
    let quota_error = RedbDurableStateAdapter::open(&quota_excess_root, current_quota, bounded_descriptor)
        .expect_err("one excess cumulative payload byte must deny before cloning the second value");
    assert!(
        quota_error.to_string().contains("NamespaceQuotaExceeded { actual: 17, maximum: 16 }"),
        "expected current namespace byte denial, got {quota_error}"
    );

    let mut descriptor_quota = descriptor();
    descriptor_quota.quota_bytes = 16;
    let current_descriptor_quota = current_restore_profile(PROFILE_LIMIT, OPERATION_BYTE_LIMIT, NAMESPACE_BYTE_LIMIT);
    let exact_descriptor =
        RedbDurableStateAdapter::open(&quota_exact_root, current_descriptor_quota.clone(), descriptor_quota.clone())
            .expect("exactly the descriptor quota must restore under a wider current profile");
    assert_eq!(exact_descriptor.state().durable_bytes, 16);
    drop(exact_descriptor);
    let descriptor_error = RedbDurableStateAdapter::open(&quota_excess_root, current_descriptor_quota, descriptor_quota)
        .expect_err("one byte above the descriptor quota must deny under a wider current profile");
    assert!(
        descriptor_error.to_string().contains("NamespaceQuotaExceeded { actual: 17, maximum: 16 }"),
        "expected current descriptor quota denial, got {descriptor_error}"
    );
}
