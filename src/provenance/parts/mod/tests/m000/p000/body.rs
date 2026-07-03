    use super::*;

    #[test]
    fn reviewed_provenance_passes_node_control_and_wrong_artifact_denies() {
        let artifact_ref = synthetic_ref("artifact", "reviewed").expect("artifact ref");
        let record = synthetic_reviewed_record(&artifact_ref).expect("record");
        let pass = evaluate(&EvaluationInput {
            operation: "install",
            profile: "node-control",
            artifact_ref: &artifact_ref,
            provenance_values: std::slice::from_ref(&record),
            build_verification_values: &[],
            prior_diagnostics: &[],
        })
        .expect("evaluate pass");
        assert_eq!(pass.decision, "pass");
        assert_eq!(crate::ledger::artifact_kind(&pass.receipt_value), "provenance-receipt");
        let wrong_ref = synthetic_ref("artifact", "wrong").expect("wrong ref");
        let denied = evaluate(&EvaluationInput {
            operation: "install",
            profile: "node-control",
            artifact_ref: &wrong_ref,
            provenance_values: &[record],
            build_verification_values: &[],
            prior_diagnostics: &[],
        })
        .expect("evaluate deny");
        assert_eq!(denied.decision, "deny");
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("no provenance")));
    }

    #[test]
    fn sandbox_only_is_local_test_only_and_hash_identity_is_not_trust() {
        let artifact_ref = synthetic_ref("artifact", "sandbox").expect("artifact ref");
        let source_refs = vec![synthetic_ref("source", "sandbox").expect("source ref")];
        let toolchain_refs = vec![synthetic_ref("toolchain", "sandbox").expect("toolchain ref")];
        let dependency_ref = synthetic_ref("deps", "sandbox").expect("deps ref");
        let builder_ref = synthetic_ref("builder", "sandbox").expect("builder ref");
        let record = record_value(&RecordInput {
            artifact_ref: &artifact_ref,
            trust_state: TRUST_STATE_SANDBOX_ONLY,
            source_refs: &source_refs,
            dependency_closure_ref: &dependency_ref,
            toolchain_refs: &toolchain_refs,
            builder_ref: &builder_ref,
            review_refs: &[],
            test_refs: &[],
            source_gate_refs: &[],
            policy_refs: &[],
            build_record_refs: &[],
        })
        .expect("sandbox record");
        let node = evaluate(&EvaluationInput {
            operation: "run",
            profile: "node-control",
            artifact_ref: &artifact_ref,
            provenance_values: std::slice::from_ref(&record),
            build_verification_values: &[],
            prior_diagnostics: &[],
        })
        .expect("node deny");
        assert_eq!(node.decision, "deny");
        let local = evaluate(&EvaluationInput {
            operation: "run",
            profile: "local-test",
            artifact_ref: &artifact_ref,
            provenance_values: &[record],
            build_verification_values: &[],
            prior_diagnostics: &[],
        })
        .expect("local pass");
        assert_eq!(local.decision, "pass");
    }

    #[test]
    fn build_record_verification_passes_and_mismatch_denies() {
        let expected_ref = synthetic_ref("artifact", "expected-build").expect("expected ref");
        let actual_ref = synthetic_ref("artifact", "actual-build").expect("actual ref");
        let source_refs = vec![synthetic_ref("source", "build").expect("source ref")];
        let toolchain_refs = vec![synthetic_ref("toolchain", "rust").expect("toolchain ref")];
        let dependency_ref = synthetic_ref("deps", "build").expect("deps ref");
        let builder_ref = synthetic_ref("builder", "nix").expect("builder ref");
        let nix_refs = vec![synthetic_ref("nix-derivation", "build").expect("nix ref")];
        let policy_refs = vec![synthetic_ref("policy", "build").expect("policy ref")];
        let evidence_refs = vec![synthetic_ref("octet", "build").expect("evidence ref")];
        let params = vec![
            BuildParam {
                key: "target".to_string(),
                value: "x86_64-linux".to_string(),
            },
            BuildParam {
                key: "profile".to_string(),
                value: "release".to_string(),
            },
        ];
        let value = build_record_value(&BuildRecordInput {
            expected_artifact_ref: &expected_ref,
            source_refs: &source_refs,
            dependency_closure_ref: &dependency_ref,
            toolchain_refs: &toolchain_refs,
            build_params: &params,
            builder_ref: &builder_ref,
            nix_derivation_refs: &nix_refs,
            policy_refs: &policy_refs,
            evidence_refs: &evidence_refs,
        })
        .expect("build record");
        let record = parse_build_record(&value).expect("parse build record");
        assert_eq!(record.expected_artifact_ref, expected_ref);
        assert_eq!(record.build_params.len(), 2);
        assert_eq!(crate::ledger::artifact_kind(&value), "provenance-build-record");

        let pass = verify_build(&BuildVerificationInput {
            build_record_value: &value,
            actual_artifact_ref: &expected_ref,
            prior_diagnostics: &[],
        })
        .expect("verify pass");
        assert_eq!(pass.decision, "pass");
        assert_eq!(crate::ledger::artifact_kind(&pass.receipt_value), "provenance-build-verify-receipt");

        let deny = verify_build(&BuildVerificationInput {
            build_record_value: &value,
            actual_artifact_ref: &actual_ref,
            prior_diagnostics: &[],
        })
        .expect("verify deny");
        assert_eq!(deny.decision, "deny");
        assert!(deny.diagnostics.iter().any(|diagnostic| diagnostic.contains("build artifact mismatch")));
    }

    struct Case {
        artifact_ref: String,
        source_refs: Vec<String>,
        dependency_ref: String,
        toolchain_refs: Vec<String>,
        builder_ref: String,
        build_record: IoValue,
        provenance: IoValue,
    }

    fn seed() -> Case {
        let artifact_ref = synthetic_ref("artifact", "reproducible").expect("artifact ref");
        let source_refs = vec![synthetic_ref("source", "reproducible").expect("source ref")];
        let toolchain_refs = vec![synthetic_ref("toolchain", "reproducible").expect("toolchain ref")];
        let dependency_ref = synthetic_ref("deps", "reproducible").expect("deps ref");
        let builder_ref = synthetic_ref("builder", "reproducible").expect("builder ref");
        let build_record = build_record_value(&BuildRecordInput {
            expected_artifact_ref: &artifact_ref,
            source_refs: &source_refs,
            dependency_closure_ref: &dependency_ref,
            toolchain_refs: &toolchain_refs,
            build_params: &[],
            builder_ref: &builder_ref,
            nix_derivation_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("build record");
        let build_record_refs = vec![canonical_hash(&build_record).expect("build record ref")];
        let provenance = record_value(&RecordInput {
            artifact_ref: &artifact_ref,
            trust_state: TRUST_STATE_REPRODUCIBLE_VERIFIED,
            source_refs: &source_refs,
            dependency_closure_ref: &dependency_ref,
            toolchain_refs: &toolchain_refs,
            builder_ref: &builder_ref,
            review_refs: &[],
            test_refs: &[],
            source_gate_refs: &[],
            policy_refs: &[],
            build_record_refs: &build_record_refs,
        })
        .expect("reproducible provenance");
        Case {
            artifact_ref,
            source_refs,
            dependency_ref,
            toolchain_refs,
            builder_ref,
            build_record,
            provenance,
        }
    }

    fn wrong_value(case: &Case) -> IoValue {
        let wrong_record_refs = vec![synthetic_ref("build-record", "wrong").expect("wrong build record ref")];
        record_value(&RecordInput {
            artifact_ref: &case.artifact_ref,
            trust_state: TRUST_STATE_REPRODUCIBLE_VERIFIED,
            source_refs: &case.source_refs,
            dependency_closure_ref: &case.dependency_ref,
            toolchain_refs: &case.toolchain_refs,
            builder_ref: &case.builder_ref,
            review_refs: &[],
            test_refs: &[],
            source_gate_refs: &[],
            policy_refs: &[],
            build_record_refs: &wrong_record_refs,
        })
        .expect("wrong binding provenance")
    }

    fn assert_missing(case: &Case) {
        // r[verify molten.provenance_state_proof.build_verification_binding]
        let receipt = evaluate(&EvaluationInput {
            operation: "install",
            profile: "node-control",
            artifact_ref: &case.artifact_ref,
            provenance_values: std::slice::from_ref(&case.provenance),
            build_verification_values: &[],
            prior_diagnostics: &[],
        })
        .expect("missing build verification denies");
        assert_eq!(receipt.decision, "deny");
        assert!(
            receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("requires a passing build verification"))
        );
    }

    fn assert_match(case: &Case) -> BuildVerification {
        // r[verify molten.provenance_state_proof.profile_thresholds]
        // r[verify molten.provenance_state_proof.build_verification_binding]
        let verification = verify_build(&BuildVerificationInput {
            build_record_value: &case.build_record,
            actual_artifact_ref: &case.artifact_ref,
            prior_diagnostics: &[],
        })
        .expect("verify reproducible build");
        let pass = evaluate(&EvaluationInput {
            operation: OPERATION_INSTALL_PRODUCTION_EXECUTABLE,
            profile: "node-control",
            artifact_ref: &case.artifact_ref,
            provenance_values: std::slice::from_ref(&case.provenance),
            build_verification_values: std::slice::from_ref(&verification.receipt_value),
            prior_diagnostics: &[],
        })
        .expect("matching build verification passes");
        assert_eq!(pass.decision, "pass");
        let parsed_record = parse_record(&case.provenance).expect("parse provenance");
        let parsed_receipt = parse_build_verification_receipt(&verification.receipt_value).expect("parse receipt");
        let binding = evaluate_build_verification_binding(&parsed_record, &case.artifact_ref, &[parsed_receipt]);
        assert!(binding.is_bound);
        assert_eq!(binding.provenance_record_ref, parsed_record.record_ref);
        assert_eq!(binding.matched_build_record_ref, Some(verification.build_record_ref.clone()));
        verification
    }

    fn assert_wrong_artifact(case: &Case) {
        // r[verify molten.provenance_state_proof.build_verification_binding]
        let wrong_artifact_ref = synthetic_ref("artifact", "wrong-verification-target").expect("wrong artifact ref");
        let wrong_artifact = verify_build(&BuildVerificationInput {
            build_record_value: &case.build_record,
            actual_artifact_ref: &wrong_artifact_ref,
            prior_diagnostics: &[],
        })
        .expect("verify wrong artifact");
        let eval = evaluate(&EvaluationInput {
            operation: OPERATION_INSTALL_PRODUCTION_EXECUTABLE,
            profile: "node-control",
            artifact_ref: &case.artifact_ref,
            provenance_values: std::slice::from_ref(&case.provenance),
            build_verification_values: std::slice::from_ref(&wrong_artifact.receipt_value),
            prior_diagnostics: &[],
        })
        .expect("wrong artifact verification denies");
        assert_eq!(eval.decision, "deny");
        assert!(eval.diagnostics.iter().any(|diagnostic| diagnostic.contains("decision is deny")));
        assert!(eval.diagnostics.iter().any(|diagnostic| diagnostic.contains("does not match artifact")));
    }

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
