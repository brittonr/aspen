use molten_core::addressable_actor::*;
use molten_core::system_extension::LifecyclePhase;
use molten_core::system_extension::LifecycleState;

use super::*;

const MAX_ACTOR_STATUS_EVENTS: usize = 64;
const UNKNOWN_EFFECT_OPERATION_PREFIX: &str = "unknown-effect:";

#[derive(Clone, Debug)]
pub struct ActorHostBindingFacts {
    pub schema: String,
    pub actor_key_ref: String,
    pub profile_ref: String,
    pub system_extension_manifest_ref: String,
    pub placement_ref: String,
    pub extension_generation: u64,
    pub system_extension_generation: u64,
    pub system_extension_phase: LifecyclePhase,
    pub system_extension_checkpoint_ref: Option<String>,
    pub delivery_profile_ref: String,
    pub policy_current: bool,
    pub capability_current: bool,
    pub placement_current: bool,
    pub resources_admitted: bool,
    pub adapter_admitted: bool,
}

#[derive(Clone, Debug)]
pub struct ActorServiceRequest<'a> {
    pub profile: &'a AddressableActorProfile,
    pub actor_key: &'a ActorKey,
    pub host_binding: &'a ActorHostBindingFacts,
    pub expected: ExpectedActorState,
    pub request: &'a ActorRequest,
    pub requested_engine_epoch: u64,
}

