use super::super::*;
use super::fixture::*;
use crate::world_commit::RootKind;

#[test]
fn comparison_stops_at_earliest_typed_root_divergence() {
    // r[verify molten.world_replay.divergence]
    let request = valid_request();
    let first_step = &request.trace.steps[0];
    let second_step = &request.trace.steps[1];
    let observations = vec![
        WorldReplayTransitionObservation {
            position: first_step.position,
            observed_parent: first_step.expected_parent.clone(),
            actual: WorldReplayObservedCommit {
                commit_ref: commit_ref("unexpected-successor"),
                roots: vec![
                    root(RootKind::Artifact, "unexpected-artifact"),
                    root(RootKind::Schema, "schema"),
                ],
            },
            field_differences: vec![
                WorldReplayFieldDifference {
                    root_kind: RootKind::Artifact,
                    field_path: vec!["z-last".to_string()],
                },
                WorldReplayFieldDifference {
                    root_kind: RootKind::Artifact,
                    field_path: vec!["a-first".to_string(), "leaf".to_string()],
                },
            ],
        },
        WorldReplayTransitionObservation {
            position: second_step.position,
            observed_parent: commit_ref("also-wrong-parent"),
            actual: WorldReplayObservedCommit {
                commit_ref: commit_ref("also-wrong-successor"),
                roots: Vec::new(),
            },
            field_differences: Vec::new(),
        },
    ];

    let comparison = compare_world_replay(&request.trace, &request.commits, &observations, &request.bounds)
        .expect("bounded comparison");
    let divergence = comparison.divergence.expect("first divergence");
    assert!(!comparison.complete);
    assert_eq!(comparison.matched_steps, 0);
    assert_eq!(divergence.position, 0);
    assert_eq!(divergence.root_kind, Some(RootKind::Artifact));
    assert_eq!(divergence.field_path, vec!["a-first".to_string(), "leaf".to_string()]);
}

#[test]
fn comparison_accepts_complete_repeated_observations() {
    let request = valid_request();
    let observations = matching_observations(&request);
    let first = compare_world_replay(&request.trace, &request.commits, &observations, &request.bounds)
        .expect("matching replay");
    let second = compare_world_replay(&request.trace, &request.commits, &observations, &request.bounds)
        .expect("stable matching replay");

    assert_eq!(first, second);
    assert!(first.complete);
    assert_eq!(first.matched_steps, EXPECTED_STEP_COUNT);
    assert!(first.divergence.is_none());
}

#[test]
fn malformed_field_path_is_denied_before_comparison() {
    let request = valid_request();
    let mut observations = matching_observations(&request);
    observations[0].field_differences.push(WorldReplayFieldDifference {
        root_kind: RootKind::Artifact,
        field_path: vec!["secret value".to_string()],
    });

    let issues = compare_world_replay(&request.trace, &request.commits, &observations, &request.bounds)
        .expect_err("malformed field path denied");
    assert!(issues.contains(&WorldReplayIssue::InvalidFieldPath));
}

#[test]
fn equal_commit_ref_with_divergent_roots_is_a_root_divergence() {
    // r[verify molten.world_commit.replay_sealed_observation_and_full_comparison]
    let request = valid_request();
    let mut observations = matching_observations(&request);
    observations[1].actual.roots = vec![
        root(RootKind::Artifact, "unexpected-artifact"),
        root(RootKind::Schema, "schema"),
    ];
    observations[1].field_differences = vec![WorldReplayFieldDifference {
        root_kind: RootKind::Artifact,
        field_path: vec!["payload".to_string()],
    }];
    assert_eq!(observations[1].actual.commit_ref, request.trace.steps[1].expected_successor);

    let comparison = compare_world_replay(&request.trace, &request.commits, &observations, &request.bounds)
        .expect("bounded comparison");
    let divergence = comparison.divergence.expect("root divergence under an equal commit ref");
    assert!(!comparison.complete);
    assert_eq!(comparison.matched_steps, 1);
    assert_eq!(divergence.kind, WorldReplayDivergenceKind::RootMismatch);
    assert_eq!(divergence.position, 1);
    assert_eq!(divergence.root_kind, Some(RootKind::Artifact));
    assert_eq!(divergence.field_path, vec!["payload".to_string()]);

    let mut missing_root = matching_observations(&request);
    missing_root[0].actual.roots.retain(|root| root.kind() != RootKind::Schema);
    let divergence = compare_world_replay(&request.trace, &request.commits, &missing_root, &request.bounds)
        .expect("bounded comparison")
        .divergence
        .expect("a missing root diverges");
    assert_eq!(divergence.root_kind, Some(RootKind::Schema));
    assert!(divergence.field_path.is_empty());
}

#[test]
fn equal_commit_ref_and_roots_with_field_differences_is_a_divergence() {
    // r[verify molten.world_commit.replay_sealed_observation_and_full_comparison]
    let request = valid_request();
    let mut observations = matching_observations(&request);
    observations[0].field_differences = vec![
        WorldReplayFieldDifference {
            root_kind: RootKind::Schema,
            field_path: vec!["version".to_string()],
        },
        WorldReplayFieldDifference {
            root_kind: RootKind::Artifact,
            field_path: vec!["z-last".to_string()],
        },
    ];

    let comparison = compare_world_replay(&request.trace, &request.commits, &observations, &request.bounds)
        .expect("bounded comparison");
    let divergence = comparison.divergence.expect("field divergence under equal commit and roots");
    assert_eq!(comparison.matched_steps, 0);
    assert_eq!(divergence.kind, WorldReplayDivergenceKind::RootMismatch);
    let expected_first = [(RootKind::Schema, "version"), (RootKind::Artifact, "z-last")]
        .into_iter()
        .min()
        .expect("first difference");
    assert_eq!(divergence.root_kind, Some(expected_first.0));
    assert_eq!(divergence.field_path, vec![expected_first.1.to_string()]);

    let clean = matching_observations(&request);
    let complete =
        compare_world_replay(&request.trace, &request.commits, &clean, &request.bounds).expect("bounded comparison");
    assert!(complete.complete && complete.divergence.is_none());
}
