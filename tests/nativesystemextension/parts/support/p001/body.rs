
fn execution_profile_descriptor() -> ExecutionProfileDescriptor {
    ExecutionProfileDescriptor {
        schema: EXECUTION_PROFILE_SCHEMA.to_string(),
        profile_id: "native-callback-live-v1".to_string(),
        profile_ref: HASH_B.to_string(),
        kind: ExecutionProfileKind::LiveBoundedProcess,
        platform: ExecutionPlatform::UnixProcessGroup,
        supported_termination_scopes: vec![ExecutionTerminationScope::ProcessGroup],
        max_timeout_ms: TIMEOUT_MS,
        max_stdin_bytes: CALLBACK_LIMIT,
        max_stdout_bytes: CALLBACK_LIMIT,
        max_stderr_bytes: DIAGNOSTIC_LIMIT,
        max_poll_interval_ms: POLL_INTERVAL_MS,
        max_teardown_timeout_ms: TEARDOWN_TIMEOUT_MS,
        max_arguments: MAX_ARGUMENTS,
        max_argument_bytes: MAX_ARGUMENT_BYTES,
        max_environment_entries: MAX_ENVIRONMENT,
        max_environment_name_bytes: MAX_ENVIRONMENT_NAME,
        max_environment_value_bytes: MAX_ENVIRONMENT_VALUE,
        max_concurrency_units: RESOURCE_UNITS,
        max_queue_units: QUEUE_UNITS,
        component_repository: BOUNDED_EXEC_REPOSITORY.to_string(),
        component_revision: BOUNDED_EXEC_REVISION.to_string(),
        component_license: BOUNDED_EXEC_LICENSE.to_string(),
        component_package: BOUNDED_EXEC_PACKAGE.to_string(),
        conformance_refs: vec![HASH_A.to_string()],
        fabric_non_claims: REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
        non_claims: REQUIRED_EXECUTION_NON_CLAIMS.to_vec(),
    }
}

fn native_profile(execution: &CanonicalExecutionProfile) -> NativeHostProfile {
    NativeHostProfile {
        schema: NATIVE_HOST_PROFILE_SCHEMA.to_string(),
        profile_id: "native-host-local-pilot-v2".to_string(),
        profile_ref: HASH_A.to_string(),
        execution_profile_ref: execution.profile_ref.clone(),
        transport_profile_ref: HASH_C.to_string(),
        alpn: NATIVE_ALPN.to_string(),
        framing: NATIVE_FRAMING.to_string(),
        max_callback_input_bytes: CALLBACK_LIMIT,
        max_callback_output_bytes: CALLBACK_LIMIT,
        max_diagnostic_bytes: DIAGNOSTIC_LIMIT,
        max_materialized_value_bytes: MAX_VALUE_BYTES,
        max_instances: MAX_INSTANCES,
        max_unresolved_operations: MAX_OPERATIONS,
        max_port_bindings: MAX_BINDINGS,
        max_policy_refs: MAX_POLICIES,
        max_materialized_values: MAX_OPERATIONS,
        is_local_live_pilot: true,
        requires_materialized_values: true,
        non_claims: REQUIRED_NATIVE_HOST_NON_CLAIMS.to_vec(),
    }
}

fn executable_evidence(
    admitted: &CanonicalAdmittedSystemExtensionManifest,
    execution: &CanonicalExecutionProfile,
    executable_bytes_ref: &str,
) -> NativeExecutableEvidence {
    NativeExecutableEvidence {
        schema: NATIVE_EXECUTABLE_EVIDENCE_SCHEMA.to_string(),
        executable_ref: HASH_E.to_string(),
        executable_bytes_ref: executable_bytes_ref.to_string(),
        artifact_kind_ref: HASH_A.to_string(),
        target_ref: HASH_B.to_string(),
        dependency_closure_ref: HASH_C.to_string(),
        materialization_ref: HASH_D.to_string(),
        provenance_ref: HASH_E.to_string(),
        source_gate_ref: HASH_F.to_string(),
        policy_ref: HASH_A.to_string(),
        authority_ref: HASH_B.to_string(),
        resource_ref: HASH_C.to_string(),
        execution_profile_ref: execution.profile_ref.clone(),
        manifest_ref: admitted.manifest_ref().to_string(),
        state_schema_ref: HASH_D.to_string(),
        port_binding_refs: admitted.all_binding_refs().map(str::to_string).collect(),
    }
}

