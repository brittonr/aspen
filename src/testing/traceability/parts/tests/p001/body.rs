    #[test]
    fn verification_run_receipts_derive_receipt_backed_coverage() {
        let positive = build_verification_run_receipt(&receipt_input(REQUIREMENT_ID, "positive", "positive", 0))
            .expect("positive receipt");
        let negative = build_verification_run_receipt(&receipt_input(REQUIREMENT_ID, "negative", "negative", 1))
            .expect("negative receipt");
        let coverage = coverage_from_verification_receipts(&[
            ReceiptCoverageSource {
                value: positive.value,
                target_exists: true,
            },
            ReceiptCoverageSource {
                value: negative.value,
                target_exists: true,
            },
        ])
        .expect("receipt coverage");
        let manifest = build_traceability_manifest(&TraceabilityInput {
            requirements: vec![requirement(REQUIREMENT_ID, "evidence", true)],
            coverage,
            require_receipt_backed: true,
        })
        .expect("receipt-backed manifest");
        assert_eq!(manifest.decision, "pass");
        assert!(manifest.summary.compatibility_only.is_empty());
    }

    #[test]
    fn receipt_backed_policy_denies_raw_compatibility_tuples() {
        let manifest = build_traceability_manifest(&TraceabilityInput {
            requirements: vec![requirement(REQUIREMENT_ID, "evidence", true)],
            coverage: vec![CoverageInput {
                requirement_id: REQUIREMENT_ID.to_string(),
                positive: vec![evidence("positive")],
                negative: vec![evidence("negative")],
                exemption: None,
            }],
            require_receipt_backed: true,
        })
        .expect("manifest");
        assert_eq!(manifest.decision, "deny");
        assert_eq!(manifest.summary.stale_reference, vec![REQUIREMENT_ID.to_string()]);
    }

    #[test]
    fn verification_receipt_denies_wrong_exit_for_negative_coverage() {
        let receipt = build_verification_run_receipt(&receipt_input(REQUIREMENT_ID, "negative", "bad-negative", 0))
            .expect("negative receipt");
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic == "negative-run-did-not-deny"));
    }

    #[test]
    fn aggregate_proof_requires_all_children_and_subjects() {
        let subject = local_ref("subject");
        let manifest = build_aggregate_proof_manifest(&AggregateProofInput {
            manifest_id: "proof:complete".to_string(),
            subject_ref: subject.clone(),
            required_obligation_ids: vec!["validation".to_string(), "negative".to_string()],
            obligations: vec![
                proof_obligation("validation", "input-validation", &subject, "pass"),
                proof_obligation("negative", "fail-closed-negative", &subject, "deny"),
            ],
        })
        .expect("aggregate proof");
        assert_eq!(manifest.decision, "pass");
        let stale = build_aggregate_proof_manifest(&AggregateProofInput {
            manifest_id: "proof:stale".to_string(),
            subject_ref: subject,
            required_obligation_ids: vec!["validation".to_string(), "negative".to_string(), "replay".to_string()],
            obligations: manifest.obligations,
        })
        .expect("stale aggregate proof");
        assert_eq!(stale.decision, "deny");
        assert!(stale.diagnostics.iter().any(|diagnostic| diagnostic == "missing-child:replay"));
    }

    #[test]
    fn layered_proof_denies_cycles_and_readback_pass_promotion() {
        let subject = local_ref("layered-subject");
        let pass = build_layered_proof_manifest(&LayeredProofInput {
            subject_ref: subject.clone(),
            layers: vec![
                proof_layer("core", "pure-core", &subject, Vec::new()),
                proof_layer("gate", "gate", &subject, vec!["core".to_string()]),
                proof_layer("replay", "replay", &subject, vec!["gate".to_string()]),
                proof_layer("release", "release", &subject, vec!["replay".to_string()]),
            ],
        })
        .expect("layered proof");
        assert_eq!(pass.decision, "pass");
        let mut readback = proof_layer("readback", "operator-readback", &subject, vec!["release".to_string()]);
        readback.decision = "pass".to_string();
        let deny = build_layered_proof_manifest(&LayeredProofInput {
            subject_ref: subject,
            layers: vec![readback],
        })
        .expect("diagnostic layer proof");
        assert_eq!(deny.decision, "deny");
        assert!(deny.diagnostics.iter().any(|diagnostic| diagnostic.contains("diagnostic-readback-used-as-pass")));
    }

    #[test]
    fn deny_path_matrix_requires_no_mutation_and_all_classes() {
        let mut cases = required_deny_classes().iter().map(|class| deny_case(class)).collect::<Vec<_>>();
        let matrix = build_deny_path_matrix(&DenyPathMatrixInput {
            gate: "traceability".to_string(),
            subject_ref: local_ref("deny-subject"),
            cases: cases.clone(),
        })
        .expect("deny matrix");
        assert_eq!(matrix.decision, "pass");
        cases.retain(|case| case.class != "diagnostic-only-not-pass");
        let missing = build_deny_path_matrix(&DenyPathMatrixInput {
            gate: "traceability".to_string(),
            subject_ref: local_ref("deny-subject"),
            cases,
        })
        .expect("missing class matrix");
        assert_eq!(missing.decision, "deny");
        assert!(
            missing
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "missing-deny-class:diagnostic-only-not-pass")
        );
    }

