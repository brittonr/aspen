
fn reference_matrix_suite_value(summary: &ReferenceMatrixSummary) -> preserves::IOValue {
    let matrices = summary.matrices.iter().map(reference_matrix_value).collect::<Vec<_>>();
    crate::preserves_rail::record("fabric-reference-matrix-suite-v1", vec![
        crate::preserves_rail::string(FABRIC_REFERENCE_MATRIX_SUITE_SCHEMA),
        field("matrices", crate::preserves_rail::sequence(matrices)),
        checks_value(&[
            "three-reference-classes",
            "ports-not-ambient-access",
            "semantics-extension-owned",
            "conformance-not-correctness-proof",
        ]),
    ])
}

fn reference_matrix_value(matrix: &ReferenceSystemMatrix) -> preserves::IOValue {
    let semantics = matrix
        .semantics
        .iter()
        .map(|ownership| {
            crate::preserves_rail::record("semantic-ownership", vec![
                crate::preserves_rail::string(ownership.semantic.as_str()),
                crate::preserves_rail::string(ownership.owner.as_str()),
            ])
        })
        .collect::<Vec<_>>();
    crate::preserves_rail::record("fabric-reference-matrix-v1", vec![
        crate::preserves_rail::string(FABRIC_REFERENCE_MATRIX_SCHEMA),
        field("system", crate::preserves_rail::string(matrix.system.as_str())),
        field("capabilities", strings_value(matrix.capabilities.iter().map(|capability| capability.as_str()))),
        field("semantics", crate::preserves_rail::sequence(semantics)),
        field("ambient-accesses", strings_value(matrix.ambient_accesses.iter().map(String::as_str))),
        field("non-claims", strings_value(matrix.non_claims.iter().map(|non_claim| non_claim.as_str()))),
    ])
}

fn field(label: &'static str, value: preserves::IOValue) -> preserves::IOValue {
    crate::preserves_rail::record(label, vec![value])
}

fn strings_value<'a>(values: impl IntoIterator<Item = &'a str>) -> preserves::IOValue {
    crate::preserves_rail::sequence(values.into_iter().map(crate::preserves_rail::string).collect())
}

fn optional_string_value(value: Option<&str>) -> preserves::IOValue {
    match value {
        Some(value) => crate::preserves_rail::record("some", vec![crate::preserves_rail::string(value)]),
        None => crate::preserves_rail::record("none", Vec::new()),
    }
}

fn checks_value(checks: &[&str]) -> preserves::IOValue {
    field("checks", strings_value(checks.iter().copied()))
}

fn validation_error(label: &str, issues: &impl std::fmt::Debug) -> crate::error::MoltenError {
    crate::error::MoltenError::invalid_harness(format!("{label} validation denied: {issues:?}"))
}

// r[impl molten.fabric_boundary.final_validation]
#[cfg(test)]
mod tests {
    use super::*;

    const CONFORMANCE_REF: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const LIMIT_REF: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const PORT_ID: &str = "molten.fabric.transport.session";
    const PORT_VERSION: &str = "v1";
    const PORT_PROFILE: &str = "iroh-live-v1";
    const PORT_OPERATION: &str = "send-envelope";
    const INPUT_SCHEMA: &str = "molten.fabric.transport-send.v1";
    const OUTPUT_SCHEMA: &str = "molten.fabric.transport-outcome.v1";

    fn descriptor() -> FabricPortDescriptor {
        FabricPortDescriptor {
            schema: FABRIC_PORT_DESCRIPTOR_SCHEMA.to_string(),
            port_id: PORT_ID.to_string(),
            version: PORT_VERSION.to_string(),
            class: FabricPortClass::Transport,
            operation_classes: vec![PORT_OPERATION.to_string()],
            input_schema_refs: vec![INPUT_SCHEMA.to_string()],
            output_schema_refs: vec![OUTPUT_SCHEMA.to_string()],
            authority_requirements: vec![FabricAuthority::Transport],
            resource_requirements: vec![FabricResource::NetworkBytes],
            determinism: DeterminismClass::ExternalEffect,
            replay: ReplayClass::RecordedEffectRequired,
            implementation_profile: PORT_PROFILE.to_string(),
            conformance_refs: vec![CONFORMANCE_REF.to_string()],
            non_claims: REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
            enabled: true,
        }
    }

    fn requirement() -> FabricPortRequirement {
        FabricPortRequirement {
            port_id: PORT_ID.to_string(),
            version: PORT_VERSION.to_string(),
            class: FabricPortClass::Transport,
            operation_classes: vec![PORT_OPERATION.to_string()],
            input_schema_refs: vec![INPUT_SCHEMA.to_string()],
            output_schema_refs: vec![OUTPUT_SCHEMA.to_string()],
            allowed_authorities: vec![FabricAuthority::Transport],
            available_resources: vec![FabricResource::NetworkBytes],
            expected_determinism: DeterminismClass::ExternalEffect,
            expected_replay: ReplayClass::RecordedEffectRequired,
            expected_profile: PORT_PROFILE.to_string(),
        }
    }