struct ExecutionTemplateInput<'a> {
    native_profile: &'a AdmittedNativeHostProfile,
    executable: &'a AdmittedNativeExecutable,
    execution: &'a CanonicalExecutionProfile,
    executable_path: std::path::PathBuf,
    instance_id: String,
    admitted: &'a CanonicalAdmittedSystemExtensionManifest,
}

fn execution_template(input: ExecutionTemplateInput<'_>) -> NativeExecutionTemplate {
    let ExecutionTemplateInput {
        native_profile,
        executable,
        execution,
        executable_path,
        instance_id,
        admitted,
    } = input;
    NativeExecutionTemplate {
        host_profile: native_profile.clone(),
        executable: executable.clone(),
        admitted: admitted.clone(),
        request: execution_request(executable, execution, admitted),
        authority: ExecutionAuthorityFacts {
            authority_ref: executable.executable.authority_ref.clone(),
            executable_authority_ref: HASH_A.to_string(),
            provenance_ref: executable.executable.provenance_ref.clone(),
            effect_admission_ref: HASH_B.to_string(),
            workspace_authority_ref: HASH_C.to_string(),
            process_authority_ref: HASH_D.to_string(),
            resource_grant_ref: executable.executable.resource_ref.clone(),
            policy_ref: executable.executable.policy_ref.clone(),
            executable_artifact_ref: executable.executable.executable_ref.clone(),
            executable_identity_ref: executable.executable.executable_bytes_ref.clone(),
            workspace_ref: HASH_D.to_string(),
            operation_ref: HASH_A.to_string(),
            extension_id: admitted.manifest().extension_id.clone(),
            service_id: admitted.manifest().service_id.clone(),
            generation: GENERATION,
            profile_ref: execution.profile.descriptor.profile_ref.clone(),
        },
        resources: ExecutionResourceGrant {
            memory_bytes: CALLBACK_LIMIT + DIAGNOSTIC_LIMIT,
            storage_bytes: CALLBACK_LIMIT,
            diagnostic_bytes: CALLBACK_LIMIT + DIAGNOSTIC_LIMIT,
            logical_deadline_ticks: LOGICAL_DEADLINE,
            concurrency_units: RESOURCE_UNITS,
            queue_units: QUEUE_UNITS,
        },
        resolved: ResolvedExecutionContext {
            executable_path,
            executable_artifact_ref: executable.executable.executable_ref.clone(),
            executable_identity_ref: executable.executable.executable_bytes_ref.clone(),
            workspace_path: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            workspace_ref: HASH_D.to_string(),
            stdin_ref: None,
            stdin_bytes: None,
        },
        context: NativeCallbackContext {
            manifest_ref: admitted.manifest_ref().to_string(),
            executable_ref: executable.executable.executable_ref.clone(),
            instance_id,
            extension_id: admitted.manifest().extension_id.clone(),
            service_id: admitted.manifest().service_id.clone(),
            state_ref: None,
            policy_refs: admitted.manifest().policy_refs.clone(),
            resource_ref: executable.executable.resource_ref.clone(),
            port_binding_refs: admitted.all_binding_refs().map(str::to_string).collect(),
        },
    }
}

/// The direct, clear-environment, exact-artifact callback execution request for the admitted
/// executable.
fn execution_request(
    executable: &AdmittedNativeExecutable,
    execution: &CanonicalExecutionProfile,
    admitted: &CanonicalAdmittedSystemExtensionManifest,
) -> ExecutionRequest {
    ExecutionRequest {
        schema: EXECUTION_REQUEST_SCHEMA.to_string(),
        operation_ref: HASH_A.to_string(),
        idempotency_ref: HASH_B.to_string(),
        extension_id: admitted.manifest().extension_id.clone(),
        service_id: admitted.manifest().service_id.clone(),
        callback_ref: HASH_C.to_string(),
        effect_ref: HASH_D.to_string(),
        generation: GENERATION,
        profile_ref: execution.profile.descriptor.profile_ref.clone(),
        executable_artifact_ref: executable.executable.executable_ref.clone(),
        executable_identity_ref: executable.executable.executable_bytes_ref.clone(),
        arguments: Vec::new(),
        environment: Vec::new(),
        environment_mode: ExecutionEnvironmentMode::Clear,
        invocation_mode: ExecutionInvocationMode::Direct,
        executable_resolution: ExecutableResolutionMode::ExactArtifact,
        workspace_ref: HASH_D.to_string(),
        workspace_mode: WorkspaceMode::CapabilityRoot,
        stdin_ref: Some(HASH_E.to_string()),
        limits: ExecutionRequestLimits {
            timeout_ms: TIMEOUT_MS,
            stdin_max_bytes: CALLBACK_LIMIT,
            stdout_max_bytes: CALLBACK_LIMIT,
            stderr_max_bytes: DIAGNOSTIC_LIMIT,
            poll_interval_ms: POLL_INTERVAL_MS,
            teardown_timeout_ms: TEARDOWN_TIMEOUT_MS,
            concurrency_units: RESOURCE_UNITS,
            queue_units: RESOURCE_UNITS,
        },
        termination_scope: ExecutionTerminationScope::ProcessGroup,
        accepted_exit_codes: vec![SUCCESS_EXIT_CODE],
        reject_stdout_truncation: true,
        reject_stderr_truncation: true,
        authority_ref: executable.executable.authority_ref.clone(),
        resource_grant_ref: executable.executable.resource_ref.clone(),
    }
}

