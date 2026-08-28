use std::collections::VecDeque;

use molten_core::addressable_actor::*;
use molten_core::system_extension::LifecyclePhase;

use super::*;

const SIMULATION_ENGINE_EPOCH: u64 = 1;

#[derive(Clone, Debug)]
pub struct ActorSimulationStep {
    pub operation_id: String,
    pub logical_tick: u64,
    pub operation: ActorOperation,
}

#[derive(Clone, Debug)]
pub struct ActorSimulationReport {
    pub receipt_refs: Vec<String>,
    pub final_state: PublishedActorState,
    pub effect_observations: Vec<ActorEffectObservation>,
    pub status_refs: Vec<String>,
    pub deterministic: bool,
    pub authorizes_production: bool,
}

// r[impl molten.addressable_actor.verification]
pub fn simulate_actor_sequence(
    profile: &AddressableActorProfile,
    actor_key: &ActorKey,
    system_extension_manifest_ref: &str,
    placement_ref: &str,
    steps: &[ActorSimulationStep],
    scripted_effects: &[ActorEffectDisposition],
) -> ActorServiceResult<ActorSimulationReport> {
    let actor_key_ref = identify_actor_key(actor_key);
    let profile_ref = identify_addressable_actor_profile(profile);
    let mut commit = SimulationCommitPort::default();
    let mut effects = SimulationEffectPort {
        scripted: scripted_effects.iter().copied().collect(),
    };
    let mut statuses = SimulationStatusPort::default();
    let mut receipt_refs = Vec::new();
    let mut effect_observations = Vec::new();

    for step in steps {
        let current = commit.state.as_ref().map_or_else(
            || {
                ActorState::dormant(
                    actor_key_ref.clone(),
                    profile_ref.clone(),
                    system_extension_manifest_ref.to_string(),
                    placement_ref.to_string(),
                    ADDRESSABLE_ACTOR_INITIAL_GENERATION,
                )
            },
            |published| published.state.clone(),
        );
        let expected = commit.state.as_ref().map_or(
            ExpectedActorState {
                state_ref: None,
                revision: ADDRESSABLE_ACTOR_INITIAL_REVISION,
            },
            |published| ExpectedActorState {
                state_ref: Some(published.state_ref.clone()),
                revision: published.revision,
            },
        );
        let admission = ActorAdmissionFacts {
            profile_ref: current.profile_ref.clone(),
            system_extension_manifest_ref: current.system_extension_manifest_ref.clone(),
            authority_ref: simulation_ref("authority"),
            resource_ref: simulation_ref("resource"),
            adapter_ref: simulation_ref("adapter"),
            policy_current: true,
            capability_current: true,
            placement_current: true,
            generation_current: true,
            resources_admitted: true,
            adapter_admitted: true,
        };
        let request = ActorRequest {
            schema: ACTOR_REQUEST_SCHEMA.to_string(),
            operation_id: step.operation_id.clone(),
            actor_key_ref: current.actor_key_ref.clone(),
            placement_ref: current.placement_ref.clone(),
            extension_generation: current.extension_generation,
            expected_lifecycle_sequence: current.lifecycle_sequence,
            logical_tick: step.logical_tick,
            admission,
            operation: step.operation.clone(),
        };
        let host_binding = ActorHostBindingFacts {
            schema: ACTOR_HOST_BINDING_SCHEMA.to_string(),
            actor_key_ref: current.actor_key_ref.clone(),
            profile_ref: current.profile_ref.clone(),
            system_extension_manifest_ref: current.system_extension_manifest_ref.clone(),
            placement_ref: current.placement_ref.clone(),
            extension_generation: current.extension_generation,
            system_extension_generation: current.extension_generation,
            system_extension_phase: extension_phase(current.phase),
            system_extension_checkpoint_ref: current.checkpoint_ref.clone(),
            delivery_profile_ref: profile.delivery_profile_ref.clone(),
            policy_current: true,
            capability_current: true,
            placement_current: true,
            resources_admitted: true,
            adapter_admitted: true,
        };
        let outcome = apply_actor_request(&mut commit, &mut effects, &mut statuses, &ActorServiceRequest {
            profile,
            actor_key,
            host_binding: &host_binding,
            expected,
            request: &request,
            requested_engine_epoch: SIMULATION_ENGINE_EPOCH,
        })?;
        receipt_refs.push(outcome.receipt.receipt_ref);
        effect_observations.extend(outcome.effect_observations);
    }
    if !effects.scripted.is_empty() {
        return Err(ActorServiceError::Port(ActorPortError::new(
            "actor-simulation-script",
            "unused scripted effect outcomes remain",
            false,
        )));
    }
    let final_state = commit.state.ok_or_else(|| {
        ActorServiceError::Port(ActorPortError::new(
            "actor-simulation-state",
            "simulation produced no committed actor state",
            false,
        ))
    })?;
    Ok(ActorSimulationReport {
        receipt_refs,
        final_state,
        effect_observations,
        status_refs: statuses.refs,
        deterministic: true,
        authorizes_production: false,
    })
}

#[derive(Default)]
struct SimulationCommitPort {
    state: Option<PublishedActorState>,
}

