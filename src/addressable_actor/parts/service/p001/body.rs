
fn reconcile_unknown_commit(
    commit_port: &impl ActorCommitPort,
    request: &ActorCommitRequest,
    original: Option<ActorCommitObservation>,
) -> ActorServiceResult<(ActorServiceStatus, ActorCommitObservation)> {
    let observed = commit_port.load(&request.actor_key_ref).map_err(ActorServiceError::Port)?;
    let (status, state_ref) = if observed.as_ref() == Some(&request.next) {
        (ActorServiceStatus::AppliedAfterReconciliation, Some(request.next.state_ref.clone()))
    } else if expected_matches(&request.expected, observed.as_ref()) {
        (ActorServiceStatus::NotAppliedAfterReconciliation, observed.map(|published| published.state_ref))
    } else {
        (ActorServiceStatus::Unknown, observed.map(|published| published.state_ref))
    };
    Ok((
        status,
        original.unwrap_or(ActorCommitObservation {
            disposition: ActorCommitDisposition::Unknown,
            currentness: ActorCommitCurrentness::Unknown,
            durability: ActorDurabilityOutcome::Unknown,
            engine_epoch: request.requested_engine_epoch,
            observed_state_ref: state_ref,
        }),
    ))
}

fn record_unknown_effect_state(
    commit_port: &mut impl ActorCommitPort,
    service: &ActorServiceRequest<'_>,
    current: &PublishedActorState,
    effect: &ActorEffectIntent,
) -> ActorServiceResult<PublishedActorState> {
    let internal_request = ActorRequest {
        schema: ACTOR_REQUEST_SCHEMA.to_string(),
        operation_id: format!("{UNKNOWN_EFFECT_OPERATION_PREFIX}{}", effect.effect_ref),
        actor_key_ref: current.state.actor_key_ref.clone(),
        placement_ref: current.state.placement_ref.clone(),
        extension_generation: current.state.extension_generation,
        expected_lifecycle_sequence: current.state.lifecycle_sequence,
        logical_tick: service.request.logical_tick,
        admission: service.request.admission.clone(),
        operation: ActorOperation::RecordUnknownEffect {
            effect_ref: effect.effect_ref.clone(),
        },
    };
    let transition = plan_actor_transition(service.profile, &current.state, &internal_request);
    if transition.decision != ActorDecision::Applied {
        return Ok(current.clone());
    }
    let next = PublishedActorState::from_state(transition.next_state);
    let request = ActorCommitRequest {
        actor_key_ref: current.state.actor_key_ref.clone(),
        expected: ExpectedActorState {
            state_ref: Some(current.state_ref.clone()),
            revision: current.revision,
        },
        next: next.clone(),
        requested_engine_epoch: service.requested_engine_epoch,
    };
    let (status, _observation) = commit_with_reconciliation(commit_port, &request)?;
    if status.commit_confirmed() {
        Ok(next)
    } else {
        Ok(current.clone())
    }
}

fn status_from_observation(observation: &ActorCommitObservation, request: &ActorCommitRequest) -> ActorServiceStatus {
    if observation.engine_epoch != request.requested_engine_epoch
        || !observation.currentness.is_current()
        || observation.durability != ActorDurabilityOutcome::Durable
    {
        return ActorServiceStatus::Unknown;
    }
    match observation.disposition {
        ActorCommitDisposition::Applied => ActorServiceStatus::Applied,
        ActorCommitDisposition::AlreadyApplied => ActorServiceStatus::AlreadyApplied,
        ActorCommitDisposition::Stale => ActorServiceStatus::Stale,
        ActorCommitDisposition::Unknown => ActorServiceStatus::Unknown,
    }
}

fn publish_status(
    status_port: &mut impl ActorStatusPort,
    state: &PublishedActorState,
    effect_observations: &[ActorEffectObservation],
) -> ActorServiceResult<ActorStatusObservation> {
    let evidence_refs = effect_observations
        .iter()
        .filter_map(|observation| observation.outcome_ref.clone())
        .collect::<Vec<_>>();
    let status = project_actor_status(&state.state, ActorStatusProjectionInput {
        maximum_events: MAX_ACTOR_STATUS_EVENTS,
        evidence_refs: &evidence_refs,
    })
    .map_err(ActorServiceError::Status)?;
    match status_port.publish_status(&status) {
        Ok(observation) => Ok(observation),
        Err(error) => Ok(ActorStatusObservation {
            status_ref: None,
            outcome_unknown: error.outcome_unknown,
        }),
    }
}

