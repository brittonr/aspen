pub(in crate::authoring) struct Selected {
    pub execution: molten::fabric_execution::ExecutionProfileDescriptor,
    pub admitted: molten::fabric_execution::Profile,
    pub native: molten::system_extension::NativeHostProfile,
    pub resources: molten::fabric_execution::ExecutionResourceGrant,
    pub resource_ref: String,
}

pub(super) fn build(source: &super::super::source::Prepared) -> molten::error::Result<Selected> {
    let execution = execution_descriptor(source, execution_declaration(source)?);
    let admitted = molten::fabric_execution::admit_profile(&execution)?;
    super::super::source::retain(&source.root, &admitted.value)?;
    let transport_ref =
        super::super::source::record(&source.root, "native-materialized-local-transport-profile-v2", vec![
            molten::preserves_rail::string(molten::system_extension::NATIVE_ALPN),
            molten::preserves_rail::string(molten::system_extension::NATIVE_FRAMING),
            molten::preserves_rail::string("local-callback-standard-input-output-not-network-availability"),
        ])?;
    let native_ref = native_declaration(source, &admitted.profile_ref, &transport_ref)?;
    let native = molten::system_extension::NativeHostProfile {
        schema: molten::system_extension::NATIVE_HOST_PROFILE_SCHEMA.to_string(),
        profile_id: "native-host-local-pilot-v2".to_string(),
        profile_ref: native_ref,
        execution_profile_ref: admitted.profile_ref.clone(),
        transport_profile_ref: transport_ref,
        alpn: molten::system_extension::NATIVE_ALPN.to_string(),
        framing: molten::system_extension::NATIVE_FRAMING.to_string(),
        max_callback_input_bytes: 1_048_576,
        max_callback_output_bytes: 1_048_576,
        max_diagnostic_bytes: 1_048_576,
        max_materialized_value_bytes: 262_144,
        max_instances: 3,
        max_unresolved_operations: 128,
        max_port_bindings: 16,
        max_policy_refs: 16,
        max_materialized_values: 128,
        is_local_live_pilot: true,
        requires_materialized_values: true,
        non_claims: molten::system_extension::REQUIRED_NATIVE_HOST_NON_CLAIMS.to_vec(),
    };
    let resources = molten::fabric_execution::ExecutionResourceGrant {
        memory_bytes: 2_097_152,
        storage_bytes: 1_048_576,
        diagnostic_bytes: 2_097_152,
        logical_deadline_ticks: 10_000,
        concurrency_units: 1,
        queue_units: 64,
    };
    let resource_ref = super::super::source::record(&source.root, "experimental-native-resource-envelope-v1", vec![
        molten::preserves_rail::u64_value(resources.memory_bytes),
        molten::preserves_rail::u64_value(resources.storage_bytes),
        molten::preserves_rail::u64_value(resources.diagnostic_bytes),
        molten::preserves_rail::u64_value(resources.logical_deadline_ticks),
        molten::preserves_rail::u64_value(resources.concurrency_units),
        molten::preserves_rail::u64_value(resources.queue_units),
    ])?;
    Ok(Selected {
        execution,
        admitted,
        native,
        resources,
        resource_ref,
    })
}

fn execution_descriptor(
    source: &super::super::source::Prepared,
    profile_ref: String,
) -> molten::fabric_execution::ExecutionProfileDescriptor {
    molten::fabric_execution::ExecutionProfileDescriptor {
        schema: molten::fabric_execution::EXECUTION_PROFILE_SCHEMA.to_string(),
        profile_id: "native-callback-live-v1".to_string(),
        profile_ref,
        kind: molten::fabric_execution::ExecutionProfileKind::LiveBoundedProcess,
        platform: molten::fabric_execution::ExecutionPlatform::UnixProcessGroup,
        supported_termination_scopes: vec![molten::fabric_execution::ExecutionTerminationScope::ProcessGroup],
        max_timeout_ms: 5000,
        max_stdin_bytes: 1_048_576,
        max_stdout_bytes: 1_048_576,
        max_stderr_bytes: 1_048_576,
        max_poll_interval_ms: 5,
        max_teardown_timeout_ms: 1000,
        max_arguments: 16,
        max_argument_bytes: 4096,
        max_environment_entries: 16,
        max_environment_name_bytes: 128,
        max_environment_value_bytes: 4096,
        max_concurrency_units: 1,
        max_queue_units: 64,
        component_repository: molten::fabric_execution::BOUNDED_EXEC_REPOSITORY.to_string(),
        component_revision: molten::fabric_execution::BOUNDED_EXEC_REVISION.to_string(),
        component_license: molten::fabric_execution::BOUNDED_EXEC_LICENSE.to_string(),
        component_package: molten::fabric_execution::BOUNDED_EXEC_PACKAGE.to_string(),
        conformance_refs: vec![source.validation.selected_output_validation_ref.clone()],
        fabric_non_claims: molten::fabric::REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
        non_claims: molten::fabric_execution::REQUIRED_EXECUTION_NON_CLAIMS.to_vec(),
    }
}

