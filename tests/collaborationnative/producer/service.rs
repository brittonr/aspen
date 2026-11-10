pub(super) fn build(
    source: &super::Source,
    mechanisms: &molten::live_reliability::service::profiles::Experimental,
) -> super::Result<molten::node_service_host::codec::ExtensionInput> {
    let profiles = super::mechanisms::build(source)?;
    let descriptors = super::mechanisms::descriptors(mechanisms)?;
    let manifest = manifest(source, &profiles, &descriptors);
    let tier = tier(&descriptors);
    let admitted_tier = molten::fabric::canonical_extension_tier_admission(&tier)?;
    let admitted =
        molten::collaboration_scope::admit_extension(&manifest, molten::collaboration_scope::AdmissionContext {
            descriptors: &descriptors,
            tier: &admitted_tier,
            execution_profiles: &[molten::system_extension::ExecutionProfile::NativeProcess],
        })?;
    let executable = executable(source, &profiles, &admitted)?;
    let native = molten::system_extension::admit_native_host_profile(&profiles.native)
        .map_err(|error| format!("native host profile: {error:?}"))?;
    molten::system_extension::admit_native_executable(&native, &executable)
        .map_err(|error| format!("native executable: {error:?}"))?;
    let execution = super::execution::build(source, &profiles, &admitted)?;
    let mechanisms = super::mechanisms::experimental("collaboration-journal")?;
    let durability = mechanisms.durable.into_profile()?;
    let namespace = mechanisms.namespace.into_descriptor(&durability)?;
    Ok(molten::node_service_host::codec::ExtensionInput {
        manifest,
        tier,
        descriptors,
        host_profile: profiles.native,
        executable,
        execution_profile: profiles.execution,
        request: execution.request,
        authority: execution.authority,
        resources: profiles.resources,
        resolved: execution.resolved,
        context: execution.context,
        durability_profile: durability.profile,
        namespace,
        artifact_path: "artifacts/collaboration-scope".into(),
        storage_namespace: "collaboration-journal".into(),
        cache_namespace: "collaboration-callback".into(),
    })
}

/// The system-extension tier requests exactly the declared port authorities plus fabric domains.
fn tier(descriptors: &[molten::fabric::FabricPortDescriptor]) -> molten::fabric::ExtensionTierRequest {
    let mut authorities = descriptors
        .iter()
        .flat_map(|port| port.authority_requirements.iter().copied())
        .collect::<std::collections::BTreeSet<_>>();
    authorities.extend([
        molten::fabric::FabricAuthority::Execution,
        molten::fabric::FabricAuthority::Resources,
        molten::fabric::FabricAuthority::Supervision,
        molten::fabric::FabricAuthority::Evidence,
    ]);
    molten::fabric::ExtensionTierRequest {
        tier: molten::fabric::ExtensionTier::SystemExtension,
        requested_authorities: authorities.into_iter().collect(),
        admission_evidence: molten::fabric::REQUIRED_SYSTEM_EXTENSION_EVIDENCE.to_vec(),
    }
}

/// The executable evidence the native host admission measures.
fn executable(
    source: &super::Source,
    profiles: &super::mechanisms::Profiles,
    admitted: &molten::system_extension::AdmittedManifest,
) -> super::Result<molten::system_extension::NativeExecutableEvidence> {
    let state_schema_ref = super::record(&source.root, "collaboration-native-state-schema-v1", &[
        molten::collaboration_scope::SCHEMA,
        "collaboration-native-bridge-v1",
        &source.bytes_ref,
    ])?;
    Ok(molten::system_extension::NativeExecutableEvidence {
        schema: molten::system_extension::NATIVE_EXECUTABLE_EVIDENCE_SCHEMA.into(),
        executable_ref: source.artifact_ref.clone(),
        executable_bytes_ref: source.bytes_ref.clone(),
        artifact_kind_ref: source.kind_ref.clone(),
        target_ref: source.target_ref.clone(),
        dependency_closure_ref: source.closure_ref.clone(),
        materialization_ref: source.materialization_ref.clone(),
        provenance_ref: source.provenance_ref.clone(),
        source_gate_ref: source.validation.expected_startup_validation_ref.clone(),
        policy_ref: profiles.policy_ref.clone(),
        authority_ref: profiles.grant_ref.clone(),
        resource_ref: profiles.resource_ref.clone(),
        execution_profile_ref: profiles.execution.profile_ref.clone(),
        manifest_ref: admitted.manifest_ref().into(),
        state_schema_ref,
        port_binding_refs: admitted.all_binding_refs().map(str::to_string).collect(),
    })
}

fn manifest(
    source: &super::Source,
    profiles: &super::mechanisms::Profiles,
    descriptors: &[molten::fabric::FabricPortDescriptor],
) -> molten::system_extension::SystemExtensionManifestInput {
    molten::system_extension::SystemExtensionManifestInput {
        schema: molten::system_extension::SYSTEM_EXTENSION_MANIFEST_SCHEMA.into(),
        extension_id: molten::collaboration_scope::EXTENSION_ID.into(),
        service_id: super::SERVICE.into(),
        implementation_ref: source.artifact_ref.clone(),
        callback_groups: [
            "initialize",
            "start",
            "request",
            "message",
            "health",
            "checkpoint",
            "recover",
            "drain",
            "shutdown",
        ]
        .iter()
        .map(|name| name.to_string())
        .collect(),
        required_ports: descriptors.iter().map(super::mechanisms::requirement).collect(),
        optional_ports: Vec::new(),
        capability_refs: vec![profiles.grant_ref.clone()],
        policy_refs: vec![profiles.policy_ref.clone()],
        provenance_refs: vec![source.provenance_ref.clone()],
        resources: molten::system_extension::ResourceEnvelope {
            max_concurrent_callbacks: 1,
            max_queued_events: 8,
            max_inflight_bytes: molten::collaboration_scope::MAX_RECORD_BYTES,
            max_open_streams: 8,
            max_timers: 128,
            max_effect_requests: 8,
            callback_deadline_ticks: 100,
            shutdown_grace_ticks: 100,
            max_restart_attempts: 16,
            overload_policy: molten::system_extension::OverloadPolicy::UpstreamBackpressure,
        },
        // The operator installs this exact admitted group after readiness; without it the host
        // has no member and no work can be submitted to the extension.
        recovery_group: Some(molten::system_extension::RecoveryGroupDeclaration {
            group_id: super::GROUP_ID.into(),
            members: vec![molten::system_extension::RecoveryGroupMember {
                member_id: super::SERVICE.into(),
                restart_class: molten::system_extension::RestartClass::Transient,
                after: Vec::new(),
            }],
            window_ticks: 1_000,
            backoff_ticks: 10,
            max_restarts_per_member: 16,
            max_restarts_per_group: 16,
            elapsed_source_ref: super::reference("collaboration-native-elapsed-source"),
        }),
        execution_profile: molten::system_extension::ExecutionProfile::NativeProcess,
        state_schema: molten::collaboration_scope::SCHEMA.into(),
        compatible_state_schemas: vec![molten::collaboration_scope::SCHEMA.into()],
        evidence_profile_ref: source.validation.selected_output_validation_ref.clone(),
        initial_generation: 1,
        non_claims: molten::system_extension::REQUIRED_SYSTEM_EXTENSION_NON_CLAIMS.to_vec(),
    }
}
