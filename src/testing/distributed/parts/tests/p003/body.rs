    #[test]
    fn ci_metadata_rejects_missing_receipt_ref() {
        // r[verify molten.testing.distributed_ci.profile_wiring_evidence]
        let profile = configured_profile(PROFILE_FAST);
        let mut input = metadata_input(&profile);
        input.receipt_refs = Vec::new();
        let error = build_test_metadata(&input).expect_err("missing receipt refs denied");

        assert!(format!("{error}").contains("distributed metadata requires receipt refs"));
    }

    #[test]
    fn ci_gate_requires_traceability_and_zero_retry_pass() {
        // r[verify molten.testing.distributed_ci.traceability_required_gate]
        // r[verify molten.testing.distributed_ci.retry_policy]
        let matrix = build_ci_matrix(default_ci_profiles()).expect("matrix");
        let traceability = traceability_manifest();
        let metadata = metadata(PROFILE_FAST);
        let mut run = profile_run(PROFILE_FAST, metadata.metadata_ref.clone(), traceability.manifest_ref.clone());
        run.retry_attempts = 1;
        let gate = evaluate_ci_gate(&CiGateInput {
            matrix: &matrix,
            metadata: &[metadata],
            traceability_manifest: &traceability,
            runs: &[run],
        })
        .expect("gate");

        assert_eq!(gate.decision, DENY_DECISION);
        assert!(gate.diagnostics.iter().any(|diagnostic| diagnostic == "retry-only-success-denied:fast"));
    }

    #[test]
    fn ci_gate_rejects_missing_negative_coverage_and_unavailable_pass() {
        // r[verify molten.testing.distributed_ci.unavailable_handling]
        // r[verify molten.testing.distributed_ci.negative_fixtures]
        let matrix = build_ci_matrix(default_ci_profiles()).expect("matrix");
        let traceability = traceability_manifest();
        let metadata = metadata(PROFILE_VM_FAULT);
        let mut run = profile_run(PROFILE_VM_FAULT, metadata.metadata_ref.clone(), traceability.manifest_ref.clone());
        run.negative_coverage = false;
        run.unavailable = true;
        run.required_for_release = true;
        run.unsupported_reason = Some("no-kvm".to_string());
        let gate = evaluate_ci_gate(&CiGateInput {
            matrix: &matrix,
            metadata: &[metadata],
            traceability_manifest: &traceability,
            runs: &[run],
        })
        .expect("gate");

        assert_eq!(gate.decision, DENY_DECISION);
        assert!(gate.diagnostics.iter().any(|diagnostic| diagnostic == "missing-negative-coverage:vm-fault"));
        assert!(gate.diagnostics.iter().any(|diagnostic| diagnostic == "unavailable-profile-cannot-pass:vm-fault"));
        assert!(gate.diagnostics.iter().any(|diagnostic| diagnostic == "required-profile-unavailable:vm-fault"));
    }

    #[test]
    fn ci_matrix_negative_fixture_rejects_missing_profile() {
        // r[verify molten.testing.distributed_ci.negative_fixtures]
        let mut profiles = default_ci_profiles();
        profiles.retain(|profile| profile.id != PROFILE_PROTOCOL);
        let matrix = build_ci_matrix(profiles).expect("matrix");

        assert_eq!(matrix.decision, DENY_DECISION);
        assert!(matrix.diagnostics.iter().any(|diagnostic| diagnostic == "missing-profile:protocol"));
    }
