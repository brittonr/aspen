    // Layer-graph composition and fail-closed aggregate coverage tests.

    fn layer_map(layers: &[ProofLayerInput]) -> OrderedMap<String, &ProofLayerInput> {
        layers.iter().map(|layer| (layer.id.clone(), layer)).collect()
    }

    #[test]
    fn layered_proof_accepts_acyclic_diamond_with_shared_descendants() {
        // r[verify molten.evidence_gates.layered_proof_sound_composition]
        let subject = local_ref("diamond-subject");
        let layers = vec![
            proof_layer("core", "pure-core", &subject, Vec::new()),
            proof_layer("gate", "gate", &subject, vec!["core".to_string()]),
            proof_layer("replay", "replay", &subject, vec!["gate".to_string(), "core".to_string()]),
            proof_layer("release", "release", &subject, vec![
                "replay".to_string(),
                "gate".to_string(),
                "core".to_string(),
            ]),
        ];
        assert!(layer_cycles(&layer_map(&layers)).expect("cycle scan").is_empty());
        let manifest = build_layered_proof_manifest(&LayeredProofInput {
            subject_ref: subject,
            layers,
        })
        .expect("diamond layered proof");
        assert_eq!(manifest.decision, "pass", "{:?}", manifest.diagnostics);
        assert!(manifest.diagnostics.is_empty());
    }

    #[test]
    fn layered_proof_reports_only_layers_on_a_true_cycle() {
        // r[verify molten.evidence_gates.layered_proof_sound_composition]
        let subject = local_ref("cycle-subject");
        let layers = vec![
            proof_layer("release", "release", &subject, vec!["replay".to_string()]),
            proof_layer("replay", "replay", &subject, vec!["gate-a".to_string()]),
            proof_layer("gate-a", "gate", &subject, vec!["gate-b".to_string()]),
            proof_layer("gate-b", "gate", &subject, vec!["gate-a".to_string()]),
            proof_layer("loop", "gate", &subject, vec!["loop".to_string()]),
        ];
        assert_eq!(layer_cycles(&layer_map(&layers)).expect("cycle scan"), vec![
            "gate-a".to_string(),
            "gate-b".to_string(),
            "loop".to_string(),
        ]);
        let manifest = build_layered_proof_manifest(&LayeredProofInput {
            subject_ref: subject,
            layers,
        })
        .expect("cyclic layered proof");
        assert_eq!(manifest.decision, "deny");
        for expected in ["cycle:gate-a", "cycle:gate-b", "cycle:loop"] {
            assert!(manifest.diagnostics.iter().any(|diagnostic| diagnostic == expected), "{expected}");
        }
        assert!(!manifest.diagnostics.iter().any(|diagnostic| diagnostic == "cycle:release"));
    }

    #[test]
    fn passing_layers_binding_a_denied_child_are_denied_transitively() {
        // r[verify molten.evidence_gates.layered_proof_sound_composition]
        let subject = local_ref("denied-child-subject");
        let mut core = proof_layer("core", "pure-core", &subject, Vec::new());
        core.decision = "deny".to_string();
        let manifest = build_layered_proof_manifest(&LayeredProofInput {
            subject_ref: subject.clone(),
            layers: vec![
                core.clone(),
                proof_layer("gate", "gate", &subject, vec!["core".to_string()]),
                proof_layer("replay", "replay", &subject, vec!["gate".to_string()]),
                proof_layer("release", "release", &subject, vec!["replay".to_string()]),
            ],
        })
        .expect("denied child layered proof");
        assert_eq!(manifest.decision, "deny");
        assert_eq!(manifest.diagnostics, vec![
            "denied-child-layer:gate:core".to_string(),
            "denied-child-layer:release:replay".to_string(),
            "denied-child-layer:replay:gate".to_string(),
        ]);

        let mut denying_gate = proof_layer("gate", "gate", &subject, vec!["core".to_string()]);
        denying_gate.decision = "deny".to_string();
        let honest = build_layered_proof_manifest(&LayeredProofInput {
            subject_ref: subject,
            layers: vec![core, denying_gate],
        })
        .expect("honest denied layered proof");
        assert!(honest.diagnostics.is_empty(), "{:?}", honest.diagnostics);
    }

    #[test]
    fn aggregate_coverage_extraction_fails_closed_on_denied_manifests() {
        // r[verify molten.evidence_gates.layered_proof_sound_composition]
        let subject = local_ref("coverage-subject");
        let obligations = vec![
            proof_obligation("validation", "input-validation", &subject, "pass"),
            proof_obligation("negative", "fail-closed-negative", &subject, "deny"),
        ];
        let passing = build_aggregate_proof_manifest(&AggregateProofInput {
            manifest_id: "proof:coverage".to_string(),
            subject_ref: subject.clone(),
            required_obligation_ids: vec!["validation".to_string(), "negative".to_string()],
            obligations: obligations.clone(),
        })
        .expect("passing aggregate proof");
        let coverage = coverage_from_aggregate_proof(&passing, true).expect("coverage from passing manifest");
        assert_eq!(coverage.len(), 1);
        assert_eq!(coverage[0].requirement_id, REQUIREMENT_ID);
        assert_eq!((coverage[0].positive.len(), coverage[0].negative.len()), (1, 1));

        let denied = build_aggregate_proof_manifest(&AggregateProofInput {
            manifest_id: "proof:denied".to_string(),
            subject_ref: subject,
            required_obligation_ids: vec!["validation".to_string(), "negative".to_string(), "replay".to_string()],
            obligations,
        })
        .expect("denied aggregate proof");
        assert_eq!(denied.decision, "deny");
        let mut forged = denied.clone();
        forged.decision = "pass".to_string();
        for manifest in [&denied, &forged] {
            match coverage_from_aggregate_proof(manifest, true) {
                Err(MoltenError::HarnessDivergence(divergence)) => {
                    assert_eq!(divergence.kind, "aggregate-proof-denied");
                    assert!(divergence.detail.contains("missing-child:replay"), "{}", divergence.detail);
                }
                other => panic!("denied aggregate manifest yielded {other:?}"),
            }
        }
    }
