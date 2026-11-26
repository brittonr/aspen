    #[test]
    fn ci_test_run_receipt_binds_metadata_and_counts() {
        let receipt = build_ci_test_run_receipt(&CiTestRunInput {
            source_ref: local_ref("source"),
            profile_id: "ci".to_string(),
            command_surface: "cargo nextest run --profile ci".to_string(),
            nextest_config_ref: local_ref("nextest"),
            cargo_metadata_ref: local_ref("cargo"),
            binaries_metadata_ref: local_ref("binaries"),
            junit_ref: local_ref("junit"),
            counts: CiTestCounts {
                total: PASS_COUNT,
                passed: PASS_COUNT,
                failed: SKIP_COUNT,
                skipped: SKIP_COUNT,
            },
            decision: DECISION_PASS.to_string(),
            diagnostics: Vec::new(),
            caveats: vec![EVIDENCE_ONLY_CAVEAT.to_string()],
        })
        .expect("ci receipt");
        assert_eq!(receipt.decision, DECISION_PASS);
        assert!(receipt.diagnostics.is_empty());
    }

    #[test]
    fn ci_test_run_receipt_denies_mismatched_counts_and_exploratory_pass() {
        let receipt = build_ci_test_run_receipt(&CiTestRunInput {
            source_ref: local_ref("source"),
            profile_id: "exploratory".to_string(),
            command_surface: "cargo nextest run --profile exploratory".to_string(),
            nextest_config_ref: local_ref("nextest"),
            cargo_metadata_ref: local_ref("cargo"),
            binaries_metadata_ref: local_ref("binaries"),
            junit_ref: local_ref("junit"),
            counts: CiTestCounts {
                total: PASS_COUNT,
                passed: PASS_COUNT,
                failed: FAIL_COUNT,
                skipped: SKIP_COUNT,
            },
            decision: DECISION_PASS.to_string(),
            diagnostics: Vec::new(),
            caveats: Vec::new(),
        })
        .expect("ci receipt");
        assert_eq!(receipt.decision, DECISION_DENY);
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic == "mismatched-counts"));
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic == "exploratory-pass-is-diagnostic-only"));
    }

    #[test]
    fn tamper_matrix_requires_generated_negative_cases() {
        let family = TamperFamilyInput {
            family: "harness-report".to_string(),
            control_ref: local_ref("control"),
            parser: "parse_report".to_string(),
            gate: "gate-report".to_string(),
        };
        let matrix = build_tamper_matrix(&TamperMatrixInput {
            subject_ref: local_ref("subject"),
            families: vec![family],
            cases: all_tamper_cases("harness-report"),
        })
        .expect("tamper matrix");
        assert_eq!(matrix.decision, DECISION_PASS);
        assert_eq!(matrix.generated_cases.len(), required_tamper_mutations().len());
    }
