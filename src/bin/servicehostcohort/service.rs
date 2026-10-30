mod durability;
mod execution;
mod manifest;
mod profiles;

pub(super) struct Context<'a> {
    pub source: &'a super::source::Prepared,
    pub profiles: profiles::Selected,
    pub policy_ref: &'a str,
    pub grant_ref: String,
    pub evidence_ref: String,
    state_v1_ref: String,
    state_v2_ref: String,
}

impl<'a> Context<'a> {
    pub(super) fn new(
        source: &'a super::source::Prepared,
        operator: &super::operator::Authority,
        policy_ref: &'a str,
    ) -> molten::error::Result<Self> {
        let profiles = profiles::build(source)?;
        let scope_ref = super::source::record(&source.root, "experimental-native-execution-scope-v1", vec![
            molten::preserves_rail::string(&source.validation.selected_output_validation_ref),
            molten::preserves_rail::string(&source.callback_ref),
            molten::preserves_rail::string(policy_ref),
            molten::preserves_rail::string(&profiles.native.profile_ref),
            molten::preserves_rail::string(&profiles.resource_ref),
        ])?;
        let grant_ref = operator.grant(
            &source.root,
            &format!("molten://node-service-host/experimental-native/{scope_ref}"),
            "execute",
            &scope_ref,
        )?;
        let evidence_ref = super::source::record(&source.root, "experimental-native-evidence-profile-v1", vec![
            molten::preserves_rail::string(&source.validation.selected_output_validation_ref),
            molten::preserves_rail::string(&grant_ref),
            molten::preserves_rail::string("source-selection-and-authorship-only-runtime-tests-still-required"),
        ])?;
        let state_v1_ref = state_schema(source, "molten.fixture.native.state.v1")?;
        let state_v2_ref = state_schema(source, "molten.fixture.native.state.v2")?;
        Ok(Self {
            source,
            profiles,
            policy_ref,
            grant_ref,
            evidence_ref,
            state_v1_ref,
            state_v2_ref,
        })
    }
}

pub(super) fn build(
    context: &Context<'_>,
    member: &str,
    group: &molten::system_extension::RecoveryGroupDeclaration,
    successor: bool,
) -> molten::error::Result<molten::node_service_host::codec::ExtensionInput> {
    let manifest = manifest::build(context, member, group, successor);
    let tier = manifest::tier();
    let descriptors = vec![manifest::descriptor(
        &context.source.validation.selected_output_validation_ref,
    )];
    let admitted_tier = molten::fabric::canonical_extension_tier_admission(&tier)?;
    let admitted = molten::system_extension::admit_manifest(&manifest, &descriptors, &admitted_tier, &[
        molten::system_extension::ExecutionProfile::NativeProcess,
    ])?;
    super::source::retain(&context.source.root, admitted.value())?;
    let executable = molten::system_extension::NativeExecutableEvidence {
        schema: molten::system_extension::NATIVE_EXECUTABLE_EVIDENCE_SCHEMA.to_string(),
        executable_ref: context.source.artifact_ref.clone(),
        executable_bytes_ref: context.source.callback_ref.clone(),
        artifact_kind_ref: context.source.kind_ref.clone(),
        target_ref: context.source.target_ref.clone(),
        dependency_closure_ref: context.source.closure_ref.clone(),
        materialization_ref: context.source.materialization_ref.clone(),
        provenance_ref: context.source.provenance_ref.clone(),
        source_gate_ref: context.source.validation.expected_startup_validation_ref.clone(),
        policy_ref: context.policy_ref.to_string(),
        authority_ref: context.grant_ref.clone(),
        resource_ref: context.profiles.resource_ref.clone(),
        execution_profile_ref: context.profiles.admitted.profile_ref.clone(),
        manifest_ref: admitted.manifest_ref().to_string(),
        state_schema_ref: if successor {
            context.state_v2_ref.clone()
        } else {
            context.state_v1_ref.clone()
        },
        port_binding_refs: admitted.all_binding_refs().map(str::to_string).collect(),
    };
    let profile = molten::system_extension::admit_native_host_profile(&context.profiles.native)
        .map_err(|issues| super::invalid(format!("authored native profile denied: {issues:?}")))?;
    molten::system_extension::admit_native_executable(&profile, &executable)
        .map_err(|issues| super::invalid(format!("authored native executable denied: {issues:?}")))?;
    let cache_namespace = format!("callback-values/{member}");
    let execution = execution::build(context, &admitted, &executable, &cache_namespace)?;
    let (durability_profile, namespace) = durability::build(context, member)?;
    Ok(molten::node_service_host::codec::ExtensionInput {
        manifest,
        tier,
        descriptors,
        host_profile: context.profiles.native.clone(),
        executable,
        execution_profile: context.profiles.execution.clone(),
        request: execution.request,
        authority: execution.authority,
        resources: context.profiles.resources,
        resolved: execution.resolved,
        context: execution.context,
        durability_profile,
        namespace,
        artifact_path: "artifacts/native-callback".to_string(),
        storage_namespace: format!("journals/{member}"),
        cache_namespace,
    })
}

fn state_schema(source: &super::source::Prepared, schema: &str) -> molten::error::Result<String> {
    super::source::record(&source.root, "declared-materialized-native-state-schema-v1", vec![
        molten::preserves_rail::string(schema),
        molten::preserves_rail::string(&source.callback_ref),
        molten::preserves_rail::sequence(
            [
                "instance-id:utf8-blake3-reference",
                "generation:u64-little-endian",
                "callback-sequence:u64-little-endian",
                "prior-state:remaining-bytes",
            ]
            .iter()
            .map(molten::preserves_rail::string)
            .collect(),
        ),
        molten::preserves_rail::string("declared-callback-schema-not-an-execution-receipt"),
    ])
}