fn execution_declaration(source: &super::super::source::Prepared) -> molten::error::Result<String> {
    super::super::source::record(&source.root, "experimental-bounded-process-profile-v1", vec![
        molten::preserves_rail::string(molten::fabric_execution::EXECUTION_PROFILE_SCHEMA),
        molten::preserves_rail::string("native-callback-live-v1"),
        molten::preserves_rail::string(molten::fabric_execution::ExecutionProfileKind::LiveBoundedProcess.as_str()),
        molten::preserves_rail::string(molten::fabric_execution::ExecutionPlatform::UnixProcessGroup.as_str()),
        molten::preserves_rail::string(molten::fabric_execution::ExecutionTerminationScope::ProcessGroup.as_str()),
        molten::preserves_rail::string(molten::fabric_execution::BOUNDED_EXEC_REPOSITORY),
        molten::preserves_rail::string(molten::fabric_execution::BOUNDED_EXEC_REVISION),
        molten::preserves_rail::string(molten::fabric_execution::BOUNDED_EXEC_LICENSE),
        molten::preserves_rail::string(molten::fabric_execution::BOUNDED_EXEC_PACKAGE),
        molten::preserves_rail::sequence(
            [
                5000_u64, 1_048_576, 1_048_576, 1_048_576, 5, 1000, 16, 4096, 16, 128, 4096, 1, 64,
            ]
            .into_iter()
            .map(molten::preserves_rail::u64_value)
            .collect(),
        ),
        molten::preserves_rail::string(&source.validation.selected_output_validation_ref),
        molten::preserves_rail::sequence(
            molten::fabric::REQUIRED_FABRIC_NON_CLAIMS
                .iter()
                .map(|claim| molten::preserves_rail::string(claim.as_str()))
                .collect(),
        ),
        molten::preserves_rail::sequence(
            molten::fabric_execution::REQUIRED_EXECUTION_NON_CLAIMS
                .iter()
                .map(|claim| molten::preserves_rail::string(claim.as_str()))
                .collect(),
        ),
        molten::preserves_rail::string("source-conformance-evidence-only-no-runtime-conformance-run-claimed"),
    ])
}

fn native_declaration(
    source: &super::super::source::Prepared,
    execution: &str,
    transport: &str,
) -> molten::error::Result<String> {
    super::super::source::record(&source.root, "experimental-native-host-profile-v1", vec![
        molten::preserves_rail::string(molten::system_extension::NATIVE_HOST_PROFILE_SCHEMA),
        molten::preserves_rail::string("native-host-local-pilot-v2"),
        molten::preserves_rail::string(execution),
        molten::preserves_rail::string(transport),
        molten::preserves_rail::string(molten::system_extension::NATIVE_ALPN),
        molten::preserves_rail::string(molten::system_extension::NATIVE_FRAMING),
        molten::preserves_rail::sequence(
            [1_048_576_u64, 1_048_576, 1_048_576, 262_144, 3, 128, 16, 16, 128]
                .into_iter()
                .map(molten::preserves_rail::u64_value)
                .collect(),
        ),
        molten::preserves_rail::bool_value(true),
        molten::preserves_rail::bool_value(true),
        molten::preserves_rail::sequence(
            molten::system_extension::REQUIRED_NATIVE_HOST_NON_CLAIMS
                .iter()
                .map(|claim| molten::preserves_rail::string(claim.as_str()))
                .collect(),
        ),
    ])
}
