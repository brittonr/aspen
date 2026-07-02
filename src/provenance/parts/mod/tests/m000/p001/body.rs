
    #[test]
    fn sensitive_operations_require_stronger_provenance_than_reviewed() {
        let artifact_ref = synthetic_ref("artifact", "sensitive").expect("artifact ref");
        let reviewed = synthetic_reviewed_record(&artifact_ref).expect("reviewed record");
        let denied = evaluate(&EvaluationInput {
            operation: "install-policy-artifact",
            profile: "node-control",
            artifact_ref: &artifact_ref,
            provenance_values: std::slice::from_ref(&reviewed),
            build_verification_values: &[],
            prior_diagnostics: &[],
        })
        .expect("reviewed sensitive evaluation");
        assert_eq!(denied.decision, "deny");
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("requires stronger provenance")));

        let source_refs = vec![synthetic_ref("source", "policy-trusted").expect("source ref")];
        let toolchain_refs = vec![synthetic_ref("toolchain", "policy-trusted").expect("toolchain ref")];
        let dependency_ref = synthetic_ref("deps", "policy-trusted").expect("deps ref");
        let builder_ref = synthetic_ref("builder", "policy-trusted").expect("builder ref");
        let policy = record_value(&RecordInput {
            artifact_ref: &artifact_ref,
            trust_state: TRUST_STATE_POLICY_TRUSTED,
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
        .expect("policy trusted record");
        let admitted = evaluate(&EvaluationInput {
            operation: "install-policy-artifact",
            profile: "node-control",
            artifact_ref: &artifact_ref,
            provenance_values: &[policy],
            build_verification_values: &[],
            prior_diagnostics: &[],
        })
        .expect("policy trusted sensitive evaluation");
        assert_eq!(admitted.decision, "pass");
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_provenance_hash_only_denied_and_trust_monotonicity(tc: hegel::TestCase) {
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(1).max_value(1_000_000));
        let artifact_ref = synthetic_ref("artifact", &format!("hegel-{salt}")).expect("artifact ref");
        let hash_only = evaluate(&EvaluationInput {
            operation: "install",
            profile: "node-control",
            artifact_ref: &artifact_ref,
            provenance_values: &[],
            build_verification_values: &[],
            prior_diagnostics: &[],
        })
        .expect("hash-only evaluation");
        assert_eq!(hash_only.decision, "deny");
        assert!(hash_only.diagnostics.iter().any(|diagnostic| diagnostic.contains("missing provenance")));

        let reviewed = synthetic_reviewed_record(&artifact_ref).expect("reviewed record");
        let reviewed_eval = evaluate(&EvaluationInput {
            operation: "install",
            profile: "node-control",
            artifact_ref: &artifact_ref,
            provenance_values: std::slice::from_ref(&reviewed),
            build_verification_values: &[],
            prior_diagnostics: &[],
        })
        .expect("reviewed evaluation");
        assert_eq!(reviewed_eval.decision, "pass");

        let sensitive_eval = evaluate(&EvaluationInput {
            operation: "remote-sync-execute",
            profile: "node-control",
            artifact_ref: &artifact_ref,
            provenance_values: &[reviewed],
            build_verification_values: &[],
            prior_diagnostics: &[],
        })
        .expect("sensitive reviewed evaluation");
        assert_eq!(sensitive_eval.decision, "deny");
        assert!(
            sensitive_eval
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("requires stronger provenance"))
        );
    }
