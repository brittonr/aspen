mod operations;
mod support;

use super::*;

pub(super) struct EffectDraft {
    pub kind: ActorEffectIntentKind,
    pub wake_ref: Option<String>,
    pub subject_ref: Option<String>,
}

pub(super) struct PlannedChange {
    pub kind: ActorTransitionKind,
    pub next: ActorState,
    pub effects: Vec<EffectDraft>,
    pub restored_classes: Vec<SurvivalClass>,
}

pub(super) struct TransitionRefs {
    pub request_ref: String,
    pub operation_ref: String,
    pub before_state_ref: String,
}

// r[impl molten.addressable_actor.lifecycle]
// r[impl molten.addressable_actor.delivery]
// r[impl molten.addressable_actor.authority]
#[must_use]
pub fn plan_actor_transition(
    profile: &AddressableActorProfile,
    state: &ActorState,
    request: &ActorRequest,
) -> ActorTransition {
    let refs = transition_refs(state, request);
    if let Some(issue) = input_issue(profile, state, request) {
        return preserve_issue(state, &refs, ActorDecision::Denied, issue);
    }
    if let Some(applied) = state.applied_operations.get(&request.operation_id) {
        let issue = (applied.request_ref != refs.request_ref).then_some(ActorIssue::StateIdentityMismatch);
        return support::duplicate_or_deny(state, &refs, issue);
    }
    if state.applied_operations.len() >= MAX_ACTOR_OPERATIONS {
        return preserve_issue(state, &refs, ActorDecision::Denied, ActorIssue::OperationCapacityExceeded);
    }
    if state.unknown_effect_ref.is_some() && !matches!(request.operation, ActorOperation::ResolveUnknownEffect { .. }) {
        return preserve_issue(state, &refs, ActorDecision::Unknown, ActorIssue::UnknownExternalOutcome);
    }
    if is_completed_event_replay(state, &request.operation) {
        return support::duplicate_or_deny(state, &refs, None);
    }
    let planned = match operations::plan_operation(profile, state, request) {
        Ok(planned) => planned,
        Err(issue) => {
            let decision = decision_for_issue(&issue);
            return preserve_issue(state, &refs, decision, issue);
        }
    };
    support::finish_applied(support::FinishInput {
        state,
        request,
        refs,
        planned,
    })
}

fn transition_refs(state: &ActorState, request: &ActorRequest) -> TransitionRefs {
    let request_ref = identify_actor_request(request);
    TransitionRefs {
        operation_ref: identify_actor_operation(&request_ref, &request.operation),
        request_ref,
        before_state_ref: identify_actor_state(state),
    }
}

fn input_issue(profile: &AddressableActorProfile, state: &ActorState, request: &ActorRequest) -> Option<ActorIssue> {
    if !validate_addressable_actor_profile(profile).is_empty()
        || state.profile_ref != identify_addressable_actor_profile(profile)
    {
        return Some(ActorIssue::ProfileIdentityMismatch);
    }
    if !validate_actor_state(state).is_empty() {
        return Some(ActorIssue::StateIdentityMismatch);
    }
    validate_actor_request(request).into_iter().next().or_else(|| currentness_issue(state, request))
}

fn currentness_issue(state: &ActorState, request: &ActorRequest) -> Option<ActorIssue> {
    if request.actor_key_ref != state.actor_key_ref {
        return Some(ActorIssue::StaleActorKey);
    }
    if request.placement_ref != state.placement_ref {
        return Some(ActorIssue::StalePlacement);
    }
    if request.extension_generation != state.extension_generation {
        return Some(ActorIssue::StaleGeneration);
    }
    if request.expected_lifecycle_sequence != state.lifecycle_sequence {
        return Some(ActorIssue::StaleLifecycleSequence);
    }
    if request.admission.profile_ref != state.profile_ref
        || request.admission.system_extension_manifest_ref != state.system_extension_manifest_ref
        || !request.admission.all_current()
    {
        return Some(ActorIssue::AdmissionDenied);
    }
    None
}

fn preserve_issue(
    state: &ActorState,
    refs: &TransitionRefs,
    decision: ActorDecision,
    issue: ActorIssue,
) -> ActorTransition {
    support::preserve(support::PreserveInput {
        state,
        refs,
        decision,
        issue,
    })
}

fn is_completed_event_replay(state: &ActorState, operation: &ActorOperation) -> bool {
    let ActorOperation::CompleteDelivery { semantic_event_ref, .. } = operation else {
        return false;
    };
    state.completed_event_refs.binary_search(semantic_event_ref).is_ok()
}

const fn decision_for_issue(issue: &ActorIssue) -> ActorDecision {
    if matches!(issue, ActorIssue::UnknownExternalOutcome) {
        ActorDecision::Unknown
    } else {
        ActorDecision::Denied
    }
}
