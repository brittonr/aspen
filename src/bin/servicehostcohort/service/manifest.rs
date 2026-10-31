pub(super) fn tier() -> molten::fabric::ExtensionTierRequest {
    molten::fabric::ExtensionTierRequest {
        tier: molten::fabric::ExtensionTier::SystemExtension,
        requested_authorities: vec![
            molten::fabric::FabricAuthority::Execution,
            molten::fabric::FabricAuthority::Resources,
            molten::fabric::FabricAuthority::Supervision,
            molten::fabric::FabricAuthority::Evidence,
        ],
        admission_evidence: molten::fabric::REQUIRED_SYSTEM_EXTENSION_EVIDENCE.to_vec(),
    }
}

pub(super) fn descriptor(source_ref: &str) -> molten::fabric::FabricPortDescriptor {
    molten::fabric::FabricPortDescriptor {
        schema: molten::fabric::FABRIC_PORT_DESCRIPTOR_SCHEMA.to_string(),
        port_id: "molten.fixture.native.effect".to_string(),
        version: "v1".to_string(),
        class: molten::fabric::FabricPortClass::Evidence,
        operation_classes: vec!["fixture-effect".to_string()],
        // The fixture's schema labels travel as BLAKE3 content identities on the service-host wire.
        input_schema_refs: vec![molten::preserves_rail::content_ref_from_bytes(
            b"molten.fixture.native.effect-input.v1",
        )],
        output_schema_refs: vec![molten::preserves_rail::content_ref_from_bytes(
            b"molten.fixture.native.effect-output.v1",
        )],
        authority_requirements: vec![molten::fabric::FabricAuthority::Evidence],
        resource_requirements: vec![molten::fabric::FabricResource::Diagnostics],
        determinism: molten::fabric::DeterminismClass::ExternalEffect,
        replay: molten::fabric::ReplayClass::RecordedEffectRequired,
        implementation_profile: "native-fixture-effect-v1".to_string(),
        conformance_refs: vec![source_ref.to_string()],
        non_claims: molten::fabric::REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
        enabled: true,
    }
}

pub(super) fn build(
    context: &super::Context<'_>,
    member: &str,
    group: &molten::system_extension::RecoveryGroupDeclaration,
    successor: bool,
) -> molten::system_extension::SystemExtensionManifestInput {
    let descriptor = descriptor(&context.source.validation.selected_output_validation_ref);
    let requirement = molten::fabric::FabricPortRequirement {
        port_id: descriptor.port_id,
        version: descriptor.version,
        class: descriptor.class,
        operation_classes: descriptor.operation_classes,
        input_schema_refs: descriptor.input_schema_refs,
        output_schema_refs: descriptor.output_schema_refs,
        allowed_authorities: descriptor.authority_requirements,
        available_resources: descriptor.resource_requirements,
        expected_determinism: descriptor.determinism,
        expected_replay: descriptor.replay,
        expected_profile: descriptor.implementation_profile,
    };
    let schema = if successor {
        "molten.fixture.native.state.v2"
    } else {
        "molten.fixture.native.state.v1"
    };
    molten::system_extension::SystemExtensionManifestInput {
        schema: molten::system_extension::SYSTEM_EXTENSION_MANIFEST_SCHEMA.to_string(),
        extension_id: "molten.composition.native".to_string(),
        service_id: member.to_string(),
        implementation_ref: context.source.artifact_ref.clone(),
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
        required_ports: vec![requirement],
        optional_ports: Vec::new(),
        capability_refs: vec![context.grant_ref.clone()],
        policy_refs: vec![context.policy_ref.to_string()],
        provenance_refs: vec![context.source.provenance_ref.clone()],
        resources: molten::system_extension::ResourceEnvelope {
            max_concurrent_callbacks: 1,
            max_queued_events: 8,
            max_inflight_bytes: 1_048_576,
            max_open_streams: 1,
            max_timers: 1,
            max_effect_requests: 8,
            callback_deadline_ticks: 100,
            shutdown_grace_ticks: 100,
            max_restart_attempts: 16,
            overload_policy: molten::system_extension::OverloadPolicy::UpstreamBackpressure,
        },
        recovery_group: Some(group.clone()),
        execution_profile: molten::system_extension::ExecutionProfile::NativeProcess,
        state_schema: schema.to_string(),
        compatible_state_schemas: if successor {
            vec!["molten.fixture.native.state.v1".to_string(), schema.to_string()]
        } else {
            vec![schema.to_string()]
        },
        evidence_profile_ref: context.evidence_ref.clone(),
        initial_generation: 1,
        non_claims: molten::system_extension::REQUIRED_SYSTEM_EXTENSION_NON_CLAIMS.to_vec(),
    }
}
