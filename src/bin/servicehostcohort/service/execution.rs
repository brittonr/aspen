pub(super) struct Template {
    pub request: molten::fabric_execution::ExecutionRequest,
    pub authority: molten::fabric_execution::ExecutionAuthorityFacts,
    pub resolved: molten::fabric_execution::ResolvedContext,
    pub context: molten::system_extension::CallbackContext,
}

pub(super) fn build(
    context: &super::Context<'_>,
    admitted: &molten::system_extension::AdmittedManifest,
    executable: &molten::system_extension::NativeExecutableEvidence,
    cache: &str,
) -> molten::error::Result<Template> {
    let source = context.source;
    source.root.ledger()?.open_subdir(&molten::node_state::RelativePath::parse(cache)?)?;
    let workspace_ref = super::super::source::record(&source.root, "experimental-capability-workspace-v1", vec![
        molten::preserves_rail::string(cache),
        molten::preserves_rail::string(&context.grant_ref),
        molten::preserves_rail::string(&source.validation.binding_ref),
    ])?;
    let operation_ref = super::super::source::record(&source.root, "native-callback-execution-template-v1", vec![
        molten::preserves_rail::string(admitted.manifest_ref()),
        molten::preserves_rail::string(&source.callback_ref),
        molten::preserves_rail::string(&workspace_ref),
        molten::preserves_rail::string("template-not-observed-execution"),
    ])?;
    let request = request(context, admitted, executable, &operation_ref, &workspace_ref);
    let authority = molten::fabric_execution::ExecutionAuthorityFacts {
        authority_ref: context.grant_ref.clone(),
        executable_authority_ref: context.grant_ref.clone(),
        provenance_ref: source.provenance_ref.clone(),
        effect_admission_ref: context.grant_ref.clone(),
        workspace_authority_ref: context.grant_ref.clone(),
        process_authority_ref: context.grant_ref.clone(),
        resource_grant_ref: context.profiles.resource_ref.clone(),
        policy_ref: context.policy_ref.to_string(),
        executable_artifact_ref: source.artifact_ref.clone(),
        executable_identity_ref: source.callback_ref.clone(),
        workspace_ref: workspace_ref.clone(),
        operation_ref,
        extension_id: admitted.manifest().extension_id.clone(),
        service_id: admitted.manifest().service_id.clone(),
        generation: 1,
        profile_ref: context.profiles.execution.profile_ref.clone(),
    };
    let resolved = molten::fabric_execution::ResolvedContext {
        executable_path: source.output.join("bin/molten-native-extension-fixture"),
        executable_artifact_ref: source.artifact_ref.clone(),
        executable_identity_ref: source.callback_ref.clone(),
        workspace_path: source.root_path.join("ledger").join(cache),
        workspace_ref,
        stdin_ref: None,
        stdin_bytes: None,
    };
    let callback = callback(context, admitted);
    molten::fabric_execution::admit_request(
        &context.profiles.admitted,
        &request,
        &authority,
        context.profiles.resources,
        1,
    )?;
    Ok(Template {
        request,
        authority,
        resolved,
        context: callback,
    })
}

fn request(
    context: &super::Context<'_>,
    admitted: &molten::system_extension::AdmittedManifest,
    executable: &molten::system_extension::NativeExecutableEvidence,
    operation_ref: &str,
    workspace_ref: &str,
) -> molten::fabric_execution::ExecutionRequest {
    molten::fabric_execution::ExecutionRequest {
        schema: molten::fabric_execution::EXECUTION_REQUEST_SCHEMA.to_string(),
        operation_ref: operation_ref.to_string(),
        idempotency_ref: operation_ref.to_string(),
        extension_id: admitted.manifest().extension_id.clone(),
        service_id: admitted.manifest().service_id.clone(),
        callback_ref: context.profiles.native.profile_ref.clone(),
        effect_ref: operation_ref.to_string(),
        generation: 1,
        profile_ref: context.profiles.execution.profile_ref.clone(),
        executable_artifact_ref: executable.executable_ref.clone(),
        executable_identity_ref: executable.executable_bytes_ref.clone(),
        arguments: Vec::new(),
        environment: Vec::new(),
        environment_mode: molten::fabric_execution::ExecutionEnvironmentMode::Clear,
        invocation_mode: molten::fabric_execution::ExecutionInvocationMode::Direct,
        executable_resolution: molten::fabric_execution::ExecutableResolutionMode::ExactArtifact,
        workspace_ref: workspace_ref.to_string(),
        workspace_mode: molten::fabric_execution::WorkspaceMode::CapabilityRoot,
        stdin_ref: None,
        limits: molten::fabric_execution::ExecutionRequestLimits {
            timeout_ms: 5000,
            stdin_max_bytes: 1_048_576,
            stdout_max_bytes: 1_048_576,
            stderr_max_bytes: 1_048_576,
            poll_interval_ms: 5,
            teardown_timeout_ms: 1000,
            concurrency_units: 1,
            queue_units: 1,
        },
        termination_scope: molten::fabric_execution::ExecutionTerminationScope::ProcessGroup,
        accepted_exit_codes: vec![0],
        reject_stdout_truncation: true,
        reject_stderr_truncation: true,
        authority_ref: context.grant_ref.clone(),
        resource_grant_ref: context.profiles.resource_ref.clone(),
    }
}

fn callback(
    context: &super::Context<'_>,
    admitted: &molten::system_extension::AdmittedManifest,
) -> molten::system_extension::CallbackContext {
    let instance_id = molten::system_extension::native_identity_ref(&[
        "native-instance-v2",
        &admitted.manifest().extension_id,
        &admitted.manifest().service_id,
        admitted.manifest_ref(),
    ]);
    molten::system_extension::CallbackContext {
        manifest_ref: admitted.manifest_ref().to_string(),
        executable_ref: context.source.artifact_ref.clone(),
        instance_id,
        extension_id: admitted.manifest().extension_id.clone(),
        service_id: admitted.manifest().service_id.clone(),
        state_ref: None,
        policy_refs: admitted.manifest().policy_refs.clone(),
        resource_ref: context.profiles.resource_ref.clone(),
        port_binding_refs: admitted.all_binding_refs().map(str::to_string).collect(),
    }
}
