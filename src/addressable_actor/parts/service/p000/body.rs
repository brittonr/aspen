use molten_core::addressable_actor::*;

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
    pub system_extension_phase: molten_core::system_extension::LifecyclePhase,
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
    let state = current_published_state(observed.as_ref(), service);
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
        let status = no_commit_status(transition.decision);
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
            let observation = match admitted_effect_observation(effect_port, effect) {
                std::ops::ControlFlow::Continue(observation) => observation,
                std::ops::ControlFlow::Break((stop_status, observation)) => {
                    status = stop_status;
                    effect_observations.push(observation);
                    break;
                }
            };
            let disposition = observation.disposition;
            effect_observations.push(observation);
            if let Some(stop_status) = stopping_status(disposition) {
                status = stop_status;
                if disposition == ActorEffectDisposition::Unknown {
                    final_state = record_unknown_effect_state(commit_port, service, &final_state, effect)?;
                }
                break;
            }
        }
    }

    let status_observation = publish_status(status_port, &final_state, &effect_observations)?;
    let receipt = build_receipt(ReceiptInput {
        transition: &transition,
        final_state: &final_state,
        status,
        commit: &commit_observation,
        effects: &effect_observations,
        status_observation: &status_observation,
    })?;
    Ok(ActorServiceOutcome {
        transition,
        receipt,
        commit_observation: Some(commit_observation),
        effect_observations,
        status_observation,
        final_state,
    })
}

/// The committed actor state, or a dormant state for the requested actor when nothing is committed
/// yet.
fn current_published_state(observed: Option<&PublishedActorState>, service: &ActorServiceRequest<'_>) -> ActorState {
    observed.map_or_else(
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
    )
}

const fn no_commit_status(decision: ActorDecision) -> ActorServiceStatus {
    match decision {
        ActorDecision::Denied => ActorServiceStatus::Denied,
        ActorDecision::DuplicateReplay => ActorServiceStatus::DuplicateReplay,
        ActorDecision::Unknown | ActorDecision::Applied => ActorServiceStatus::Unknown,
    }
}

/// Admits and executes one effect. Breaks with the stopping status when admission fails or is
/// refused, and otherwise continues with the execution observation, which is failed when it names
/// another effect or admission.
fn admitted_effect_observation(
    effect_port: &mut impl ActorEffectPort,
    effect: &ActorEffectIntent,
) -> std::ops::ControlFlow<(ActorServiceStatus, ActorEffectObservation), ActorEffectObservation> {
    let admission = match effect_port.observe_admission(effect) {
        Ok(admission) => admission,
        Err(error) => {
            let (status, disposition) = if error.outcome_unknown {
                (ActorServiceStatus::EffectOutcomeUnknown, ActorEffectDisposition::Unknown)
            } else {
                (ActorServiceStatus::EffectAdmissionDenied, ActorEffectDisposition::AdmissionDenied)
            };
            return std::ops::ControlFlow::Break((
                status,
                synthetic_effect_observation(effect, disposition, error.code),
            ));
        }
    };
    if !admission.admits(effect) {
        return std::ops::ControlFlow::Break((ActorServiceStatus::EffectAdmissionDenied, ActorEffectObservation {
            effect_ref: effect.effect_ref.clone(),
            admission_ref: admission.admission_ref,
            disposition: ActorEffectDisposition::AdmissionDenied,
            outcome_ref: None,
        }));
    }
    std::ops::ControlFlow::Continue(match effect_port.execute(effect, &admission) {
        Ok(observation)
            if observation.effect_ref == effect.effect_ref && observation.admission_ref == admission.admission_ref =>
        {
            observation
        }
        Ok(_crossed) => ActorEffectObservation {
            effect_ref: effect.effect_ref.clone(),
            admission_ref: admission.admission_ref,
            disposition: ActorEffectDisposition::Failed,
            outcome_ref: None,
        },
        Err(error) => {
            let disposition = if error.outcome_unknown {
                ActorEffectDisposition::Unknown
            } else {
                ActorEffectDisposition::Failed
            };
            synthetic_effect_observation(effect, disposition, error.code)
        }
    })
}

/// The service status an executed effect's disposition stops the effect sequence with, if it stops
/// it.
const fn stopping_status(disposition: ActorEffectDisposition) -> Option<ActorServiceStatus> {
    match disposition {
        ActorEffectDisposition::Succeeded => None,
        ActorEffectDisposition::AdmissionDenied => Some(ActorServiceStatus::EffectAdmissionDenied),
        ActorEffectDisposition::Failed => Some(ActorServiceStatus::EffectFailed),
        ActorEffectDisposition::Unknown => Some(ActorServiceStatus::EffectOutcomeUnknown),
    }
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
    let extension = molten_core::system_extension::LifecycleState {
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
