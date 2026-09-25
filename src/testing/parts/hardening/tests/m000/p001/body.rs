
    #[test]
    fn tamper_matrix_denies_pass_evidence_for_negative_case() {
        let family = TamperFamilyInput {
            family: "release-bundle".to_string(),
            control_ref: local_ref("control"),
            parser: "parse_release".to_string(),
            gate: "verify_release".to_string(),
        };
        let mut cases = all_tamper_cases("release-bundle");
        cases[0].decision = DECISION_PASS.to_string();
        cases[0].pass_evidence_ref = Some(local_ref("bad-pass"));
        let matrix = build_tamper_matrix(&TamperMatrixInput {
            subject_ref: local_ref("subject"),
            families: vec![family],
            cases,
        })
        .expect("tamper matrix");
        assert_eq!(matrix.decision, DECISION_DENY);
        assert!(matrix.diagnostics.iter().any(|diagnostic| diagnostic.contains("tamper-case-emits-pass-evidence")));
    }

    #[test]
    fn hegel_counterexample_fixture_binds_replay_identity_and_redacted_input() {
        let fixture = build_hegel_counterexample_fixture(&HegelCounterexampleInput {
            property_id: "property:roundtrip".to_string(),
            requirement_ids: vec![REQUIREMENT_ID.to_string()],
            generator_profile_ref: local_ref("generator"),
            generation_seed: "seed-1".to_string(),
            shrink_path: vec!["remove-field".to_string()],
            shrunk_input_ref: local_ref("input"),
            replay_identity_ref: local_ref("replay"),
            trace_refs: vec![local_ref("trace")],
            receipt_refs: vec![local_ref("receipt")],
            diagnostics: vec!["expected-failure".to_string()],
            confidentiality: "redacted".to_string(),
        })
        .expect("hegel fixture");
        assert_eq!(fixture.decision, DECISION_PASS);
        assert!(fixture.diagnostics.iter().any(|diagnostic| diagnostic == "expected-failure"));
    }

    #[test]
    fn hegel_counterexample_fixture_denies_missing_seed_and_sensitive_export() {
        let fixture = build_hegel_counterexample_fixture(&HegelCounterexampleInput {
            property_id: "property:roundtrip".to_string(),
            requirement_ids: vec![REQUIREMENT_ID.to_string()],
            generator_profile_ref: local_ref("generator"),
            generation_seed: String::new(),
            shrink_path: Vec::new(),
            shrunk_input_ref: local_ref("input"),
            replay_identity_ref: local_ref("replay"),
            trace_refs: Vec::new(),
            receipt_refs: Vec::new(),
            diagnostics: Vec::new(),
            confidentiality: "sensitive".to_string(),
        })
        .expect("hegel fixture");
        assert_eq!(fixture.decision, DECISION_DENY);
        assert!(fixture.diagnostics.iter().any(|diagnostic| diagnostic == "missing-seed"));
        assert!(fixture.diagnostics.iter().any(|diagnostic| diagnostic == "sensitive-input-not-redacted"));
    }

    #[test]
    fn hegel_promotion_requires_reviewed_status() {
        let record = build_hegel_promotion_record(&HegelPromotionInput {
            source_fixture_ref: local_ref("source"),
            new_suite_entry_ref: local_ref("suite"),
            review_ref: local_ref("review"),
            property_id: "property:roundtrip".to_string(),
            reason: "fixed-bug".to_string(),
            status: "regression-pass".to_string(),
        })
        .expect("promotion");
        assert_eq!(record.decision, DECISION_PASS);
    }

    #[test]
    fn replay_smoke_gate_passes_stable_run_replay_fresh_refs() {
        let runs = vec![
            replay_run("fresh", "same"),
            replay_run("replay", "same"),
            replay_run("fresh-rerun", "same"),
        ];
        let gate = build_replay_smoke_gate(&ReplaySmokeInput {
            suite_id: "suite:deterministic".to_string(),
            eligibility: "deterministic".to_string(),
            runs,
            variance: Vec::new(),
            diagnostic_caveats: Vec::new(),
        })
        .expect("replay smoke");
        assert_eq!(gate.decision, DECISION_PASS);
    }

    #[test]
    fn replay_smoke_gate_denies_changed_effect_response_and_non_replayable_pass_misuse() {
        let mut replay = replay_run("replay", "same");
        replay.effect_log_ref = local_ref("changed-effect");
        let gate = build_replay_smoke_gate(&ReplaySmokeInput {
            suite_id: "suite:deterministic".to_string(),
            eligibility: "deterministic".to_string(),
            runs: vec![replay_run("fresh", "same"), replay, replay_run("fresh-rerun", "same")],
            variance: Vec::new(),
            diagnostic_caveats: Vec::new(),
        })
        .expect("replay smoke");
        assert_eq!(gate.decision, DECISION_DENY);
        assert!(gate.diagnostics.iter().any(|diagnostic| diagnostic == "effect-log-ref-mismatch:replay"));
        let non_replayable = build_replay_smoke_gate(&ReplaySmokeInput {
            suite_id: "suite:live".to_string(),
            eligibility: "live-only".to_string(),
            runs: Vec::new(),
            variance: Vec::new(),
            diagnostic_caveats: Vec::new(),
        })
        .expect("non replayable smoke");
        assert_eq!(non_replayable.decision, DECISION_DENY);
        assert!(
            non_replayable
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "non-replayable-without-diagnostic")
        );
    }

    #[test]
    fn nextest_profile_matrix_accepts_semantic_profiles() {
        let matrix = build_nextest_profile_matrix(&NextestProfileMatrixInput {
            profiles: reviewed_nextest_profile_rows()
                .into_iter()
                .filter(|profile| required_semantic_profiles().contains(&profile.profile_id.as_str()))
                .collect(),
        })
        .expect("profile matrix");
        assert_eq!(matrix.decision, DECISION_PASS);
    }

    #[test]
    fn nextest_profile_matrix_denies_missing_profile_and_unavailable_platform() {
        let mut profile = semantic_profile("vm-platform", "vm", "platform");
        profile.platform_required = true;
        profile.platform_available = false;
        let matrix = build_nextest_profile_matrix(&NextestProfileMatrixInput {
            profiles: vec![profile],
        })
        .expect("profile matrix");
        assert_eq!(matrix.decision, DECISION_DENY);
        assert!(
            matrix
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "required-platform-unavailable:vm-platform")
        );
        assert!(matrix.diagnostics.iter().any(|diagnostic| diagnostic == "missing-profile:fast-core"));
    }

    #[test]
    fn nextest_profile_matrix_denies_missing_filter_duplicate_and_junit_only() {
        let mut first = semantic_profile("fast-core", "unit", "fast");
        first.filter_expression.clear();
        first.expected_artifacts = vec![JUNIT_ARTIFACT.to_string()];
        let duplicate = first.clone();
        let matrix = build_nextest_profile_matrix(&NextestProfileMatrixInput {
            profiles: vec![
                first,
                duplicate,
                semantic_profile("harness", "integration", "moderate"),
                semantic_profile("cli", "cli", "moderate"),
                semantic_profile("distributed-simulation", "integration", "moderate"),
                semantic_profile("vm-platform", "vm", "platform"),
                semantic_profile("dogfood-soak", "dogfood", "soak"),
            ],
        })
        .expect("profile matrix");
        assert_eq!(matrix.decision, DECISION_DENY);
        assert!(matrix.diagnostics.iter().any(|diagnostic| diagnostic == "missing-filter:fast-core"));
        assert!(matrix.diagnostics.iter().any(|diagnostic| diagnostic == "duplicate-profile:fast-core"));
        assert!(
            matrix
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "junit-without-canonical-test-run:fast-core")
        );
    }

    #[test]
    fn nextest_profile_matrix_denies_deterministic_live_mixing_and_bad_retry() {
        let mut profile = semantic_profile("cli", "cli", "moderate");
        profile.filter_expression = NEXTEST_ALL_FILTER.to_string();
        profile.excluded_partitions = vec!["live-only".to_string()];
        profile.retry_policy = "retry-pass".to_string();
        let matrix = build_nextest_profile_matrix(&NextestProfileMatrixInput {
            profiles: vec![
                semantic_profile("fast-core", "unit", "fast"),
                semantic_profile("harness", "integration", "moderate"),
                profile,
                semantic_profile("distributed-simulation", "integration", "moderate"),
                semantic_profile("vm-platform", "vm", "platform"),
                semantic_profile("dogfood-soak", "dogfood", "soak"),
            ],
        })
        .expect("profile matrix");
        assert_eq!(matrix.decision, DECISION_DENY);
        assert!(matrix.diagnostics.iter().any(|diagnostic| diagnostic == "unpartitioned-filter:cli"));
        assert!(
            matrix
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "missing-deterministic-exclusion:cli:vm-only")
        );
        assert!(matrix.diagnostics.iter().any(|diagnostic| diagnostic == "retry-pass-not-deterministic:cli"));
    }

    #[test]
    fn cli_receipt_first_gate_accepts_canonical_receipt_assertion() {
        let gate = build_cli_receipt_first_gate(&CliReceiptFirstInput {
            command: "molten test gate check".to_string(),
            evidence_bearing: true,
            canonical_artifact_refs: vec![local_ref("gate-receipt")],
            rendered_output_kinds: vec!["stdout".to_string()],
            negative_case: false,
            failure_artifact_ref: None,
            diagnostics: Vec::new(),
        })
        .expect("cli gate");
        assert_eq!(gate.decision, DECISION_PASS);
    }

    #[test]
    fn cli_receipt_first_gate_denies_stdout_only_and_missing_negative_artifact() {
        let gate = build_cli_receipt_first_gate(&CliReceiptFirstInput {
            command: "molten test gate check".to_string(),
            evidence_bearing: true,
            canonical_artifact_refs: Vec::new(),
            rendered_output_kinds: vec!["stdout".to_string()],
            negative_case: true,
            failure_artifact_ref: None,
            diagnostics: Vec::new(),
        })
        .expect("cli gate");
        assert_eq!(gate.decision, DECISION_DENY);
        assert!(gate.diagnostics.iter().any(|diagnostic| diagnostic == "missing-canonical-artifact"));
        assert!(gate.diagnostics.iter().any(|diagnostic| diagnostic == "missing-negative-failure-artifact"));
    }
