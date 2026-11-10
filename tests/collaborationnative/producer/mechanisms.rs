pub(super) mod durability;

pub(super) struct Profiles {
    pub execution: molten::fabric_execution::ExecutionProfileDescriptor,
    pub native: molten::system_extension::NativeHostProfile,
    pub resources: molten::fabric_execution::ExecutionResourceGrant,
    pub resource_ref: String,
    pub policy_ref: String,
    pub grant_ref: String,
}

pub(super) fn build(source: &super::Source) -> super::Result<Profiles> {
    let policy_ref = super::record(&source.root, "collaboration-native-policy-v1", &[
        &source.validation.selected_output_validation_ref,
        "independent-current-host-and-scope-authority-required",
    ])?;
    let execution = execution_profile(source)?;
    let native = native_profile(source, &execution)?;
    let (resources, resource_ref) = resource_grant(source)?;
    let grant_ref = super::record(&source.root, "collaboration-native-execution-request-scope-v1", &[
        &source.artifact_ref,
        &policy_ref,
        &resource_ref,
        &execution.profile_ref,
        "declaration-not-current-operator-authorization",
    ])?;
    Ok(Profiles {
        execution,
        native,
        resources,
        resource_ref,
        policy_ref,
        grant_ref,
    })
}

/// The admitted live bounded-process execution profile, retained as its admitted value.
fn execution_profile(source: &super::Source) -> super::Result<molten::fabric_execution::ExecutionProfileDescriptor> {
    let profile_ref = super::record(&source.root, "collaboration-native-execution-profile-v1", &[
        "live-bounded-process",
        "unix-process-group",
        "60000",
        "8388608",
        "16",
        "64",
        molten::fabric_execution::BOUNDED_EXEC_REVISION,
        &source.validation.selected_output_validation_ref,
    ])?;
    let execution = molten::fabric_execution::ExecutionProfileDescriptor {
        schema: molten::fabric_execution::EXECUTION_PROFILE_SCHEMA.into(),
        profile_id: "collaboration-native-live".into(),
        profile_ref,
        kind: molten::fabric_execution::ExecutionProfileKind::LiveBoundedProcess,
        platform: molten::fabric_execution::ExecutionPlatform::UnixProcessGroup,
        supported_termination_scopes: vec![molten::fabric_execution::ExecutionTerminationScope::ProcessGroup],
        max_timeout_ms: 60_000,
        max_stdin_bytes: 8_388_608,
        max_stdout_bytes: 8_388_608,
        max_stderr_bytes: 8_388_608,
        max_poll_interval_ms: 5,
        max_teardown_timeout_ms: 5_000,
        max_arguments: 16,
        max_argument_bytes: 4096,
        max_environment_entries: 16,
        max_environment_name_bytes: 128,
        max_environment_value_bytes: 4096,
        max_concurrency_units: 1,
        max_queue_units: 64,
        component_repository: molten::fabric_execution::BOUNDED_EXEC_REPOSITORY.into(),
        component_revision: molten::fabric_execution::BOUNDED_EXEC_REVISION.into(),
        component_license: molten::fabric_execution::BOUNDED_EXEC_LICENSE.into(),
        component_package: molten::fabric_execution::BOUNDED_EXEC_PACKAGE.into(),
        conformance_refs: vec![source.validation.selected_output_validation_ref.clone()],
        fabric_non_claims: molten::fabric::REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
        non_claims: molten::fabric_execution::REQUIRED_EXECUTION_NON_CLAIMS.to_vec(),
    };
    let admitted = molten::fabric_execution::admit_profile(&execution)?;
    super::retain(&source.root, &admitted.value)?;
    Ok(execution)
}

