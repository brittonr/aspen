use super::*;

// r[verify molten.fabric_simulation.protocol_projection]
#[test]
fn a_lossless_cohort_with_a_sequence_gap_is_incomplete_and_cannot_pass() {
    let profile = profile();
    let complete = assemble_cohort(&profile, vec![
        projection("node-a", 0, 0, &["a"]),
        projection("node-b", 0, 1, &["a"]),
        projection("node-a", 1, 2, &["a", "b"]),
    ])
    .expect("cohort");
    assert!(complete.is_complete());

    let gapped = assemble_cohort(&profile, vec![
        projection("node-a", 0, 0, &["a"]),
        projection("node-b", 0, 1, &["a"]),
        projection("node-a", 2, 2, &["a", "b"]),
    ])
    .expect("cohort");
    assert_eq!(gapped.gaps, vec![CompletenessGap::SequenceGap {
        participant: "node-a".to_owned(),
        expected: 1,
        actual: 2,
    }]);
    let results = evaluate_safety(&profile, &gapped, &NoLocalRules);
    assert!(results.iter().all(|result| result.verdict != OracleVerdict::Pass), "{results:?}");

    let mut sampled = profile.clone();
    sampled.completeness = CompletenessMode::Sampled;
    let tolerated = assemble_cohort(&sampled, vec![
        projection("node-a", 0, 0, &["a"]),
        projection("node-b", 0, 1, &["a"]),
        projection("node-a", 2, 2, &["a", "b"]),
    ])
    .expect("cohort");
    assert!(tolerated.is_complete(), "a sampled profile tolerates gaps");
}

// r[verify molten.fabric_simulation.oracle_independence]
#[test]
fn a_required_participant_that_was_never_observed_leaves_the_cohort_incomplete() {
    let profile = profile();
    let missing = assemble_cohort(&profile, vec![projection("node-a", 0, 0, &["a"])]).expect("cohort");
    assert_eq!(missing.gaps, vec![CompletenessGap::MissingParticipant("node-b".to_owned())]);
    assert_eq!(
        verdict(&evaluate_safety(&profile, &missing, &NoLocalRules), SafetyLevel::Cohort),
        OracleVerdict::Incomplete
    );

    let mut partial = projection("node-b", 0, 1, &["a"]);
    partial.completeness = ObservationCompleteness::Partial;
    let observed = assemble_cohort(&profile, vec![projection("node-a", 0, 0, &["a"]), partial]).expect("cohort");
    assert!(!observed.is_complete());
}

// r[verify molten.fabric_simulation.protocol_oracle_validation]
#[test]
fn duplicate_sequences_generation_drift_and_unbounded_runs_are_malformed() {
    let profile = profile();
    let duplicate =
        assemble_cohort(&profile, vec![projection("node-a", 0, 0, &["a"]), projection("node-a", 0, 1, &["a"])]);
    assert!(matches!(duplicate, Err(issues) if issues.contains(&CohortIssue::DuplicateSequence {
        participant: "node-a".to_owned(),
        source_sequence: 0,
    })));

    let mut later = projection("node-a", 1, 1, &["a", "b"]);
    later.generation = 0;
    let drift = assemble_cohort(&profile, vec![projection("node-a", 0, 0, &["a"]), later]);
    assert!(matches!(drift, Err(issues) if issues.contains(&CohortIssue::GenerationDrift {
        participant: "node-a".to_owned(),
        source_sequence: 1,
    })));

    let mut bounded = profile.clone();
    bounded.bounds.max_records = 1;
    let overflow =
        assemble_cohort(&bounded, vec![projection("node-a", 0, 0, &["a"]), projection("node-b", 0, 1, &["a"])]);
    assert_eq!(overflow, Err(vec![CohortIssue::TooManyRecords]));

    let mut debug = projection("node-a", 0, 0, &["a"]);
    debug.state_ref = content("debug text");
    assert!(matches!(assemble_cohort(&profile, vec![debug]), Err(issues) if matches!(
        issues[0],
        CohortIssue::Projection { issue: ProjectionIssue::NonCanonicalStateIdentity, .. }
    )));
}