    // r[verify molten.fabric_boundary.port_registry]
    // r[verify molten.fabric_boundary.final_validation]
    #[test]
    fn canonical_port_binding_is_stable_and_names_only_reviewed_profile() {
        let first = resolve_canonical_fabric_port_binding(&[descriptor()], &requirement())
            .expect("canonical compatible binding");
        let second =
            resolve_canonical_fabric_port_binding(&[descriptor()], &requirement()).expect("repeat canonical binding");
        let text = crate::preserves_rail::to_text(&first.binding_value).expect("binding text");

        assert_eq!(first.descriptor_ref, second.descriptor_ref);
        assert_eq!(first.registry_ref, second.registry_ref);
        assert_eq!(first.binding_ref, second.binding_ref);
        assert!(first.binding_ref.starts_with("blake3:"));
        assert!(text.contains("fabric-port-binding-v1"));
        assert!(text.contains(PORT_PROFILE));
        assert!(text.contains("binding-is-not-behavioral-proof"));
        assert!(!text.contains("redb::"));
        assert!(!text.contains("iroh::Endpoint"));
    }

    // r[verify molten.fabric_boundary.fabric_identity]
    // r[verify molten.fabric_boundary.evidence_granularity]
    // r[verify molten.fabric_boundary.reference_system_exit_criteria]
    // r[verify molten.fabric_boundary.non_claims]
    // r[verify molten.fabric_boundary.final_validation]
    #[test]
    fn canonical_fabric_reports_bind_non_claims_and_reference_scope() {
        let boundary = canonical_fabric_boundary(&default_fabric_boundary_descriptor()).expect("boundary artifact");
        let evidence = canonical_fabric_evidence_profile(&default_production_evidence_profile(LIMIT_REF))
            .expect("evidence artifact");
        let references =
            canonical_reference_matrix_suite(&default_reference_system_matrices()).expect("reference artifact");
        let boundary_text = crate::preserves_rail::to_text(&boundary.value).expect("boundary text");
        let reference_text = crate::preserves_rail::to_text(&references.value).expect("reference text");

        assert!(boundary.boundary_ref.starts_with("blake3:"));
        assert!(evidence.profile_ref.starts_with("blake3:"));
        assert!(references.suite_ref.starts_with("blake3:"));
        assert!(boundary_text.contains("workload-neutral-distributed-systems-fabric"));
        assert!(boundary_text.contains("does-not-prove-global-consensus"));
        assert!(reference_text.contains("transactional-key-value"));
        assert!(reference_text.contains("replicated-log"));
        assert!(reference_text.contains("distributed-scheduler"));
        assert!(reference_text.contains("system-extension"));
    }

    // r[verify molten.fabric_boundary.extension_tiers]
    // r[verify molten.fabric_boundary.final_validation]
    #[test]
    fn canonical_tier_admission_denies_plugin_system_authority() {
        let request = ExtensionTierRequest {
            tier: ExtensionTier::SandboxedPlugin,
            requested_authorities: vec![FabricAuthority::Consistency],
            admission_evidence: Vec::new(),
        };

        let error = canonical_extension_tier_admission(&request).expect_err("plugin authority must deny");

        assert!(error.to_string().contains("AuthorityRequiresSystemExtension"));
    }

    // r[verify molten.fabric_boundary.port_registry]
    // r[verify molten.fabric_boundary.final_validation]
    #[test]
    fn canonical_binding_denies_silent_profile_substitution() {
        let mut request = requirement();
        request.expected_profile = "different-profile-v1".to_string();

        let error = resolve_canonical_fabric_port_binding(&[descriptor()], &request)
            .expect_err("profile substitution must deny");

        assert!(error.to_string().contains("SilentProfileSubstitution"));
    }

    // r[verify molten.fabric_boundary.port_registry]
    // r[verify molten.fabric_boundary.final_validation]
    #[test]
    fn canonical_port_identity_is_independent_of_set_and_registry_input_order() {
        let mut ordered = descriptor();
        ordered.operation_classes.push("receive-envelope".to_string());
        ordered.resource_requirements.push(FabricResource::Concurrency);
        let mut reordered = ordered.clone();
        reordered.operation_classes.reverse();
        reordered.resource_requirements.reverse();

        let ordered_ref = canonical_fabric_port_descriptor(&ordered).expect("ordered descriptor").0;
        let reordered_ref = canonical_fabric_port_descriptor(&reordered).expect("reordered descriptor").0;
        assert_eq!(ordered_ref, reordered_ref);

        let mut time_descriptor = descriptor();
        time_descriptor.port_id = "molten.fabric.time.logical".to_string();
        time_descriptor.class = FabricPortClass::Time;
        time_descriptor.operation_classes = vec!["schedule-logical-deadline".to_string()];
        time_descriptor.input_schema_refs = vec!["molten.fabric.time-schedule.v1".to_string()];
        time_descriptor.output_schema_refs = vec!["molten.fabric.time-event.v1".to_string()];
        time_descriptor.authority_requirements = vec![FabricAuthority::Time];
        time_descriptor.resource_requirements = vec![FabricResource::LogicalTime];
        time_descriptor.implementation_profile = "logical-time-v1".to_string();

        let forward = resolve_canonical_fabric_port_binding(&[descriptor(), time_descriptor.clone()], &requirement())
            .expect("forward registry");
        let reversed = resolve_canonical_fabric_port_binding(&[time_descriptor, descriptor()], &requirement())
            .expect("reversed registry");

        assert_eq!(forward.registry_ref, reversed.registry_ref);
        assert_eq!(forward.binding_ref, reversed.binding_ref);
    }
}
