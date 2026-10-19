
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

// r[verify molten.provenance_state_proof.direct_binding_ref_envelope]
    #[test]
    fn direct_build_binding_preflights_existing_ref_envelope() {
        let case = seed();
        let verification = verify_build(&BuildVerificationInput {
            build_record_value: &case.build_record,
            actual_artifact_ref: &case.artifact_ref,
            prior_diagnostics: &[],
        })
        .expect("valid build verification");
        let record = parse_record(&case.provenance).expect("parsed reproducible record");
        let passing = parse_build_verification_receipt(&verification.receipt_value).expect("parsed passing receipt");
        let mut receipts = vec![passing.clone(); MAX_PROVENANCE_REFS];
        let admitted = evaluate_build_verification_binding(&record, &case.artifact_ref, &receipts);
        assert!(admitted.is_bound, "exact maximum preserves valid binding");

        let mut forged_record = record.clone();
        forged_record
            .build_record_refs
            .resize(MAX_PROVENANCE_REFS + 1, verification.build_record_ref.clone());
        let denied_refs = evaluate_build_verification_binding(&forged_record, &case.artifact_ref, &receipts);
        assert!(!denied_refs.is_bound, "forged record refs exceed the same source-owned envelope");
        assert_eq!(
            denied_refs.diagnostics,
            vec!["invalid harness artifact: too many provenance build record ref: 65 > 64"]
        );

        receipts.push(passing);
        let denied = evaluate_build_verification_binding(&record, &case.artifact_ref, &receipts);
        assert!(!denied.is_bound, "over-limit receipts cannot hide behind an early passing receipt");
        assert_eq!(denied.matched_receipt_ref, None);
        assert_eq!(
            denied.diagnostics,
            vec!["invalid harness artifact: too many provenance build verification receipts: 65 > 64"]
        );

        for receipt in &mut receipts {
            receipt.decision = "deny".to_string();
        }
        let denied_failures = evaluate_build_verification_binding(&record, &case.artifact_ref, &receipts);
        assert!(!denied_failures.is_bound);
        assert_eq!(denied_failures.diagnostics, denied.diagnostics);
    }