#[derive(Clone, Debug)]
pub struct ActorServiceOutcome {
    pub transition: ActorTransition,
    pub receipt: CanonicalActorCommitReceipt,
    pub commit_observation: Option<ActorCommitObservation>,
    pub effect_observations: Vec<ActorEffectObservation>,
    pub status_observation: ActorStatusObservation,
    pub final_state: PublishedActorState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActorServiceError {
    Port(ActorPortError),
    Profile(ActorIssue),
    Host(ActorIssue),
    Receipt(String),
    Status(ActorIssue),
}

pub type ActorServiceResult<T> = std::result::Result<T, ActorServiceError>;

// r[impl molten.addressable_actor.lifecycle]
// r[impl molten.addressable_actor.authority]
// r[impl molten.addressable_actor.delivery]
pub fn apply_actor_request(
    commit_port: &mut impl ActorCommitPort,
    effect_port: &mut impl ActorEffectPort,
    status_port: &mut impl ActorStatusPort,
    service: &ActorServiceRequest<'_>,
) -> ActorServiceResult<ActorServiceOutcome> {
    validate_service_profile(service)?;
    let observed = commit_port.load(&service.request.actor_key_ref).map_err(ActorServiceError::Port)?;
    let state = observed.as_ref().map_or_else(
        || {
            ActorState::dormant(
                service.request.actor_key_ref.clone(),
                service.host_binding.profile_ref.clone(),
                service.host_binding.system_extension_manifest_ref.clone(),
                service.request.placement_ref.clone(),
                service.request.extension_generation,
            )
        },
        |published| published.state.clone(),
    );
    validate_host_binding(service, &state)?;
    if !expected_matches(&service.expected, observed.as_ref()) {
        return no_commit_outcome(
            stale_transition(&state, service.request),
            ActorServiceStatus::Stale,
            service,
            status_port,
        );
    }

    let transition = plan_actor_transition(service.profile, &state, service.request);
    if transition.decision != ActorDecision::Applied {
        let status = match transition.decision {
            ActorDecision::Denied => ActorServiceStatus::Denied,
            ActorDecision::DuplicateReplay => ActorServiceStatus::DuplicateReplay,
            ActorDecision::Unknown => ActorServiceStatus::Unknown,
            ActorDecision::Applied => ActorServiceStatus::Unknown,
        };
        return no_commit_outcome(transition, status, service, status_port);
    }

    let planned = PublishedActorState::from_state(transition.next_state.clone());
    let commit_request = ActorCommitRequest {
        actor_key_ref: service.request.actor_key_ref.clone(),
        expected: service.expected.clone(),
        next: planned.clone(),
        requested_engine_epoch: service.requested_engine_epoch,
    };
    let (mut status, commit_observation) = commit_with_reconciliation(commit_port, &commit_request)?;
    let mut final_state = planned;
    let mut effect_observations = Vec::new();
    if status.commit_confirmed() {
        for effect in &transition.effects {
            let admission = match effect_port.observe_admission(effect) {
                Ok(admission) => admission,
                Err(error) => {
                    status = if error.outcome_unknown {
                        ActorServiceStatus::EffectOutcomeUnknown
                    } else {
                        ActorServiceStatus::EffectAdmissionDenied
                    };
                    effect_observations.push(synthetic_effect_observation(
                        effect,
                        if error.outcome_unknown {
                            ActorEffectDisposition::Unknown
                        } else {
                            ActorEffectDisposition::AdmissionDenied
                        },
                        error.code,
                    ));
                    break;
                }
            };
            if !admission.admits(effect) {
                status = ActorServiceStatus::EffectAdmissionDenied;
                effect_observations.push(ActorEffectObservation {
                    effect_ref: effect.effect_ref.clone(),
                    admission_ref: admission.admission_ref,
                    disposition: ActorEffectDisposition::AdmissionDenied,
                    outcome_ref: None,
                });
                break;
            }
            let observation = match effect_port.execute(effect, &admission) {
                Ok(observation)
                    if observation.effect_ref == effect.effect_ref
                        && observation.admission_ref == admission.admission_ref =>
                {
                    observation
                }
                Ok(_crossed) => ActorEffectObservation {
                    effect_ref: effect.effect_ref.clone(),
                    admission_ref: admission.admission_ref,
                    disposition: ActorEffectDisposition::Failed,
                    outcome_ref: None,
                },
                Err(error) => synthetic_effect_observation(
                    effect,
                    if error.outcome_unknown {
                        ActorEffectDisposition::Unknown
                    } else {
                        ActorEffectDisposition::Failed
                    },
                    error.code,
                ),
            };
            let disposition = observation.disposition;
            effect_observations.push(observation);
            match disposition {
                ActorEffectDisposition::Succeeded => {}
                ActorEffectDisposition::AdmissionDenied => {
                    status = ActorServiceStatus::EffectAdmissionDenied;
                    break;
                }
                ActorEffectDisposition::Failed => {
                    status = ActorServiceStatus::EffectFailed;
                    break;
                }
                ActorEffectDisposition::Unknown => {
                    status = ActorServiceStatus::EffectOutcomeUnknown;
                    final_state = record_unknown_effect_state(commit_port, service, &final_state, effect)?;
                    break;
                }
            }
        }
    }

    let status_observation = publish_status(status_port, &final_state, &effect_observations)?;
    let receipt = build_receipt(
        &transition,
        &final_state,
        status,
        &commit_observation,
        &effect_observations,
        &status_observation,
    )?;
    Ok(ActorServiceOutcome {
        transition,
        receipt,
        commit_observation: Some(commit_observation),
        effect_observations,
        status_observation,
        final_state,
    })
}

fn validate_service_profile(service: &ActorServiceRequest<'_>) -> ActorServiceResult<()> {
    if let Some(issue) = validate_addressable_actor_profile(service.profile).into_iter().next() {
        return Err(ActorServiceError::Profile(issue));
    }
    if let Some(issue) = validate_actor_key(service.actor_key).into_iter().next() {
        return Err(ActorServiceError::Profile(issue));
    }
    if identify_actor_key(service.actor_key) != service.request.actor_key_ref
        || identify_addressable_actor_profile(service.profile) != service.host_binding.profile_ref
        || service.requested_engine_epoch == 0
    {
        return Err(ActorServiceError::Profile(ActorIssue::ProfileIdentityMismatch));
    }
    Ok(())
}

fn validate_host_binding(service: &ActorServiceRequest<'_>, state: &ActorState) -> ActorServiceResult<()> {
    let host = service.host_binding;
    if host.schema != ACTOR_HOST_BINDING_SCHEMA
        || host.actor_key_ref != state.actor_key_ref
        || host.profile_ref != state.profile_ref
        || host.system_extension_manifest_ref != state.system_extension_manifest_ref
        || host.placement_ref != state.placement_ref
        || host.extension_generation != state.extension_generation
        || host.delivery_profile_ref != service.profile.delivery_profile_ref
        || !host.policy_current
        || !host.capability_current
        || !host.placement_current
        || !host.resources_admitted
        || !host.adapter_admitted
    {
        return Err(ActorServiceError::Host(ActorIssue::AdmissionDenied));
    }
    let extension = LifecycleState {
        generation: host.system_extension_generation,
        phase: host.system_extension_phase,
        restart_attempts: 0,
        health: molten_core::system_extension::HealthState::Unknown,
        checkpoint_ref: host.system_extension_checkpoint_ref.clone(),
    };
    if let Some(issue) = validate_system_extension_binding(state, &extension).into_iter().next() {
        return Err(ActorServiceError::Host(issue));
    }
    Ok(())
}

fn commit_with_reconciliation(
    commit_port: &mut impl ActorCommitPort,
    request: &ActorCommitRequest,
) -> ActorServiceResult<(ActorServiceStatus, ActorCommitObservation)> {
    match commit_port.compare_and_commit(request) {
        Ok(observation) if observation.disposition == ActorCommitDisposition::Unknown => {
            reconcile_unknown_commit(commit_port, request, Some(observation))
        }
        Ok(observation) => Ok((status_from_observation(&observation, request), observation)),
        Err(error) if error.outcome_unknown => reconcile_unknown_commit(commit_port, request, None),
        Err(error) => Err(ActorServiceError::Port(error)),
    }
}

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
    let receipt = build_receipt(&transition, &final_state, status, &commit, &effect_observations, &status_observation)?;
    Ok(ActorServiceOutcome {
        transition,
        receipt,
        commit_observation: None,
        effect_observations,
        status_observation,
        final_state,
    })
}

fn build_receipt(
    transition: &ActorTransition,
    final_state: &PublishedActorState,
    status: ActorServiceStatus,
    commit: &ActorCommitObservation,
    effects: &[ActorEffectObservation],
    status_observation: &ActorStatusObservation,
) -> ActorServiceResult<CanonicalActorCommitReceipt> {
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