impl ActorCommitPort for SimulationCommitPort {
    fn load(&self, actor_key_ref: &str) -> ActorPortResult<Option<PublishedActorState>> {
        if self.state.as_ref().is_some_and(|published| published.state.actor_key_ref != actor_key_ref) {
            return Err(ActorPortError::new("actor-simulation-key", "simulation actor key crossed state", false));
        }
        Ok(self.state.clone())
    }

    fn compare_and_commit(&mut self, request: &ActorCommitRequest) -> ActorPortResult<ActorCommitObservation> {
        if !simulation_expected_matches(&request.expected, self.state.as_ref()) {
            return Ok(ActorCommitObservation {
                disposition: ActorCommitDisposition::Stale,
                currentness: ActorCommitCurrentness::Linearizable,
                durability: ActorDurabilityOutcome::Durable,
                engine_epoch: SIMULATION_ENGINE_EPOCH,
                observed_state_ref: self.state.as_ref().map(|published| published.state_ref.clone()),
            });
        }
        if self.state.as_ref() == Some(&request.next) {
            return Ok(ActorCommitObservation {
                disposition: ActorCommitDisposition::AlreadyApplied,
                currentness: ActorCommitCurrentness::Linearizable,
                durability: ActorDurabilityOutcome::Durable,
                engine_epoch: SIMULATION_ENGINE_EPOCH,
                observed_state_ref: Some(request.next.state_ref.clone()),
            });
        }
        self.state = Some(request.next.clone());
        Ok(ActorCommitObservation {
            disposition: ActorCommitDisposition::Applied,
            currentness: ActorCommitCurrentness::Linearizable,
            durability: ActorDurabilityOutcome::Durable,
            engine_epoch: SIMULATION_ENGINE_EPOCH,
            observed_state_ref: Some(request.next.state_ref.clone()),
        })
    }
}

struct SimulationEffectPort {
    scripted: VecDeque<ActorEffectDisposition>,
}

impl ActorEffectPort for SimulationEffectPort {
    fn observe_admission(&mut self, effect: &ActorEffectIntent) -> ActorPortResult<ActorEffectAdmissionObservation> {
        Ok(ActorEffectAdmissionObservation {
            admission_ref: simulation_ref(&effect.effect_ref),
            actor_key_ref: effect.actor_key_ref.clone(),
            profile_ref: effect.profile_ref.clone(),
            system_extension_manifest_ref: effect.system_extension_manifest_ref.clone(),
            placement_ref: effect.placement_ref.clone(),
            extension_generation: effect.extension_generation,
            policy_current: true,
            capability_current: true,
            placement_current: true,
            generation_current: true,
            resources_admitted: true,
            adapter_admitted: true,
        })
    }

    fn execute(
        &mut self,
        effect: &ActorEffectIntent,
        admission: &ActorEffectAdmissionObservation,
    ) -> ActorPortResult<ActorEffectObservation> {
        let disposition = self.scripted.pop_front().ok_or_else(|| {
            ActorPortError::new("actor-simulation-script", "scripted effect outcome is missing", false)
        })?;
        Ok(ActorEffectObservation {
            effect_ref: effect.effect_ref.clone(),
            admission_ref: admission.admission_ref.clone(),
            disposition,
            outcome_ref: matches!(disposition, ActorEffectDisposition::Succeeded)
                .then(|| simulation_ref(&format!("outcome:{}", effect.effect_ref))),
        })
    }
}

#[derive(Default)]
struct SimulationStatusPort {
    refs: Vec<String>,
}

impl ActorStatusPort for SimulationStatusPort {
    fn publish_status(&mut self, status: &ActorStatus) -> ActorPortResult<ActorStatusObservation> {
        let status_ref = identify_canonical_actor_status(status)
            .map_err(|error| ActorPortError::new("actor-simulation-status", error.to_string(), false))?;
        self.refs.push(status_ref.clone());
        Ok(ActorStatusObservation {
            status_ref: Some(status_ref),
            outcome_unknown: false,
        })
    }
}

fn extension_phase(phase: ActorPhase) -> LifecyclePhase {
    match phase {
        ActorPhase::Dormant => LifecyclePhase::Drained,
        ActorPhase::Starting => LifecyclePhase::Starting,
        ActorPhase::Running => LifecyclePhase::Running,
        ActorPhase::Draining => LifecyclePhase::Draining,
        ActorPhase::Stopped => LifecyclePhase::Stopped,
        ActorPhase::Degraded => LifecyclePhase::Failed,
        ActorPhase::Recovering => LifecyclePhase::Recovering,
    }
}

fn simulation_expected_matches(expected: &ExpectedActorState, observed: Option<&PublishedActorState>) -> bool {
    match (&expected.state_ref, observed) {
        (None, None) => expected.revision == ADDRESSABLE_ACTOR_INITIAL_REVISION,
        (Some(expected_ref), Some(observed)) => {
            expected.revision == observed.revision && expected_ref == &observed.state_ref
        }
        _ => false,
    }
}

fn simulation_ref(label: &str) -> String {
    let mut hasher = blake3::Hasher::new_derive_key("onixresearch.molten.addressable-actor-simulation-reference.v1");
    hasher.update(label.as_bytes());
    format!("blake3:{}", hasher.finalize().to_hex())
}
