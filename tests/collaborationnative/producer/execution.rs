pub(super) struct Template {
    pub request: molten::fabric_execution::ExecutionRequest,
    pub authority: molten::fabric_execution::ExecutionAuthorityFacts,
    pub resolved: molten::fabric_execution::ResolvedContext,
    pub context: molten::system_extension::CallbackContext,
}

pub(super) fn build(
    source: &super::Source,
    profiles: &super::mechanisms::Profiles,
    admitted: &molten::system_extension::AdmittedManifest,
) -> super::Result<Template> {
    source
        .root
        .ledger()?
        .open_subdir(&molten::node_state::RelativePath::parse("collaboration-callback")?)?;
    let workspace_ref = super::record(&source.root, "collaboration-callback-workspace-v1", &[
        "collaboration-callback",
        &profiles.grant_ref,
        &source.validation.binding_ref,
    ])?;
    let operation_ref = super::record(&source.root, "collaboration-native-execution-template-v1", &[
        admitted.manifest_ref(),
        &source.bytes_ref,
        &workspace_ref,
        "template-not-observed-execution",
    ])?;
    let request = request(source, profiles, &workspace_ref, &operation_ref);
    let authority = authority(source, profiles, &workspace_ref, operation_ref);
    let resolved = context_paths(source, workspace_ref);
    let context = callback(source, profiles, admitted);
    molten::fabric_execution::admit_request(
        &molten::fabric_execution::admit_profile(&profiles.execution)?,
        &request,
        &authority,
        profiles.resources,
        1,
    )?;
    Ok(Template {
        request,
        authority,
        resolved,
        context,
    })
}

/// Every authority fact names the current request scope, never ambient node identity.
fn authority(
    source: &super::Source,
    profiles: &super::mechanisms::Profiles,
    workspace_ref: &str,
    operation_ref: String,
) -> molten::fabric_execution::ExecutionAuthorityFacts {
    molten::fabric_execution::ExecutionAuthorityFacts {
        authority_ref: profiles.grant_ref.clone(),
        executable_authority_ref: profiles.grant_ref.clone(),
        provenance_ref: source.provenance_ref.clone(),
        effect_admission_ref: profiles.grant_ref.clone(),
        workspace_authority_ref: profiles.grant_ref.clone(),
        process_authority_ref: profiles.grant_ref.clone(),
        resource_grant_ref: profiles.resource_ref.clone(),
        policy_ref: profiles.policy_ref.clone(),
        executable_artifact_ref: source.artifact_ref.clone(),
        executable_identity_ref: source.bytes_ref.clone(),
        workspace_ref: workspace_ref.to_owned(),
        operation_ref,
        extension_id: molten::collaboration_scope::EXTENSION_ID.into(),
        service_id: super::SERVICE.into(),
        generation: 1,
        profile_ref: profiles.execution.profile_ref.clone(),
    }
}

/// The resolved executable and workspace paths for the admitted callback.
fn context_paths(source: &super::Source, workspace_ref: String) -> molten::fabric_execution::ResolvedContext {
    molten::fabric_execution::ResolvedContext {
        executable_path: source.path.join("artifacts/collaboration-scope"),
        executable_artifact_ref: source.artifact_ref.clone(),
        executable_identity_ref: source.bytes_ref.clone(),
        workspace_path: source.path.join("ledger/collaboration-callback"),
        workspace_ref,
        stdin_ref: None,
        stdin_bytes: None,
    }
}

/// The callback context the installed manifest admits for exactly one native instance.
fn callback(
    source: &super::Source,
    profiles: &super::mechanisms::Profiles,
    admitted: &molten::system_extension::AdmittedManifest,
) -> molten::system_extension::CallbackContext {
    let instance_id = molten::system_extension::native_identity_ref(&[
        "native-instance-v2",
        molten::collaboration_scope::EXTENSION_ID,
        super::SERVICE,
        admitted.manifest_ref(),
    ]);
    molten::system_extension::CallbackContext {
        manifest_ref: admitted.manifest_ref().into(),
        executable_ref: source.artifact_ref.clone(),
        instance_id,
        extension_id: molten::collaboration_scope::EXTENSION_ID.into(),
        service_id: super::SERVICE.into(),
        state_ref: None,
        policy_refs: admitted.manifest().policy_refs.clone(),
        resource_ref: profiles.resource_ref.clone(),
        port_binding_refs: admitted.all_binding_refs().map(str::to_string).collect(),
    }
}

fn request(
    source: &super::Source,
    profiles: &super::mechanisms::Profiles,
    workspace_ref: &str,
    operation_ref: &str,
) -> molten::fabric_execution::ExecutionRequest {
    molten::fabric_execution::ExecutionRequest {
        schema: molten::fabric_execution::EXECUTION_REQUEST_SCHEMA.into(),
        operation_ref: operation_ref.into(),
        idempotency_ref: operation_ref.into(),
        extension_id: molten::collaboration_scope::EXTENSION_ID.into(),
        service_id: super::SERVICE.into(),
        callback_ref: profiles.native.profile_ref.clone(),
        effect_ref: operation_ref.into(),
        generation: 1,
        profile_ref: profiles.execution.profile_ref.clone(),
        executable_artifact_ref: source.artifact_ref.clone(),
        executable_identity_ref: source.bytes_ref.clone(),
        arguments: Vec::new(),
        environment: Vec::new(),
        environment_mode: molten::fabric_execution::ExecutionEnvironmentMode::Clear,
        invocation_mode: molten::fabric_execution::ExecutionInvocationMode::Direct,
        executable_resolution: molten::fabric_execution::ExecutableResolutionMode::ExactArtifact,
        workspace_ref: workspace_ref.into(),
        workspace_mode: molten::fabric_execution::WorkspaceMode::CapabilityRoot,
        stdin_ref: None,
        limits: molten::fabric_execution::ExecutionRequestLimits {
            timeout_ms: 60_000,
            stdin_max_bytes: 8_388_608,
            stdout_max_bytes: 8_388_608,
            stderr_max_bytes: 8_388_608,
            poll_interval_ms: 5,
            teardown_timeout_ms: 5_000,
            concurrency_units: 1,
            queue_units: 1,
        },
        termination_scope: molten::fabric_execution::ExecutionTerminationScope::ProcessGroup,
        accepted_exit_codes: vec![0],
        reject_stdout_truncation: true,
        reject_stderr_truncation: true,
        authority_ref: profiles.grant_ref.clone(),
        resource_grant_ref: profiles.resource_ref.clone(),
    }
}
