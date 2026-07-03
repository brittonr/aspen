
    #[test]
    fn operation_profile_thresholds_are_profile_specific() {
        // r[verify molten.provenance_state_proof.profile_thresholds]
        let low_risk = operation_profile_threshold("install", PROFILE_NODE_CONTROL);
        assert_eq!(low_risk.minimum_trust_state, TRUST_STATE_REVIEWED);
        assert!(!low_risk.reproducible_build_verification_required);

        let sensitive = operation_profile_threshold(OPERATION_INSTALL_PRODUCTION_EXECUTABLE, PROFILE_NODE_CONTROL);
        assert_eq!(sensitive.minimum_trust_state, TRUST_STATE_REPRODUCIBLE_VERIFIED);
        assert!(sensitive.reproducible_build_verification_required);

        let local = operation_profile_threshold("run-fixture", PROFILE_LOCAL_TEST);
        assert_eq!(local.minimum_trust_state, TRUST_STATE_SANDBOX_ONLY);
    }

    #[test]
    fn sensitive_operations_require_stronger_provenance_than_reviewed() {
        // r[verify molten.provenance_state_proof.profile_thresholds]
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

    #[test]
    fn provenance_receipts_do_not_replace_node_job_or_remote_gates() {
        // r[verify molten.provenance_state_proof.evidence_only_boundary]
        let provenance_ref = synthetic_ref("provenance-receipt", "boundary").expect("provenance ref");
        let authority_ref = synthetic_ref("authority", "boundary").expect("authority ref");
        let policy_ref = synthetic_ref("policy", "boundary").expect("policy ref");
        let resource_ref = synthetic_ref("resource", "boundary").expect("resource ref");
        let source_gate_ref = synthetic_ref("source-gate", "boundary").expect("source gate ref");
        let transport_ref = synthetic_ref("transport", "boundary").expect("transport ref");
        let retention_ref = synthetic_ref("retention", "boundary").expect("retention ref");
        let execution_ref = synthetic_ref("execution", "boundary").expect("execution ref");

        let node = evaluate_evidence_only_boundary(&EvidenceOnlyBoundaryInput {
            operation: "node-install",
            provenance_receipt_refs: std::slice::from_ref(&provenance_ref),
            authority_refs: &[],
            policy_refs: &[],
            resource_refs: &[],
            source_gate_refs: &[],
            transport_refs: std::slice::from_ref(&transport_ref),
            retention_refs: std::slice::from_ref(&retention_ref),
            execution_refs: std::slice::from_ref(&execution_ref),
        })
        .expect("node boundary");
        assert_eq!(node.decision, "deny");
        assert!(node.diagnostics.iter().any(|diagnostic| diagnostic.contains("authority trust")));
        assert!(node.diagnostics.iter().any(|diagnostic| diagnostic.contains("source-gate trust")));

        let job = evaluate_evidence_only_boundary(&EvidenceOnlyBoundaryInput {
            operation: "job-execute",
            provenance_receipt_refs: std::slice::from_ref(&provenance_ref),
            authority_refs: std::slice::from_ref(&authority_ref),
            policy_refs: std::slice::from_ref(&policy_ref),
            resource_refs: std::slice::from_ref(&resource_ref),
            source_gate_refs: &[],
            transport_refs: std::slice::from_ref(&transport_ref),
            retention_refs: std::slice::from_ref(&retention_ref),
            execution_refs: &[],
        })
        .expect("job boundary");
        assert_eq!(job.decision, "deny");
        assert!(job.diagnostics.iter().any(|diagnostic| diagnostic.contains("execution trust")));

        let remote = evaluate_evidence_only_boundary(&EvidenceOnlyBoundaryInput {
            operation: OPERATION_REMOTE_SYNC_EXECUTE,
            provenance_receipt_refs: std::slice::from_ref(&provenance_ref),
            authority_refs: &[],
            policy_refs: std::slice::from_ref(&policy_ref),
            resource_refs: std::slice::from_ref(&resource_ref),
            source_gate_refs: std::slice::from_ref(&source_gate_ref),
            transport_refs: std::slice::from_ref(&transport_ref),
            retention_refs: std::slice::from_ref(&retention_ref),
            execution_refs: std::slice::from_ref(&execution_ref),
        })
        .expect("remote boundary");
        assert_eq!(remote.decision, "deny");
        assert!(remote.diagnostics.iter().any(|diagnostic| diagnostic.contains("authority trust")));

        let full = evaluate_evidence_only_boundary(&EvidenceOnlyBoundaryInput {
            operation: "fully-gated",
            provenance_receipt_refs: std::slice::from_ref(&provenance_ref),
            authority_refs: std::slice::from_ref(&authority_ref),
            policy_refs: std::slice::from_ref(&policy_ref),
            resource_refs: std::slice::from_ref(&resource_ref),
            source_gate_refs: std::slice::from_ref(&source_gate_ref),
            transport_refs: std::slice::from_ref(&transport_ref),
            retention_refs: std::slice::from_ref(&retention_ref),
            execution_refs: std::slice::from_ref(&execution_ref),
        })
        .expect("full boundary");
        assert_eq!(full.decision, "pass");
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
