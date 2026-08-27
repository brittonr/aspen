use choregraph_history::BranchRef;
use choregraph_history::plan_branch_advance;

use super::WorldHeadDecision;
use super::WorldHeadIssue;
use super::WorldHeadPlanRequest;
use super::WorldHeadPurpose;
use super::WorldHeadTransitionPlan;
use super::admission::validate_request;
use super::choregraph::build_choregraph_history;

pub fn plan_world_head_transition(request: &WorldHeadPlanRequest) -> WorldHeadDecision {
    let mut issues = validate_request(request);
    let history = match build_choregraph_history(&request.history, &request.policy.policy_ref, &request.bounds) {
        Ok(history) => Some(history),
        Err(history_issues) => {
            issues.extend(history_issues);
            None
        }
    };
    if !issues.is_empty() {
        issues.sort();
        issues.dedup();
        return WorldHeadDecision::Denied(issues);
    }
    let Some(history) = history else {
        return WorldHeadDecision::Denied(vec![WorldHeadIssue::ChoregraphDenied]);
    };
    let Some(successor_event) = history.events.get(&request.claim.successor_head).copied() else {
        return WorldHeadDecision::Denied(vec![WorldHeadIssue::SuccessorMissing]);
    };

    let currentness = if request.currentness.independent_ref.is_some() {
        super::WorldHeadCurrentnessClass::IndependentObservation
    } else {
        super::WorldHeadCurrentnessClass::RelativeToObservedStore
    };
    let after = super::WorldHeadState {
        branch_id: request.claim.branch_id.clone(),
        branch_class: request.claim.branch_class,
        head: request.claim.successor_head.clone(),
        generation: request.claim.successor_generation,
        policy_ref: request.claim.policy_ref.clone(),
    };

    if request.claim.purpose == WorldHeadPurpose::Create {
        return WorldHeadDecision::Admitted(WorldHeadTransitionPlan {
            claim_ref: request.claim_ref.clone(),
            before: None,
            after,
            choregraph_before_identity: None,
            choregraph_after_identity: successor_event.to_string(),
            currentness,
        });
    }

    let Some(current) = request.current.as_ref() else {
        return WorldHeadDecision::Denied(vec![WorldHeadIssue::CurrentHeadRequired]);
    };
    let Some(current_event) = history.events.get(&current.head).copied() else {
        return WorldHeadDecision::Denied(vec![WorldHeadIssue::ExpectedHistoryHeadMissing]);
    };
    let branch = BranchRef {
        name: current.branch_id.as_str().to_string(),
        head: current_event,
        generation: current.generation,
    };
    let mutation =
        plan_branch_advance(&history.graph, &branch, request.claim.expected_generation, current_event, successor_event);
    let Ok(mutation) = mutation else {
        return WorldHeadDecision::Denied(vec![WorldHeadIssue::ChoregraphDenied]);
    };
    if mutation.after.generation != request.claim.successor_generation {
        return WorldHeadDecision::Denied(vec![WorldHeadIssue::SkippedGeneration]);
    }

    WorldHeadDecision::Admitted(WorldHeadTransitionPlan {
        claim_ref: request.claim_ref.clone(),
        before: Some(current.clone()),
        after,
        choregraph_before_identity: Some(mutation.before.head.to_string()),
        choregraph_after_identity: mutation.after.head.to_string(),
        currentness,
    })
}
