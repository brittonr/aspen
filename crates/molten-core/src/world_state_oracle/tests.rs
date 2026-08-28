use super::*;

fn reference(label: &str) -> String {
    content_ref(label.as_bytes())
}

fn source() -> OracleSourceDescriptor {
    standard_source_descriptor(reference("doltlite-build"))
}

fn rows() -> Vec<SemanticStateRow> {
    vec![
        SemanticStateRow {
            key: "alpha".to_string(),
            value: "one".to_string(),
        },
        SemanticStateRow {
            key: "beta".to_string(),
            value: "two".to_string(),
        },
    ]
}

fn observation_input(
    case: OracleCaseKind,
    branch: Option<&str>,
    rows: Vec<SemanticStateRow>,
    outcome: OracleOutcome,
    backend_root: Option<&str>,
) -> OracleObservationInput {
    OracleObservationInput {
        adapter_ref: reference("adapter"),
        case,
        branch: branch.map(str::to_string),
        rows,
        outcome,
        backend_root: backend_root.map(str::to_string),
        diagnostics: Vec::new(),
    }
}

// r[verify molten.world_state_oracle.source]
#[test]
fn exact_source_admits_and_remote_or_pin_drift_denies() {
    let valid = source();
    assert!(validate_source_descriptor(&valid).is_empty());

    let mut remote = valid.clone();
    remote.remotes_enabled = true;
    assert!(validate_source_descriptor(&remote).contains(&OracleIssue::RemoteSupportEnabled));

    let mut missing_pin = valid.clone();
    missing_pin.build_ref.clear();
    assert!(validate_source_descriptor(&missing_pin).contains(&OracleIssue::MalformedReference(String::new())));

    let mut build_drift = valid.clone();
    build_drift.build_inputs.pop();
    assert!(validate_source_descriptor(&build_drift).contains(&OracleIssue::BuildInputMismatch));

    let mut stale = valid;
    stale.revision = "moving-main".to_string();
    assert!(validate_source_descriptor(&stale).contains(&OracleIssue::SourceMismatch));
}

// r[verify molten.world_state_oracle.compatibility]
#[test]
fn compatibility_ledger_is_closed_and_exception_counts_cannot_grow() {
    let rows = standard_compatibility_rows();
    let summary = validate_compatibility_rows(&rows, STANDARD_COMPATIBILITY_LIMITS).expect("ledger");
    assert_eq!(summary.compatible, 8);
    assert_eq!(summary.adapted, 2);
    assert_eq!(summary.intentional, 7);
    assert_eq!(summary.unsupported, 1);
    assert_eq!(summary.engine_gap, 0);

    let mut missing_issue = rows.clone();
    missing_issue
        .iter_mut()
        .find(|row| row.status == CompatibilityStatus::Unsupported)
        .expect("unsupported row")
        .issue = None;
    assert!(
        validate_compatibility_rows(&missing_issue, STANDARD_COMPATIBILITY_LIMITS)
            .expect_err("missing issue")
            .iter()
            .any(|issue| matches!(issue, OracleIssue::CompatibilityIssueMissing(_)))
    );

    let limits = CompatibilityLimits {
        max_unsupported: 0,
        ..STANDARD_COMPATIBILITY_LIMITS
    };
    assert!(
        validate_compatibility_rows(&rows, limits)
            .expect_err("exception growth")
            .contains(&OracleIssue::CompatibilityLimitExceeded(CompatibilityStatus::Unsupported))
    );
}

// r[verify molten.world_state_oracle.observations]
#[test]
fn observation_identity_is_sorted_bounded_and_backend_separated() {
    let mut input = rows();
    input.reverse();
    let observation = build_oracle_observation(
        &source(),
        observation_input(
            OracleCaseKind::HistoryIndependentState,
            Some("main"),
            input,
            OracleOutcome::EqualState,
            Some("doltlite-backend-root"),
        ),
    )
    .expect("observation");
    assert_eq!(observation.rows, rows());
    assert!(validate_observation(&observation, OracleBounds::standard(), true).is_empty());
    assert!(!observation.backend_root_is_global_identity);

    let mut overclaim = observation;
    overclaim.backend_root_is_global_identity = true;
    assert!(
        validate_observation(&overclaim, OracleBounds::standard(), false)
            .contains(&OracleIssue::BackendIdentityOverclaim)
    );
}

// r[verify molten.world_state_oracle.observations]
#[test]
fn differential_comparison_ignores_backend_hash_spelling_and_reports_first_semantic_drift() {
    let expected = build_oracle_observation(
        &source(),
        observation_input(
            OracleCaseKind::BranchIsolation,
            Some("feature"),
            rows(),
            OracleOutcome::Applied,
            Some("backend-a"),
        ),
    )
    .expect("expected");
    let observed = build_oracle_observation(
        &source(),
        observation_input(
            OracleCaseKind::BranchIsolation,
            Some("feature"),
            rows(),
            OracleOutcome::Applied,
            Some("backend-b"),
        ),
    )
    .expect("observed");
    let agreement = compare_oracle_observations(&expected, &observed).expect("agreement");
    assert_eq!(agreement.decision, ComparisonDecision::Agreement);
    assert!(!agreement.backend_roots_compared_as_global);

    let prolly = project_oracle_evidence(OracleConsumer::ProllyPilot, &observed, &agreement).expect("Prolly handoff");
    let benchmark =
        project_oracle_evidence(OracleConsumer::WorldBenchmark, &observed, &agreement).expect("benchmark handoff");
    assert_ne!(prolly.projection_ref, benchmark.projection_ref);
    assert!(validate_oracle_projection(&prolly).is_empty());
    assert!(!prolly.backend_root_included);
    assert!(!prolly.authority_granted);
    assert!(!prolly.correctness_proven);

    let mut overclaim = prolly;
    overclaim.authority_granted = true;
    assert!(validate_oracle_projection(&overclaim).contains(&OracleIssue::ProjectionOverclaim));

    let mut crossed_comparison = agreement.clone();
    crossed_comparison.observed_ref = reference("other-observation");
    assert!(
        project_oracle_evidence(OracleConsumer::WorldBenchmark, &observed, &crossed_comparison)
            .expect_err("crossed comparison")
            .contains(&OracleIssue::ProjectionComparisonMismatch)
    );

    let mut changed_rows = rows();
    changed_rows[1].value = "changed".to_string();
    let changed = build_oracle_observation(
        &source(),
        observation_input(OracleCaseKind::BranchIsolation, Some("feature"), changed_rows, OracleOutcome::Applied, None),
    )
    .expect("changed");
    let divergence = compare_oracle_observations(&expected, &changed).expect("divergence");
    assert_eq!(divergence.decision, ComparisonDecision::Divergence);
    assert_eq!(divergence.first_divergence.as_deref(), Some("rows[1]"));
}
