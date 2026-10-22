use molten_core::addressable_actor::*;

use super::*;

const MAX_ACTOR_STATUS_EVENTS: usize = 64;
const UNKNOWN_EFFECT_OPERATION_PREFIX: &str = "unknown-effect:";

#[derive(Clone, Debug)]
pub struct HostBindingFacts {
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
pub struct Request<'a> {
    pub profile: &'a AddressableActorProfile,
    pub actor_key: &'a ActorKey,
    pub host_binding: &'a HostBindingFacts,
    pub expected: ExpectedState,
    pub request: &'a ActorRequest,
    pub requested_engine_epoch: u64,
}

#[derive(Clone, Debug)]
pub struct Outcome {
    pub transition: ActorTransition,
    pub receipt: CanonicalCommitReceipt,
    pub commit_observation: Option<CommitObservation>,
    pub effect_observations: Vec<EffectObservation>,
    pub status_observation: StatusObservation,
    pub final_state: PublishedState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    Port(PortError),
    Profile(ActorIssue),
    Host(ActorIssue),
    Receipt(String),
    Status(ActorIssue),
}

pub type Result<T> = std::result::Result<T, Error>;

// r[impl molten.addressable_actor.lifecycle]
// r[impl molten.addressable_actor.authority]
// r[impl molten.addressable_actor.delivery]
pub fn apply_request(
    commit_port: &mut impl ActorCommitPort,
    effect_port: &mut impl ActorEffectPort,
    status_port: &mut impl ActorStatusPort,
    service: &Request<'_>,
) -> Result<Outcome> {
    validate_profile(service)?;
    let observed = commit_port.load(&service.request.actor_key_ref).map_err(Error::Port)?;
    let state = current_published_state(observed.as_ref(), service);
    validate_host_binding(service, &state)?;
    if !expected_matches(&service.expected, observed.as_ref()) {
        return no_commit_outcome(
            stale_transition(&state, service.request),
            ServiceStatus::Stale,
            service,
            status_port,
        );
    }

    let transition = plan_actor_transition(service.profile, &state, service.request);
    if transition.decision != ActorDecision::Applied {
        let status = no_commit_status(transition.decision);
        return no_commit_outcome(transition, status, service, status_port);
    }

    let planned = PublishedState::from_state(transition.next_state.clone());
    let commit_request = CommitRequest {
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
                if disposition == EffectDisposition::Unknown {
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
    Ok(Outcome {
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
fn current_published_state(observed: Option<&PublishedState>, service: &Request<'_>) -> ActorState {
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

const fn no_commit_status(decision: ActorDecision) -> ServiceStatus {
    match decision {
        ActorDecision::Denied => ServiceStatus::Denied,
        ActorDecision::DuplicateReplay => ServiceStatus::DuplicateReplay,
        ActorDecision::Unknown | ActorDecision::Applied => ServiceStatus::Unknown,
    }
}

/// Admits and executes one effect. Breaks with the stopping status when admission fails or is
/// refused, and otherwise continues with the execution observation, which is failed when it names
/// another effect or admission.
fn admitted_effect_observation(
    effect_port: &mut impl ActorEffectPort,
    effect: &ActorEffectIntent,
) -> std::ops::ControlFlow<(ServiceStatus, EffectObservation), EffectObservation> {
    let admission = match effect_port.observe_admission(effect) {
        Ok(admission) => admission,
        Err(error) => {
            let (status, disposition) = if error.outcome_unknown {
                (ServiceStatus::EffectOutcomeUnknown, EffectDisposition::Unknown)
            } else {
                (ServiceStatus::EffectAdmissionDenied, EffectDisposition::AdmissionDenied)
            };
            return std::ops::ControlFlow::Break((
                status,
                synthetic_effect_observation(effect, disposition, error.code),
            ));
        }
    };
    if !admission.admits(effect) {
        return std::ops::ControlFlow::Break((ServiceStatus::EffectAdmissionDenied, EffectObservation {
            effect_ref: effect.effect_ref.clone(),
            admission_ref: admission.admission_ref,
            disposition: EffectDisposition::AdmissionDenied,
            outcome_ref: None,
        }));
    }
    std::ops::ControlFlow::Continue(match effect_port.execute(effect, &admission) {
        Ok(observation)
            if observation.effect_ref == effect.effect_ref && observation.admission_ref == admission.admission_ref =>
        {
            observation
        }
        Ok(_crossed) => EffectObservation {
            effect_ref: effect.effect_ref.clone(),
            admission_ref: admission.admission_ref,
            disposition: EffectDisposition::Failed,
            outcome_ref: None,
        },
        Err(error) => {
            let disposition = if error.outcome_unknown {
                EffectDisposition::Unknown
            } else {
                EffectDisposition::Failed
            };
            synthetic_effect_observation(effect, disposition, error.code)
        }
    })
}

/// The service status an executed effect's disposition stops the effect sequence with, if it stops
/// it.
const fn stopping_status(disposition: EffectDisposition) -> Option<ServiceStatus> {
    match disposition {
        EffectDisposition::Succeeded => None,
        EffectDisposition::AdmissionDenied => Some(ServiceStatus::EffectAdmissionDenied),
        EffectDisposition::Failed => Some(ServiceStatus::EffectFailed),
        EffectDisposition::Unknown => Some(ServiceStatus::EffectOutcomeUnknown),
    }
}

fn validate_profile(service: &Request<'_>) -> Result<()> {
    if let Some(issue) = validate_addressable_actor_profile(service.profile).into_iter().next() {
        return Err(Error::Profile(issue));
    }
    if let Some(issue) = validate_actor_key(service.actor_key).into_iter().next() {
        return Err(Error::Profile(issue));
    }
    if identify_actor_key(service.actor_key) != service.request.actor_key_ref
        || identify_addressable_actor_profile(service.profile) != service.host_binding.profile_ref
        || service.requested_engine_epoch == 0
    {
        return Err(Error::Profile(ActorIssue::ProfileIdentityMismatch));
    }
    Ok(())
}

fn validate_host_binding(service: &Request<'_>, state: &ActorState) -> Result<()> {
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
        return Err(Error::Host(ActorIssue::AdmissionDenied));
    }
    let extension = molten_core::system_extension::LifecycleState {
        incarnation: 0,
        generation: host.system_extension_generation,
        phase: host.system_extension_phase,
        restart_attempts: 0,
        health: molten_core::system_extension::HealthState::Unknown,
        checkpoint_ref: host.system_extension_checkpoint_ref.clone(),
    };
    if let Some(issue) = validate_system_extension_binding(state, &extension).into_iter().next() {
        return Err(Error::Host(issue));
    }
    Ok(())
}

fn commit_with_reconciliation(
    commit_port: &mut impl ActorCommitPort,
    request: &CommitRequest,
) -> Result<(ServiceStatus, CommitObservation)> {
    match commit_port.compare_and_commit(request) {
        Ok(observation) if observation.disposition == CommitDisposition::Unknown => {
            reconcile_unknown_commit(commit_port, request, Some(observation))
        }
        Ok(observation) => Ok((status_from_observation(&observation, request), observation)),
        Err(error) if error.outcome_unknown => reconcile_unknown_commit(commit_port, request, None),
        Err(error) => Err(Error::Port(error)),
    }
}
