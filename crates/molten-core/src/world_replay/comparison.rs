use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::*;
use crate::world_commit::RootKind;
use crate::world_commit::WorldRootRef;

// r[impl molten.world_replay.divergence]
pub fn compare_world_replay(
    trace: &WorldTransitionTrace,
    expected_commits: &[WorldReplayCommitClosure],
    observations: &[WorldReplayTransitionObservation],
    bounds: &WorldReplayBounds,
) -> Result<WorldReplayComparison, Vec<WorldReplayIssue>> {
    let expected = expected_commits
        .iter()
        .map(|commit| (commit.commit_ref.as_str(), commit))
        .collect::<BTreeMap<_, _>>();
    let mut issues = validate_observations(observations, bounds);
    for step in &trace.steps {
        if !expected.contains_key(step.expected_successor.as_str()) {
            issues.push(WorldReplayIssue::MissingCommit(step.expected_successor.as_str().to_string()));
        }
    }
    if !issues.is_empty() {
        return Err(sorted_issues(issues));
    }
    for (index, step) in trace.steps.iter().enumerate() {
        let Some(observation) = observations.get(index) else {
            return comparison_with_divergence(
                index,
                make_divergence(WorldReplayDivergenceKind::MissingObservation, step, None, None, Vec::new())?,
            );
        };
        if let Some(divergence) = compare_step(step, observation, &expected)? {
            return comparison_with_divergence(index, divergence);
        }
    }
    if let Some(extra) = observations.get(trace.steps.len()) {
        let last = trace.steps.last().ok_or_else(|| vec![WorldReplayIssue::EmptyTrace])?;
        return comparison_with_divergence(
            trace.steps.len(),
            make_divergence(WorldReplayDivergenceKind::UnexpectedObservation, last, Some(extra), None, Vec::new())?,
        );
    }
    Ok(WorldReplayComparison {
        complete: true,
        matched_steps: trace.steps.len(),
        divergence: None,
    })
}

fn compare_step(
    step: &WorldTransitionStep,
    observation: &WorldReplayTransitionObservation,
    expected: &BTreeMap<&str, &WorldReplayCommitClosure>,
) -> Result<Option<WorldReplayDivergence>, Vec<WorldReplayIssue>> {
    if observation.position != step.position {
        return Err(vec![WorldReplayIssue::ObservationPositionMismatch {
            expected: step.position,
            actual: observation.position,
        }]);
    }
    if observation.observed_parent != step.expected_parent {
        return make_divergence(WorldReplayDivergenceKind::ParentMismatch, step, Some(observation), None, Vec::new())
            .map(Some);
    }
    if observation.actual.commit_ref == step.expected_successor {
        return Ok(None);
    }
    let Some(expected_commit) = expected.get(step.expected_successor.as_str()) else {
        return Err(vec![WorldReplayIssue::MissingCommit(
            step.expected_successor.as_str().to_string(),
        )]);
    };
    let root_kind = first_root_difference(&expected_commit.roots, &observation.actual.roots);
    let field_path = first_field_path(root_kind, &observation.field_differences);
    make_divergence(
        if root_kind.is_some() {
            WorldReplayDivergenceKind::RootMismatch
        } else {
            WorldReplayDivergenceKind::CommitMismatch
        },
        step,
        Some(observation),
        root_kind,
        field_path,
    )
    .map(Some)
}

fn comparison_with_divergence(
    matched_steps: usize,
    divergence: WorldReplayDivergence,
) -> Result<WorldReplayComparison, Vec<WorldReplayIssue>> {
    Ok(WorldReplayComparison {
        complete: false,
        matched_steps,
        divergence: Some(divergence),
    })
}

fn validate_observations(
    observations: &[WorldReplayTransitionObservation],
    bounds: &WorldReplayBounds,
) -> Vec<WorldReplayIssue> {
    let mut issues = Vec::with_capacity(bounds.max_diagnostics);
    if observations.len() > bounds.max_steps {
        issues.push(WorldReplayIssue::ObservationLimitExceeded);
    }
    for observation in observations {
        issues.extend(validate_observed_roots(observation));
        for difference in &observation.field_differences {
            if difference.field_path.is_empty() || difference.field_path.len() > bounds.max_field_path_segments {
                issues.push(WorldReplayIssue::FieldPathLimitExceeded);
            }
            if difference
                .field_path
                .iter()
                .any(|segment| !valid_field_segment(segment, bounds.max_field_segment_bytes))
            {
                issues.push(WorldReplayIssue::InvalidFieldPath);
            }
        }
    }
    bounded_sorted_issues(issues, bounds.max_diagnostics)
}

fn validate_observed_roots(observation: &WorldReplayTransitionObservation) -> Vec<WorldReplayIssue> {
    let mut issues = Vec::with_capacity(observation.actual.roots.len());
    let mut root_kinds = BTreeSet::new();
    for root in &observation.actual.roots {
        if !root_kinds.insert(root.kind()) {
            issues.push(WorldReplayIssue::DuplicateCommitRoot {
                commit_ref: observation.actual.commit_ref.as_str().to_string(),
                root_kind: root.kind(),
            });
        }
    }
    issues
}

fn first_root_difference(expected: &[WorldRootRef], actual: &[WorldRootRef]) -> Option<RootKind> {
    let expected = expected.iter().map(|root| (root.kind(), root.as_str())).collect::<BTreeMap<_, _>>();
    let actual = actual.iter().map(|root| (root.kind(), root.as_str())).collect::<BTreeMap<_, _>>();
    let kinds = expected.keys().chain(actual.keys()).copied().collect::<BTreeSet<_>>();
    kinds.into_iter().find(|kind| expected.get(kind) != actual.get(kind))
}

fn first_field_path(root_kind: Option<RootKind>, differences: &[WorldReplayFieldDifference]) -> Vec<String> {
    let Some(root_kind) = root_kind else {
        return Vec::new();
    };
    let mut paths = differences
        .iter()
        .filter(|difference| difference.root_kind == root_kind)
        .map(|difference| difference.field_path.clone())
        .collect::<Vec<_>>();
    paths.sort();
    paths.into_iter().next().unwrap_or_default()
}

fn make_divergence(
    kind: WorldReplayDivergenceKind,
    step: &WorldTransitionStep,
    observation: Option<&WorldReplayTransitionObservation>,
    root_kind: Option<RootKind>,
    field_path: Vec<String>,
) -> Result<WorldReplayDivergence, Vec<WorldReplayIssue>> {
    let mut divergence = WorldReplayDivergence {
        schema: WORLD_REPLAY_DIVERGENCE_SCHEMA.to_string(),
        divergence_ref: placeholder_ref(),
        kind,
        position: step.position,
        expected_parent: step.expected_parent.clone(),
        observed_parent: observation.map(|value| value.observed_parent.clone()),
        expected_commit: step.expected_successor.clone(),
        actual_commit: observation.map(|value| value.actual.commit_ref.clone()),
        root_kind,
        field_path,
    };
    divergence.divergence_ref = identify_world_replay_divergence(&divergence).map_err(|issue| vec![issue])?;
    Ok(divergence)
}

fn placeholder_ref() -> String {
    const ZERO_DIGEST: &str = "0000000000000000000000000000000000000000000000000000000000000000";
    format!("blake3:{ZERO_DIGEST}")
}