/// The native host profile that admits exactly that execution profile and callback transport.
fn native_profile(
    source: &super::Source,
    execution: &molten::fabric_execution::ExecutionProfileDescriptor,
) -> super::Result<molten::system_extension::NativeHostProfile> {
    let transport_ref = super::record(&source.root, "native-local-callback-transport-v1", &[
        molten::system_extension::NATIVE_ALPN,
        molten::system_extension::NATIVE_FRAMING,
    ])?;
    let native_ref = super::record(&source.root, "collaboration-native-host-profile-v1", &[
        &execution.profile_ref,
        &transport_ref,
        "8388608",
        "8388608",
        "25165824",
        "1048576",
        "1",
        "128",
        "16",
    ])?;
    Ok(molten::system_extension::NativeHostProfile {
        schema: molten::system_extension::NATIVE_HOST_PROFILE_SCHEMA.into(),
        profile_id: "collaboration-native-host".into(),
        profile_ref: native_ref,
        execution_profile_ref: execution.profile_ref.clone(),
        transport_profile_ref: transport_ref,
        alpn: molten::system_extension::NATIVE_ALPN.into(),
        framing: molten::system_extension::NATIVE_FRAMING.into(),
        max_callback_input_bytes: 8_388_608,
        max_callback_output_bytes: 8_388_608,
        max_diagnostic_bytes: 25_165_824,
        max_materialized_value_bytes: 1_048_576,
        max_instances: 1,
        max_unresolved_operations: 128,
        max_port_bindings: 16,
        max_policy_refs: 16,
        max_materialized_values: 128,
        is_local_live_pilot: true,
        requires_materialized_values: true,
        non_claims: molten::system_extension::REQUIRED_NATIVE_HOST_NON_CLAIMS.to_vec(),
    })
}

/// The bounded execution resource grant and its retained identity.
fn resource_grant(source: &super::Source) -> super::Result<(molten::fabric_execution::ExecutionResourceGrant, String)> {
    let resources = molten::fabric_execution::ExecutionResourceGrant {
        memory_bytes: 268_435_456,
        storage_bytes: 67_108_864,
        diagnostic_bytes: 25_165_824,
        logical_deadline_ticks: 600_000,
        concurrency_units: 1,
        queue_units: 64,
    };
    let resource_ref = super::record(&source.root, "collaboration-resource-envelope-v1", &[
        "268435456",
        "67108864",
        "8388608",
        "600000",
        "1",
        "64",
    ])?;
    Ok((resources, resource_ref))
}

pub(super) fn experimental(
    namespace: &str,
) -> super::Result<molten::live_reliability::service::profiles::Experimental> {
    Ok(molten::live_reliability::service::profiles::experimental(
        molten::live_reliability::service::profiles::ExperimentalInput {
            service_id: namespace.into(),
            extension_id: molten::collaboration_scope::EXTENSION_ID.into(),
            protocol_id: "collaboration-raft".into(),
            adapter_id: namespace.into(),
            namespace_id: namespace.into(),
            registration_authority_ref: super::reference("collaboration-protocol-registration-request"),
            value_schema_ref: super::reference(molten::collaboration_scope::SCHEMA),
            operation_deadline_ticks: 100,
            max_duration_ticks: 100_000,
        },
    )?)
}

/// Re-author all transport identities before endpoint, manifest and runtime admission.
pub(super) fn bounded_transport(
    namespace: &str,
    maximum: Option<u64>,
) -> super::Result<molten::live_reliability::service::profiles::Experimental> {
    let mut mechanisms = durability::buffered(namespace)?;
    if let Some(maximum) = maximum {
        mechanisms.transport.limits.max_frame_bytes = maximum;
        mechanisms.transport.profile_ref = definition_ref(&mechanisms.transport)?;
        mechanisms.protocol.profile_ref = mechanisms.transport.profile_ref.clone();
        mechanisms.protocol.framing.max_frame_bytes = maximum;
        mechanisms.protocol.framing.profile_ref = definition_ref(&mechanisms.protocol.framing)?;
        let profile = mechanisms.transport.clone().into_profile()?;
        mechanisms.protocol.clone().into_descriptor(&profile)?;
    }
    Ok(mechanisms)
}

fn definition_ref(value: &impl serde::Serialize) -> super::Result<String> {
    let mut definition = serde_json::to_value(value)?;
    definition.as_object_mut().ok_or("mechanism definition object")?.remove("profile_ref");
    let projected = molten::live_reliability::checker::canonical_value(&definition)?;
    Ok(molten::preserves_rail::canonical_hash(&molten::preserves_rail::record(
        "molten.experimental.mechanism-definition.v1",
        vec![projected],
    ))?)
}

