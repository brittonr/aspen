use super::super::*;
use super::bounded_sorted_issues;
use super::valid_reference;
use super::validate_capsule_closure;
use super::validate_commit_closure;
use super::validate_world_replay_bounds;

pub fn validate_world_replay_plan_request(request: &WorldReplayPlanRequest) -> Vec<WorldReplayIssue> {
    let mut issues = Vec::with_capacity(request.bounds.max_diagnostics);
    issues.extend(validate_world_replay_bounds(&request.bounds));
    issues.extend(validate_world_transition_trace(&request.trace, &request.bounds));
    issues.extend(validate_world_replay_capsule(&request.capsule, &request.bounds));
    if request.capsule.trace_ref != request.trace.trace_ref {
        issues.push(WorldReplayIssue::InvalidReference("capsule-trace-ref"));
    }
    if request.capsule.profile != request.trace.profile {
        issues.push(WorldReplayIssue::UnsupportedProfile);
    }
    if request.supported_profile_refs.len() > MAX_WORLD_REPLAY_DEPENDENCY_REFS {
        issues.push(WorldReplayIssue::DependencyLimitExceeded);
    }
    if !request
        .supported_profile_refs
        .iter()
        .any(|reference| reference == request.trace.profile.profile_ref.as_str())
    {
        issues.push(WorldReplayIssue::UnsupportedProfile);
    }
    for reference in &request.supported_profile_refs {
        if !valid_reference(reference) {
            issues.push(WorldReplayIssue::InvalidReference("supported-profile-ref"));
        }
    }
    issues.extend(validate_commit_closure(&request.trace, &request.commits));
    if issues.is_empty() {
        let requirements =
            required_world_replay_closure(&request.trace, &request.commits, &request.additional_requirements);
        issues.extend(validate_capsule_closure(&request.capsule, &requirements, request.bounds.max_diagnostics));
    }
    bounded_sorted_issues(issues, request.bounds.max_diagnostics)
}
