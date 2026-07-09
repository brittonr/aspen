
    fn coverage_row(subsystem: &str, workflow: &str, eligibility: CoverageEligibility) -> CoverageRow {
        CoverageRow {
            subsystem: subsystem.to_string(),
            workflow: workflow.to_string(),
            eligibility,
            fresh_run_ref: Some(DEFAULT_ARTIFACT_REF.to_string()),
            verify_ref: Some(DEFAULT_CLOSURE_REF.to_string()),
            second_fresh_run_ref: Some(DEFAULT_INITIAL_STATE_REF.to_string()),
            negative_evidence_ref: Some(DEFAULT_SCHEMA_REF.to_string()),
            index_ref: Some(DEFAULT_POLICY_REF.to_string()),
            caveat_refs: vec![DEFAULT_CAPABILITY_REF.to_string()],
        }
    }

    #[test]
    fn coverage_matrix_passes_with_positive_and_negative_evidence() {
        let matrix = validate_coverage_matrix(&[
            coverage_row("harness", "report-replay", CoverageEligibility::Deterministic),
            coverage_row("node-control", "workflow-bundle", CoverageEligibility::Recorded),
        ])
        .expect("coverage matrix");
        assert_eq!(matrix.decision, "pass");
        let text = to_text(&matrix.value).expect("coverage text");
        assert!(text.contains("positive-and-negative-evidence"));
        assert!(text.contains("evidence-only"));
    }

    #[test]
    fn coverage_matrix_denies_missing_duplicate_and_diagnostic_only_rows() {
        let mut missing = coverage_row("job-worker", "lease-replay", CoverageEligibility::Deterministic);
        missing.verify_ref = None;
        missing.negative_evidence_ref = None;
        let duplicate = coverage_row("job-worker", "lease-replay", CoverageEligibility::Deterministic);
        let matrix = validate_coverage_matrix(&[missing, duplicate]).expect("denied matrix");
        assert_eq!(matrix.decision, "deny");
        assert!(matrix.diagnostics.iter().any(|diagnostic| diagnostic.contains("missing replay verify")));
        assert!(matrix.diagnostics.iter().any(|diagnostic| diagnostic.contains("duplicate")));

        let diagnostic_only = coverage_row("live-wan", "soak", CoverageEligibility::DiagnosticOnly);
        let diagnostic_matrix = validate_coverage_matrix(&[diagnostic_only]).expect("diagnostic-only matrix");
        assert_eq!(diagnostic_matrix.decision, "deny");
        assert!(diagnostic_matrix
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("diagnostic-only")));
    }

    #[test]
    fn coverage_matrix_rejects_non_replayable_pass_claims_and_empty_matrix() {
        let non_replayable = coverage_row("external-service", "live-side-effect", CoverageEligibility::NonReplayable);
        let matrix = validate_coverage_matrix(&[non_replayable]).expect("non-replayable matrix");
        assert_eq!(matrix.decision, "deny");
        assert!(matrix
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("non-replayable")));
        assert!(validate_coverage_matrix(&[]).is_err());
    }
