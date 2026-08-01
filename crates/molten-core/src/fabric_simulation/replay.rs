use super::*;

const SHRINK_ATTEMPT_INCREMENT: u64 = 1;
const SHRINK_REMOVAL_INCREMENT: u64 = 1;
const SHRINK_DIVISOR: u64 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShrinkIssue {
    OriginalFailureNotReproduced,
    AttemptBoundExceeded { attempts: u64, maximum: u64 },
    InvalidOriginalWorld(Vec<WorldIssue>),
    Overflow(&'static str),
}

pub fn compare_replay(expected: &[SchedulerChoiceRecord], actual: &[SchedulerChoiceRecord]) -> ReplayComparison {
    let shared = expected.len().min(actual.len());
    for index in 0..shared {
        let expected_record = &expected[index];
        let actual_record = &actual[index];
        let expected_eligible = choice_ids(&expected_record.eligible);
        let actual_eligible = choice_ids(&actual_record.eligible);
        if expected_record.position != actual_record.position
            || expected_record.selected.choice_id != actual_record.selected.choice_id
            || expected_eligible != actual_eligible
        {
            return ReplayComparison {
                matches: false,
                first_divergence: Some(ReplayDivergence {
                    position: expected_record.position.min(actual_record.position),
                    expected_choice_id: expected_record.selected.choice_id.clone(),
                    eligible_choice_ids: actual_eligible,
                    diagnostic: "scheduler choice or eligible set diverged".to_string(),
                }),
            };
        }
    }
    if expected.len() != actual.len() {
        let (position, expected_choice_id, eligible_choice_ids) = if expected.len() > shared {
            let record = &expected[shared];
            (record.position, record.selected.choice_id.clone(), Vec::new())
        } else {
            let record = &actual[shared];
            (record.position, "end-of-trace".to_string(), choice_ids(&record.eligible))
        };
        return ReplayComparison {
            matches: false,
            first_divergence: Some(ReplayDivergence {
                position,
                expected_choice_id,
                eligible_choice_ids,
                diagnostic: "scheduler trace length diverged".to_string(),
            }),
        };
    }
    ReplayComparison {
        matches: true,
        first_divergence: None,
    }
}

pub fn shrink_simulation_failure(
    original: &SimulatedWorldManifest,
    mut preserves_failure: impl FnMut(&AdmittedSimulatedWorld) -> bool,
) -> Result<ShrinkResult, ShrinkIssue> {
    let admitted = admit_simulated_world(original).map_err(ShrinkIssue::InvalidOriginalWorld)?;
    if !preserves_failure(&admitted) {
        return Err(ShrinkIssue::OriginalFailureNotReproduced);
    }
    let maximum = original.bounds.max_shrink_attempts.min(MAX_SHRINK_ATTEMPTS);
    let mut current = original.clone();
    let mut attempts = 0_u64;
    let mut removed_workload_steps = 0_u64;

    loop {
        let mut changed = false;
        if current.workload.len() > 1 {
            let mut candidate = current.clone();
            candidate.workload.pop();
            if try_candidate(&candidate, &mut preserves_failure, &mut attempts, maximum)? {
                current = candidate;
                removed_workload_steps = removed_workload_steps
                    .checked_add(SHRINK_REMOVAL_INCREMENT)
                    .ok_or(ShrinkIssue::Overflow("removed-workload-steps"))?;
                changed = true;
            }
        }
        if !current.faults.is_empty() {
            let mut candidate = current.clone();
            candidate.faults.pop();
            if try_candidate(&candidate, &mut preserves_failure, &mut attempts, maximum)? {
                current = candidate;
                changed = true;
            }
        }
        if current.nodes.len() > 1 {
            let candidate_node = current.nodes.last().map(|node| node.node_id.clone());
            if let Some(candidate_node) = candidate_node
                && node_is_removable(&current, &candidate_node)
            {
                let mut candidate = current.clone();
                candidate.nodes.retain(|node| node.node_id != candidate_node);
                if try_candidate(&candidate, &mut preserves_failure, &mut attempts, maximum)? {
                    current = candidate;
                    changed = true;
                }
            }
        }
        let mut candidate = current.clone();
        let reduced_resources = reduce_positive_bound(candidate.bounds.max_resource_units);
        let reduced_trace = reduce_positive_bound(candidate.bounds.max_trace_bytes);
        if reduced_resources < candidate.bounds.max_resource_units || reduced_trace < candidate.bounds.max_trace_bytes {
            candidate.bounds.max_resource_units = reduced_resources;
            candidate.bounds.max_trace_bytes = reduced_trace;
            if try_candidate(&candidate, &mut preserves_failure, &mut attempts, maximum)? {
                current = candidate;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    let admitted = admit_simulated_world(&current).map_err(ShrinkIssue::InvalidOriginalWorld)?;
    let failure_preserved = preserves_failure(&admitted);
    Ok(ShrinkResult {
        world: current,
        attempts,
        removed_workload_steps,
        failure_preserved,
    })
}

fn try_candidate(
    candidate: &SimulatedWorldManifest,
    preserves_failure: &mut impl FnMut(&AdmittedSimulatedWorld) -> bool,
    attempts: &mut u64,
    maximum: u64,
) -> Result<bool, ShrinkIssue> {
    *attempts = attempts.checked_add(SHRINK_ATTEMPT_INCREMENT).ok_or(ShrinkIssue::Overflow("shrink-attempts"))?;
    if *attempts > maximum {
        return Err(ShrinkIssue::AttemptBoundExceeded {
            attempts: *attempts,
            maximum,
        });
    }
    let Ok(admitted) = admit_simulated_world(candidate) else {
        return Ok(false);
    };
    Ok(preserves_failure(&admitted))
}

fn node_is_removable(world: &SimulatedWorldManifest, node_id: &str) -> bool {
    !world.workload.iter().any(|step| step.node_id == node_id)
        && !world.faults.iter().any(|fault| fault.target == node_id)
}

fn reduce_positive_bound(value: u64) -> u64 {
    (value / SHRINK_DIVISOR).max(1)
}

fn choice_ids(choices: &[EligibleChoice]) -> Vec<String> {
    choices.iter().map(|choice| choice.choice_id.clone()).collect()
}
