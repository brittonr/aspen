
    fn assert_stale_verification(case: &Case) {
        // r[verify molten.provenance_state_proof.build_verification_binding]
        let stale_diagnostic = "stale build verification receipt".to_string();
        let stale = verify_build(&BuildVerificationInput {
            build_record_value: &case.build_record,
            actual_artifact_ref: &case.artifact_ref,
            prior_diagnostics: std::slice::from_ref(&stale_diagnostic),
        })
        .expect("verify stale artifact");
        let eval = evaluate(&EvaluationInput {
            operation: OPERATION_INSTALL_PRODUCTION_EXECUTABLE,
            profile: "node-control",
            artifact_ref: &case.artifact_ref,
            provenance_values: std::slice::from_ref(&case.provenance),
            build_verification_values: std::slice::from_ref(&stale.receipt_value),
            prior_diagnostics: &[],
        })
        .expect("stale verification denies");
        assert_eq!(eval.decision, "deny");
        assert!(eval.diagnostics.iter().any(|diagnostic| diagnostic.contains("decision is deny")));
    }

    fn assert_wrong(case: &Case, verification: &BuildVerification) {
        let wrong_binding = wrong_value(case);
        let eval = evaluate(&EvaluationInput {
            operation: "install",
            profile: "node-control",
            artifact_ref: &case.artifact_ref,
            provenance_values: std::slice::from_ref(&wrong_binding),
            build_verification_values: std::slice::from_ref(&verification.receipt_value),
            prior_diagnostics: &[],
        })
        .expect("wrong binding denies");
        assert_eq!(eval.decision, "deny");
        assert!(eval.diagnostics.iter().any(|diagnostic| diagnostic.contains("is not bound by provenance record")));
    }

    #[test]
    fn reproducible_verified_requires_matching_build_verification_evidence() {
        let case = seed();
        assert_missing(&case);
        let verification = assert_match(&case);
        assert_wrong_artifact(&case);
        assert_stale_verification(&case);
        assert_wrong(&case, &verification);
    }

    #[test]
    fn wrong_profile_is_rejected_before_admission() {
        // r[verify molten.provenance_state_proof.profile_thresholds]
        let artifact_ref = synthetic_ref("artifact", "wrong-profile").expect("artifact ref");
        let record = synthetic_reviewed_record(&artifact_ref).expect("record");
        let error = evaluate(&EvaluationInput {
            operation: "install",
            profile: "unknown-profile",
            artifact_ref: &artifact_ref,
            provenance_values: &[record],
            build_verification_values: &[],
            prior_diagnostics: &[],
        })
        .expect_err("wrong profile rejects");
        assert!(error.to_string().contains("invalid provenance evaluation profile"));
    }