/// The fabric port descriptors the declared mechanisms own, bounded to the admitted set.
pub(super) fn descriptors(
    mechanisms: &molten::live_reliability::service::profiles::Experimental,
) -> super::Result<Vec<molten::fabric::FabricPortDescriptor>> {
    let transport = mechanisms.transport.clone().into_profile()?;
    let durable = mechanisms.durable.clone().into_profile()?;
    let time = mechanisms.time.clone().into_descriptor()?;
    let mut ports = vec![molten::fabric_transport::fabric_transport_port_descriptor(&transport)];
    ports.extend(molten::fabric_durability::fabric_durability_port_descriptors(&durable).into_iter().filter(|port| {
        [
            molten::fabric_durability::FABRIC_DURABLE_LOG_PORT_ID,
            molten::fabric_durability::FABRIC_SNAPSHOT_PORT_ID,
        ]
        .contains(&port.port_id.as_str())
    }));
    ports.extend(molten::fabric_time::port_descriptors(&time).into_iter().filter(|port| {
        [
            molten::fabric_time::FABRIC_TIMER_PORT_ID,
            molten::fabric_time::FABRIC_ENTROPY_PORT_ID,
        ]
        .contains(&port.port_id.as_str())
    }));
    for port in [
        molten::fabric_membership::FABRIC_MEMBERSHIP_PORT_ID,
        molten::fabric_membership::FABRIC_PLACEMENT_PORT_ID,
    ] {
        ports.push(descriptor(
            port,
            molten::fabric::FabricPortClass::Membership,
            "observe",
            "collaboration-static-membership",
        ));
    }
    let mut commit = descriptor(
        molten::collaboration_scope::COMMIT_PORT_ID,
        molten::fabric::FabricPortClass::Consistency,
        molten::collaboration_scope::COMMIT_OPERATION,
        "collaboration-native-commit",
    );
    commit.input_schema_refs = vec![molten::collaboration_scope::COMMIT_INPUT_SCHEMA.into()];
    commit.output_schema_refs = vec![molten::collaboration_scope::COMMIT_OUTPUT_SCHEMA.into()];
    ports.push(commit);
    ports.push(descriptor(
        "molten.collaboration.authority",
        molten::fabric::FabricPortClass::Authority,
        "authorize",
        "collaboration-current-operator",
    ));
    ports.push(descriptor(
        "molten.collaboration.resources",
        molten::fabric::FabricPortClass::Resources,
        "account",
        "collaboration-bounded-resources",
    ));
    ports.push(descriptor(
        "molten.collaboration.evidence",
        molten::fabric::FabricPortClass::Evidence,
        "record",
        "collaboration-canonical-evidence",
    ));
    Ok(ports)
}

fn descriptor(
    id: &str,
    class: molten::fabric::FabricPortClass,
    operation: &str,
    profile: &str,
) -> molten::fabric::FabricPortDescriptor {
    molten::fabric::FabricPortDescriptor {
        schema: molten::fabric::FABRIC_PORT_DESCRIPTOR_SCHEMA.into(),
        port_id: id.into(),
        version: "v1".into(),
        class,
        operation_classes: vec![operation.into()],
        input_schema_refs: vec![format!("{id}.input.v1")],
        output_schema_refs: vec![format!("{id}.output.v1")],
        authority_requirements: Vec::new(),
        resource_requirements: Vec::new(),
        determinism: molten::fabric::DeterminismClass::ExternalEffect,
        replay: molten::fabric::ReplayClass::RecordedEffectRequired,
        implementation_profile: profile.into(),
        conformance_refs: vec![super::reference(
            "collaboration-native-declared-contract-not-observed-execution",
        )],
        non_claims: molten::fabric::REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
        enabled: true,
    }
}

pub(super) fn requirement(port: &molten::fabric::FabricPortDescriptor) -> molten::fabric::FabricPortRequirement {
    molten::fabric::FabricPortRequirement {
        port_id: port.port_id.clone(),
        version: port.version.clone(),
        class: port.class,
        operation_classes: port.operation_classes.clone(),
        input_schema_refs: port.input_schema_refs.clone(),
        output_schema_refs: port.output_schema_refs.clone(),
        allowed_authorities: port.authority_requirements.clone(),
        available_resources: port.resource_requirements.clone(),
        expected_determinism: port.determinism,
        expected_replay: port.replay,
        expected_profile: port.implementation_profile.clone(),
    }
}