fn no_commit_outcome(
    transition: ActorTransition,
    status: ActorServiceStatus,
    service: &ActorServiceRequest<'_>,
    status_port: &mut impl ActorStatusPort,
) -> ActorServiceResult<ActorServiceOutcome> {
    let final_state = PublishedActorState::from_state(transition.next_state.clone());
    let effect_observations = Vec::new();
    let status_observation = publish_status(status_port, &final_state, &effect_observations)?;
    let commit = ActorCommitObservation {
        disposition: ActorCommitDisposition::Stale,
        currentness: ActorCommitCurrentness::Unknown,
        durability: ActorDurabilityOutcome::Unknown,
        engine_epoch: service.requested_engine_epoch,
        observed_state_ref: Some(final_state.state_ref.clone()),
    };
    let receipt = build_receipt(ReceiptInput {
        transition: &transition,
        final_state: &final_state,
        status,
        commit: &commit,
        effects: &effect_observations,
        status_observation: &status_observation,
    })?;
    Ok(ActorServiceOutcome {
        transition,
        receipt,
        commit_observation: None,
        effect_observations,
        status_observation,
        final_state,
    })
}

struct ReceiptInput<'a> {
    transition: &'a ActorTransition,
    final_state: &'a PublishedActorState,
    status: ActorServiceStatus,
    commit: &'a ActorCommitObservation,
    effects: &'a [ActorEffectObservation],
    status_observation: &'a ActorStatusObservation,
}

fn build_receipt(input: ReceiptInput<'_>) -> ActorServiceResult<CanonicalActorCommitReceipt> {
    let ReceiptInput {
        transition,
        final_state,
        status,
        commit,
        effects,
        status_observation,
    } = input;
    canonical_actor_commit_receipt(&ActorCommitReceipt {
        actor_key_ref: transition.next_state.actor_key_ref.clone(),
        request_ref: transition.request_ref.clone(),
        operation_ref: transition.operation_ref.clone(),
        before_state_ref: transition.before_state_ref.clone(),
        planned_state_ref: transition.after_state_ref.clone(),
        final_state_ref: final_state.state_ref.clone(),
        revision: final_state.revision,
        status,
        currentness: commit.currentness,
        durability: commit.durability,
        engine_epoch: commit.engine_epoch,
        effect_observations: effects.to_vec(),
        status_ref: status_observation.status_ref.clone(),
        issue: transition.issue.clone(),
        authorizes_future_mutation: false,
        authorizes_effects: false,
        authorizes_retry: false,
        claims_exactly_once: false,
        claims_runtime_survival: false,
        non_claims: required_addressable_actor_non_claims(),
    })
    .map_err(|error| ActorServiceError::Receipt(error.to_string()))
}

fn stale_transition(state: &ActorState, request: &ActorRequest) -> ActorTransition {
    let request_ref = identify_actor_request(request);
    let operation_ref = identify_actor_operation(&request_ref, &request.operation);
    let state_ref = identify_actor_state(state);
    ActorTransition {
        schema: ACTOR_TRANSITION_SCHEMA.to_string(),
        decision: ActorDecision::Denied,
        kind: ActorTransitionKind::DeniedPreserve,
        request_ref,
        operation_ref,
        before_state_ref: state_ref.clone(),
        after_state_ref: state_ref,
        next_state: state.clone(),
        effects: Vec::new(),
        restored_classes: Vec::new(),
        issue: Some(ActorIssue::StaleLifecycleSequence),
        effects_require_fresh_admission: true,
        external_effect_retry_authorized: false,
        receipt_authority: false,
    }
}

fn synthetic_effect_observation(
    effect: &ActorEffectIntent,
    disposition: ActorEffectDisposition,
    label: &str,
) -> ActorEffectObservation {
    let mut hasher =
        blake3::Hasher::new_derive_key("onixresearch.molten.addressable-actor-synthetic-effect-observation.v1");
    hasher.update(effect.effect_ref.as_bytes());
    hasher.update(label.as_bytes());
    ActorEffectObservation {
        effect_ref: effect.effect_ref.clone(),
        admission_ref: format!("blake3:{}", hasher.finalize().to_hex()),
        disposition,
        outcome_ref: None,
    }
}

fn expected_matches(expected: &ExpectedActorState, observed: Option<&PublishedActorState>) -> bool {
    match (&expected.state_ref, observed) {
        (None, None) => expected.revision == ADDRESSABLE_ACTOR_INITIAL_REVISION,
        (Some(expected_ref), Some(observed)) => {
            expected.revision == observed.revision && expected_ref == &observed.state_ref
        }
        _ => false,
    }
}
