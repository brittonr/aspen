
#[cfg(test)]
mod tests {

    use super::*;

    const SEMANTIC_MAPPING_SCHEMA: &str = "molten.semantic-operation-mapping.v1";
    const ADOPTION_KIND_COUNT: usize = 9;

    #[derive(serde::Deserialize)]
    struct SemanticMappingFixture {
        schema: String,
        kamacite_revision: String,
        operation_hex: String,
        surface_hexes: Vec<String>,
        expected: String,
    }

    const KINDS: [AdoptionArtifactKind; ADOPTION_KIND_COUNT] = [
        AdoptionArtifactKind::BindingRecord,
        AdoptionArtifactKind::BindingSnapshot,
        AdoptionArtifactKind::ResolutionReceipt,
        AdoptionArtifactKind::TransitionReceipt,
        AdoptionArtifactKind::RootInventory,
        AdoptionArtifactKind::GenerationAttribution,
        AdoptionArtifactKind::RetirementReport,
        AdoptionArtifactKind::DeployDiagnostic,
        AdoptionArtifactKind::SemanticOperationBinding,
    ];

    #[test]
    fn every_adoption_artifact_has_deterministic_preserves_roundtrip() {
        for kind in KINDS {
            let built = build_adoption_artifact(
                kind,
                &[
                    CanonicalField {
                        name: "snapshot".to_string(),
                        value: "blake3:snapshot".to_string(),
                    },
                    CanonicalField {
                        name: "subject".to_string(),
                        value: "blake3:subject".to_string(),
                    },
                ],
                &["artifact is evidence, not authority".to_string()],
            )
            .expect("build adoption artifact");
            let parsed = parse_adoption_artifact(kind, &built.value).expect("parse adoption artifact");
            assert_eq!(parsed.artifact_ref, built.artifact_ref);
            assert_eq!(parsed.fields, built.fields);
        }
    }

    fn semantic_surfaces(identity: &kamacite_core::Identity) -> molten_core::live_binding::SemanticSurfaceBindings {
        molten_core::live_binding::SemanticSurfaceBindings {
            manifest: identity.clone(),
            handler_binding: identity.clone(),
            handle: identity.clone(),
            request: identity.clone(),
            response: identity.clone(),
            effect_log: identity.clone(),
            adapter_import: identity.clone(),
            remote_execution: identity.clone(),
            runtime_receipt: identity.clone(),
            replay_identity: identity.clone(),
            evaluation_cache_key: identity.clone(),
            job: identity.clone(),
            upgrade_check: identity.clone(),
        }
    }

    #[test]
    fn strict_semantic_binding_is_canonical_and_rejects_drift() {
        let operation =
            kamacite_core::compute_identity(kamacite_core::IdentityDomain::EffectOperation, b"strict-operation");
        let exact = semantic_surfaces(&operation);
        let artifact =
            build_strict_semantic_operation_binding(&operation, &exact).expect("build strict semantic binding");
        let parsed = parse_adoption_artifact(AdoptionArtifactKind::SemanticOperationBinding, &artifact.value)
            .expect("parse strict semantic binding");
        assert_eq!(parsed.artifact_ref, artifact.artifact_ref);

        let mut drifted = exact;
        drifted.handler_binding =
            kamacite_core::compute_identity(kamacite_core::IdentityDomain::EffectOperation, b"drifted-operation");
        assert!(build_strict_semantic_operation_binding(&operation, &drifted).is_err());
    }

    #[test]
    fn governed_semantic_mapping_fixtures_cover_exact_and_drift_cases() {
        let positive: SemanticMappingFixture =
            serde_json::from_str(include_str!("../../../../fixtures/semantic-operation/mapping-default.json"))
                .expect("parse positive semantic mapping fixture");
        assert_eq!(positive.schema, SEMANTIC_MAPPING_SCHEMA);
        assert_eq!(positive.kamacite_revision, molten_core::live_binding::KAMACITE_SEMANTIC_REVISION);
        assert_eq!(positive.surface_hexes.len(), molten_core::live_binding::SEMANTIC_SURFACE_COUNT);
        assert!(positive.surface_hexes.iter().all(|identity| identity == &positive.operation_hex));
        assert_eq!(positive.expected, "pass");

        let negative: SemanticMappingFixture =
            serde_json::from_str(include_str!("../../../../fixtures/semantic-operation/mapping-drift.json"))
                .expect("parse negative semantic mapping fixture");
        assert!(negative.surface_hexes.iter().any(|identity| identity != &negative.operation_hex));
        assert_eq!(negative.expected, "deny");
    }

    #[test]
    fn duplicate_fields_and_wrong_artifact_kind_fail_closed() {
        let duplicate = CanonicalField {
            name: "snapshot".to_string(),
            value: "blake3:snapshot".to_string(),
        };
        assert!(
            build_adoption_artifact(AdoptionArtifactKind::BindingRecord, &[duplicate.clone(), duplicate], &[
                "not authority".to_string()
            ],)
            .is_err()
        );
        let built = build_adoption_artifact(
            AdoptionArtifactKind::BindingRecord,
            &[CanonicalField {
                name: "snapshot".to_string(),
                value: "blake3:snapshot".to_string(),
            }],
            &["not authority".to_string()],
        )
        .expect("build binding artifact");
        assert!(parse_adoption_artifact(AdoptionArtifactKind::RetirementReport, &built.value,).is_err());
    }
}
