
    #[test]
    fn release_candidate_denies_broad_caveat_missing_matrix_or_candidate_mismatch() {
        let base_refs = refs(&["base"]);
        let diagnostics = texts(&["candidate reviewed"]);
        let source_ref = reference("source");
        let other_source_ref = reference("other-source");
        let artifact_ref = reference("validation-artifact");
        let evidence = [CandidateEvidenceBinding {
            artifact_ref: &artifact_ref,
            source_ref: &source_ref,
        }];
        let mismatched_evidence = [CandidateEvidenceBinding {
            artifact_ref: &artifact_ref,
            source_ref: &other_source_ref,
        }];
        let caveats = texts(&["source caveat"]);
        let broad = pilot_decision_value(&PilotDecisionInput {
            decision: "pass",
            scope: BROAD_PRODUCTION_SCOPE,
            allowed_workloads: &texts(&["all workloads"]),
            denied_workloads: &texts(&["none"]),
            rollback_triggers: &texts(&["none"]),
            stop_conditions: &texts(&["none"]),
            operator_review_refs: &base_refs,
            caveats: &caveats,
            diagnostics: &diagnostics,
        });
        let candidate_input = ReleaseCandidateGateInput {
            decision: "pass",
            candidate: "bad-candidate",
            source_ref: &source_ref,
            rust_validation_evidence: &evidence,
            nextest_evidence: &evidence,
            nix_check_evidence: &evidence,
            cairn_validation_evidence: &evidence,
            octet_evidence: &evidence,
            dogfood_evidence: &evidence,
            bundle_verify_evidence: &evidence,
            promotion_evidence: &evidence,
            export_verify_evidence: &evidence,
            source_gate_status: CONFIGURATION_CLEAN_CAVEAT_STATUS,
            source_gate_caveats: &[],
            pilot_decision_evidence: &evidence,
            diagnostics: &diagnostics,
        };
        let missing_source_caveat = release_candidate_gate_value(&candidate_input);
        let missing_matrix = release_candidate_gate_value(&ReleaseCandidateGateInput { rust_validation_evidence: &[], source_gate_status: SOURCE_REMEDIATED_ZERO_STATUS, ..candidate_input });
        let mismatch = release_candidate_gate_value(&ReleaseCandidateGateInput { rust_validation_evidence: &mismatched_evidence, source_gate_status: SOURCE_REMEDIATED_ZERO_STATUS, ..candidate_input });
        assert!(broad.is_err());
        assert!(missing_source_caveat.is_err());
        assert!(missing_matrix
            .expect_err("missing matrix must deny")
            .to_string()
            .contains("Rust validation candidate evidence binding"));
        assert!(mismatch
            .expect_err("candidate mismatch must deny")
            .to_string()
            .contains("Rust validation candidate source mismatch"));
    }