fn admitted_manifest() -> TestResult<CanonicalAdmittedSystemExtensionManifest> {
    let tier = canonical_extension_tier_admission(&ExtensionTierRequest {
        tier: ExtensionTier::SystemExtension,
        requested_authorities: vec![
            FabricAuthority::Execution,
            FabricAuthority::Resources,
            FabricAuthority::Supervision,
            FabricAuthority::Evidence,
        ],
        admission_evidence: REQUIRED_SYSTEM_EXTENSION_EVIDENCE.to_vec(),
    })
    .or_fail("native extension tier")?;
    canonical_admit_system_extension_manifest(
        &SystemExtensionManifestInput {
            schema: SYSTEM_EXTENSION_MANIFEST_SCHEMA.to_string(),
            extension_id: "molten.fixture.native".to_string(),
            service_id: "molten.fixture.native.service".to_string(),
            implementation_ref: HASH_E.to_string(),
            callback_groups: vec![
                "initialize".to_string(),
                "start".to_string(),
                "request".to_string(),
                "message".to_string(),
                "health".to_string(),
                "checkpoint".to_string(),
                "recover".to_string(),
                "drain".to_string(),
                "shutdown".to_string(),
            ],
            required_ports: vec![effect_requirement()],
            optional_ports: Vec::new(),
            capability_refs: vec![HASH_B.to_string()],
            policy_refs: vec![HASH_C.to_string()],
            provenance_refs: vec![HASH_D.to_string()],
            resources: ResourceEnvelope {
                max_concurrent_callbacks: MAX_CONCURRENT_CALLBACKS,
                max_queued_events: MAX_QUEUED_EVENTS,
                max_inflight_bytes: MAX_INFLIGHT_BYTES,
                max_open_streams: MAX_OPEN_STREAMS,
                max_timers: MAX_TIMERS,
                max_effect_requests: MAX_EFFECT_REQUESTS,
                callback_deadline_ticks: CALLBACK_DEADLINE_TICKS,
                shutdown_grace_ticks: SHUTDOWN_GRACE_TICKS,
                max_restart_attempts: MAX_RESTART_ATTEMPTS,
                overload_policy: OverloadPolicy::UpstreamBackpressure,
            },
            execution_profile: ExecutionProfile::NativeProcess,
            state_schema: "molten.fixture.native.state.v1".to_string(),
            compatible_state_schemas: vec!["molten.fixture.native.state.v1".to_string()],
            evidence_profile_ref: HASH_E.to_string(),
            initial_generation: GENERATION,
            non_claims: REQUIRED_SYSTEM_EXTENSION_NON_CLAIMS.to_vec(),
        },
        &[effect_descriptor()],
        &tier,
        &[ExecutionProfile::NativeProcess],
    )
    .or_fail("native manifest")
}

fn effect_descriptor() -> FabricPortDescriptor {
    FabricPortDescriptor {
        schema: FABRIC_PORT_DESCRIPTOR_SCHEMA.to_string(),
        port_id: EFFECT_PORT_ID.to_string(),
        version: EFFECT_PORT_VERSION.to_string(),
        class: FabricPortClass::Evidence,
        operation_classes: vec![EFFECT_OPERATION.to_string()],
        input_schema_refs: vec![EFFECT_INPUT_SCHEMA.to_string()],
        output_schema_refs: vec![EFFECT_OUTPUT_SCHEMA.to_string()],
        authority_requirements: vec![FabricAuthority::Evidence],
        resource_requirements: vec![FabricResource::Diagnostics],
        determinism: DeterminismClass::ExternalEffect,
        replay: ReplayClass::RecordedEffectRequired,
        implementation_profile: "native-fixture-effect-v1".to_string(),
        conformance_refs: vec![HASH_A.to_string()],
        non_claims: REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
        enabled: true,
    }
}
