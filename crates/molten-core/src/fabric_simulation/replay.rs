use super::*;

const SHRINK_ATTEMPT_INCREMENT: u64 = 1;
const SHRINK_REMOVAL_INCREMENT: u64 = 1;
const SHRINK_DIVISOR: u64 = 2;
const FINGERPRINT_JOIN: &str = "|";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShrinkIssue {
    OriginalFailureNotReproduced,
    AttemptBoundExceeded { attempts: u64, maximum: u64 },
    InvalidOriginalWorld(Vec<WorldIssue>),
    Overflow(&'static str),
}

// r[impl molten.fabric_simulation.causal_exploration]
pub fn compare_replay(expected: &[SchedulerChoiceRecord], actual: &[SchedulerChoiceRecord]) -> ReplayComparison {
    let shared = expected.len().min(actual.len());
    for index in 0..shared {
        let expected_record = &expected[index];
        let actual_record = &actual[index];
        if let Some(divergence) = record_divergence(expected_record, actual_record) {
            return ReplayComparison {
                matches: false,
                first_divergence: Some(divergence),
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

fn record_divergence(expected: &SchedulerChoiceRecord, actual: &SchedulerChoiceRecord) -> Option<ReplayDivergence> {
    let expected_eligible = choice_ids(&expected.eligible);
    let actual_eligible = choice_ids(&actual.eligible);
    let mut mismatched_field = None;
    if expected.position != actual.position {
        mismatched_field = Some("position");
    } else if expected.virtual_tick != actual.virtual_tick {
        mismatched_field = Some("virtual-tick");
    } else if expected.selected.generation != actual.selected.generation {
        mismatched_field = Some("generation");
    } else if expected.selected.choice_id != actual.selected.choice_id {
        mismatched_field = Some("choice-id");
    } else if expected.semantic_output_ref != actual.semantic_output_ref {
        mismatched_field = Some("semantic-output-ref");
    } else if expected_eligible != actual_eligible {
        mismatched_field = Some("eligible-set");
    }
    mismatched_field.map(|field| ReplayDivergence {
        position: expected.position.min(actual.position),
        expected_choice_id: expected.selected.choice_id.clone(),
        eligible_choice_ids: actual_eligible,
        diagnostic: format!("scheduler record diverged at {field}"),
    })
}

// r[impl molten.fabric_simulation.causal_exploration]
pub fn failure_fingerprint(summary: &SimulationRunSummary) -> Option<FailureFingerprint> {
    if summary.decision == SimulationDecision::Pass {
        return None;
    }
    let mut failed_invariants = summary
        .invariant_results
        .iter()
        .filter(|result| !result.passed)
        .map(|result| invariant_fingerprint_key(&result.invariant))
        .collect::<Vec<_>>();
    failed_invariants.sort();
    let first_failure_sequence = summary
        .invariant_results
        .iter()
        .filter(|result| !result.passed)
        .filter_map(|result| result.first_failure_sequence)
        .min();
    let material = format!(
        "{}{FINGERPRINT_JOIN}{}{FINGERPRINT_JOIN}{}",
        summary.decision.as_str(),
        failed_invariants.join(FINGERPRINT_JOIN),
        first_failure_sequence.map_or_else(|| "none".to_string(), |sequence| sequence.to_string()),
    );
    Some(FailureFingerprint {
        decision: summary.decision,
        failed_invariants,
        first_failure_sequence,
        fingerprint_ref: format!("blake3:{}", blake3::hash(material.as_bytes()).to_hex()),
    })
}

fn invariant_fingerprint_key(invariant: &SimulationInvariant) -> String {
    match invariant {
        SimulationInvariant::Universal(kind) => format!("universal:{}", kind.as_str()),
        SimulationInvariant::ExtensionSemantic { service, invariant_id } => {
            format!("extension:{}:{}", service.as_str(), invariant_id)
        }
    }
}

// r[impl molten.fabric_simulation.causal_exploration]
pub fn shrink_simulation_failure(
    original: &SimulatedWorldManifest,
    mut rerun_candidate: impl FnMut(&AdmittedSimulatedWorld) -> Option<FailureFingerprint>,
) -> Result<ShrinkResult, ShrinkIssue> {
    let admitted = admit_simulated_world(original).map_err(ShrinkIssue::InvalidOriginalWorld)?;
    let original_fingerprint = rerun_candidate(&admitted).ok_or(ShrinkIssue::OriginalFailureNotReproduced)?;
    let maximum = original.bounds.max_shrink_attempts.min(MAX_SHRINK_ATTEMPTS);
    let mut current = original.clone();
    let mut attempts = 0_u64;
    let mut removed_workload_steps = 0_u64;

    loop {
        let mut changed = false;
        if current.workload.len() > 1 {
            let mut candidate = current.clone();
            candidate.workload.pop();
            if try_candidate(&candidate, &original_fingerprint, &mut rerun_candidate, &mut attempts, maximum)? {
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
            if try_candidate(&candidate, &original_fingerprint, &mut rerun_candidate, &mut attempts, maximum)? {
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
                if try_candidate(&candidate, &original_fingerprint, &mut rerun_candidate, &mut attempts, maximum)? {
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
            if try_candidate(&candidate, &original_fingerprint, &mut rerun_candidate, &mut attempts, maximum)? {
                current = candidate;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    let admitted = admit_simulated_world(&current).map_err(ShrinkIssue::InvalidOriginalWorld)?;
    let failure_preserved = rerun_candidate(&admitted) == Some(original_fingerprint);
    Ok(ShrinkResult {
        world: current,
        attempts,
        removed_workload_steps,
        failure_preserved,
    })
}

fn try_candidate(
    candidate: &SimulatedWorldManifest,
    original_fingerprint: &FailureFingerprint,
    mut rerun_candidate: impl FnMut(&AdmittedSimulatedWorld) -> Option<FailureFingerprint>,
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
    Ok(rerun_candidate(&admitted) == Some(original_fingerprint.clone()))
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
