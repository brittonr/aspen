use crate::addressable_actor::*;

pub(super) struct FinishInput<'a> {
    pub state: &'a ActorState,
    pub request: &'a ActorRequest,
    pub refs: super::TransitionRefs,
    pub planned: super::PlannedChange,
}

pub(super) struct PreserveInput<'a> {
    pub state: &'a ActorState,
    pub refs: &'a super::TransitionRefs,
    pub decision: ActorDecision,
    pub issue: ActorIssue,
}

pub(super) fn finish_applied(input: FinishInput<'_>) -> ActorTransition {
    let planned = match advance_planned_state(input.state, input.request, &input.refs, input.planned) {
        Ok(planned) => planned,
        Err(issue) => {
            return preserve(PreserveInput {
                state: input.state,
                refs: &input.refs,
                decision: ActorDecision::Denied,
                issue,
            });
        }
    };
    let effects = match build_effects(&planned.next, &input.refs.request_ref, planned.effects) {
        Ok(effects) => effects,
        Err(issue) => {
            return preserve(PreserveInput {
                state: input.state,
                refs: &input.refs,
                decision: ActorDecision::Denied,
                issue,
            });
        }
    };
    let after_state_ref = identify_actor_state(&planned.next);
    ActorTransition {
        schema: ACTOR_TRANSITION_SCHEMA.to_string(),
        decision: ActorDecision::Applied,
        kind: planned.kind,
        request_ref: input.refs.request_ref,
        operation_ref: input.refs.operation_ref,
        before_state_ref: input.refs.before_state_ref,
        after_state_ref,
        next_state: planned.next,
        effects,
        restored_classes: planned.restored_classes,
        issue: None,
        effects_require_fresh_admission: true,
        external_effect_retry_authorized: false,
        receipt_authority: false,
    }
}

fn advance_planned_state(
    state: &ActorState,
    request: &ActorRequest,
    refs: &super::TransitionRefs,
    mut planned: super::PlannedChange,
) -> Result<super::PlannedChange, ActorIssue> {
    planned.next.lifecycle_sequence = state
        .lifecycle_sequence
        .checked_add(ADDRESSABLE_ACTOR_SEQUENCE_INCREMENT)
        .ok_or(ActorIssue::CounterOverflow)?;
    planned.next.revision = state
        .revision
        .checked_add(ADDRESSABLE_ACTOR_REVISION_INCREMENT)
        .ok_or(ActorIssue::CounterOverflow)?;
    planned.next.applied_operations.insert(request.operation_id.clone(), AppliedActorOperation {
        request_ref: refs.request_ref.clone(),
        operation_ref: refs.operation_ref.clone(),
        operation_kind: request.operation.kind().to_string(),
    });
    if planned.effects.len() > MAX_ACTOR_EFFECTS_PER_TRANSITION {
        return Err(ActorIssue::OperationCapacityExceeded);
    }
    if !validate_actor_state(&planned.next).is_empty() {
        return Err(ActorIssue::StateIdentityMismatch);
    }
    Ok(planned)
}

fn build_effects(
    state: &ActorState,
    request_ref: &str,
    drafts: Vec<super::EffectDraft>,
) -> Result<Vec<ActorEffectIntent>, ActorIssue> {
    let mut effects = Vec::with_capacity(drafts.len());
    for (ordinal, draft) in drafts.into_iter().enumerate() {
        let ordinal = u64::try_from(ordinal).map_err(|_| ActorIssue::CounterOverflow)?;
        let effect_ref = identify_actor_effect(ActorEffectIdentityInput {
            request_ref,
            ordinal,
            kind: draft.kind,
            actor_key_ref: &state.actor_key_ref,
            profile_ref: &state.profile_ref,
            system_extension_manifest_ref: &state.system_extension_manifest_ref,
            placement_ref: &state.placement_ref,
            extension_generation: state.extension_generation,
            lifecycle_sequence: state.lifecycle_sequence,
            wake_ref: draft.wake_ref.as_deref(),
            subject_ref: draft.subject_ref.as_deref(),
        });
        effects.push(ActorEffectIntent {
            schema: ACTOR_EFFECT_INTENT_SCHEMA.to_string(),
            effect_ref,
            request_ref: request_ref.to_string(),
            kind: draft.kind,
            actor_key_ref: state.actor_key_ref.clone(),
            profile_ref: state.profile_ref.clone(),
            system_extension_manifest_ref: state.system_extension_manifest_ref.clone(),
            placement_ref: state.placement_ref.clone(),
            extension_generation: state.extension_generation,
            lifecycle_sequence: state.lifecycle_sequence,
            wake_ref: draft.wake_ref,
            subject_ref: draft.subject_ref,
            requires_fresh_admission: true,
        });
    }
    Ok(effects)
}

pub(super) fn duplicate_or_deny(
    state: &ActorState,
    refs: &super::TransitionRefs,
    issue: Option<ActorIssue>,
) -> ActorTransition {
    match issue {
        Some(issue) => preserve(PreserveInput {
            state,
            refs,
            decision: ActorDecision::Denied,
            issue,
        }),
        None => ActorTransition {
            schema: ACTOR_TRANSITION_SCHEMA.to_string(),
            decision: ActorDecision::DuplicateReplay,
            kind: ActorTransitionKind::DuplicatePreserve,
            request_ref: refs.request_ref.clone(),
            operation_ref: refs.operation_ref.clone(),
            after_state_ref: refs.before_state_ref.clone(),
            before_state_ref: refs.before_state_ref.clone(),
            next_state: state.clone(),
            effects: Vec::new(),
            restored_classes: Vec::new(),
            issue: None,
            effects_require_fresh_admission: true,
            external_effect_retry_authorized: false,
            receipt_authority: false,
        },
    }
}

pub(super) fn preserve(input: PreserveInput<'_>) -> ActorTransition {
    ActorTransition {
        schema: ACTOR_TRANSITION_SCHEMA.to_string(),
        decision: input.decision,
        kind: ActorTransitionKind::DeniedPreserve,
        request_ref: input.refs.request_ref.clone(),
        operation_ref: input.refs.operation_ref.clone(),
        after_state_ref: input.refs.before_state_ref.clone(),
        before_state_ref: input.refs.before_state_ref.clone(),
        next_state: input.state.clone(),
        effects: Vec::new(),
        restored_classes: Vec::new(),
        issue: Some(input.issue),
        effects_require_fresh_admission: true,
        external_effect_retry_authorized: false,
        receipt_authority: false,
    }
}
