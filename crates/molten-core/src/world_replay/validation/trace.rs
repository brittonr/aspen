use super::super::*;
use super::bounded_sorted_issues;
use super::valid_reference;
use super::validate_world_replay_profile;

pub fn validate_world_transition_trace(
    trace: &WorldTransitionTrace,
    bounds: &WorldReplayBounds,
) -> Vec<WorldReplayIssue> {
    let mut issues = Vec::with_capacity(bounds.max_diagnostics);
    if trace.schema != WORLD_TRANSITION_TRACE_SCHEMA {
        issues.push(WorldReplayIssue::InvalidSchema("transition-trace"));
    }
    if !valid_reference(&trace.trace_ref) {
        issues.push(WorldReplayIssue::InvalidReference("trace-ref"));
    }
    match identify_world_transition_trace(trace) {
        Ok(identity) if identity != trace.trace_ref => issues.push(WorldReplayIssue::TraceIdentityMismatch),
        Err(issue) => issues.push(issue),
        Ok(_) => {}
    }
    issues.extend(validate_world_replay_profile(&trace.profile));
    if trace.steps.is_empty() {
        issues.push(WorldReplayIssue::EmptyTrace);
    }
    if trace.steps.len() > bounds.max_steps {
        issues.push(WorldReplayIssue::StepLimitExceeded);
    }
    issues.extend(validate_steps(trace, bounds));
    bounded_sorted_issues(issues, bounds.max_diagnostics)
}

fn validate_steps(trace: &WorldTransitionTrace, bounds: &WorldReplayBounds) -> Vec<WorldReplayIssue> {
    let mut issues = Vec::with_capacity(bounds.max_diagnostics);
    let mut expected_parent = &trace.initial_commit;
    for (index, step) in trace.steps.iter().enumerate() {
        let Ok(expected_position) = u64::try_from(index) else {
            issues.push(WorldReplayIssue::StepLimitExceeded);
            break;
        };
        if step.position != expected_position {
            issues.push(WorldReplayIssue::NonContiguousStep {
                expected: expected_position,
                actual: step.position,
            });
        }
        if &step.expected_parent != expected_parent {
            issues.push(WorldReplayIssue::StepParentMismatch {
                position: step.position,
            });
        }
        if step.profile_ref != trace.profile.profile_ref {
            issues.push(WorldReplayIssue::StepProfileMismatch {
                position: step.position,
            });
        }
        if !valid_reference(&step.input.input_ref) {
            issues.push(WorldReplayIssue::InvalidReference("transition-input-ref"));
        }
        if !valid_reference(&step.input.schema_ref) {
            issues.push(WorldReplayIssue::InvalidReference("transition-input-schema-ref"));
        }
        if step.input.byte_length == 0 || step.input.byte_length > bounds.max_member_bytes {
            issues.push(WorldReplayIssue::MemberByteLimitExceeded(step.input.input_ref.clone()));
        }
        expected_parent = &step.expected_successor;
    }
    issues
}
