use std::collections::VecDeque;

use molten_core::addressable_actor::*;
use molten_core::system_extension::LifecyclePhase;

use super::super::*;

pub(super) const ENGINE_EPOCH: u64 = 7;
pub(super) const INITIAL_TICK: u64 = 10;
pub(super) const IDLE_TICK: u64 = 20;
pub(super) const IDLE_AFTER_TICKS: u64 = 5;
pub(super) const MAXIMUM_DRAIN_ITEMS: u32 = 16;

pub(super) fn reference(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}

pub(super) fn profile() -> AddressableActorProfile {
    AddressableActorProfile {
        schema: ACTOR_PROFILE_SCHEMA.to_string(),
        profile_id: "addressable-actor-v1".to_string(),
        profile_version: ADDRESSABLE_ACTOR_PROFILE_VERSION,
        reference_source: ActorReferenceSource {
            repository: RIVET_ACTORS_REPOSITORY.to_string(),
            revision: RIVET_ACTORS_REVISION.to_string(),
            license: RIVET_ACTORS_LICENSE.to_string(),
            selected_concepts: vec![
                "keyed-addressability".to_string(),
                "generation-fenced-runtime".to_string(),
                "sleep-intent-and-rewake-separation".to_string(),
                "persisted-state-and-scheduled-events".to_string(),
                "runtime-and-durable-survival-separation".to_string(),
            ],
        },
        system_extension_profile_ref: reference("system-extension-profile"),
        placement_profile_ref: reference("placement-profile"),
        delivery_profile_ref: reference("delivery-profile"),
        durable_state_profile_ref: reference("durable-state-profile"),
        time_profile_ref: reference("time-profile"),
        resource_profile_ref: reference("resource-profile"),
        supervision_profile_ref: reference("supervision-profile"),
        authority_profile_ref: reference("authority-profile"),
        evidence_profile_ref: reference("evidence-profile"),
        idle_after_ticks: IDLE_AFTER_TICKS,
        maximum_drain_items: MAXIMUM_DRAIN_ITEMS,
        survival: standard_actor_survival_matrix(),
        non_claims: required_addressable_actor_non_claims(),
    }
}

pub(super) fn actor_key() -> ActorKey {
    ActorKey {
        schema: ACTOR_KEY_SCHEMA.to_string(),
        namespace_ref: reference("namespace"),
        actor_type: "workspace-agent".to_string(),
        key: "tenant:alpha/workspace:one".to_string(),
    }
}

pub(super) fn initial_state(profile: &AddressableActorProfile, actor_key: &ActorKey) -> ActorState {
    ActorState::dormant(
        identify_actor_key(actor_key),
        identify_addressable_actor_profile(profile),
        reference("system-extension-manifest"),
        reference("placement"),
        ADDRESSABLE_ACTOR_INITIAL_GENERATION,
    )
}

pub(super) fn admission(state: &ActorState) -> ActorAdmissionFacts {
    ActorAdmissionFacts {
        profile_ref: state.profile_ref.clone(),
        system_extension_manifest_ref: state.system_extension_manifest_ref.clone(),
        authority_ref: reference("authority"),
        resource_ref: reference("resource"),
        adapter_ref: reference("adapter"),
        policy_current: true,
        capability_current: true,
        placement_current: true,
        generation_current: true,
        resources_admitted: true,
        adapter_admitted: true,
    }
}

pub(super) fn request(
    state: &ActorState,
    operation_id: &str,
    logical_tick: u64,
    operation: ActorOperation,
) -> ActorRequest {
    ActorRequest {
        schema: ACTOR_REQUEST_SCHEMA.to_string(),
        operation_id: operation_id.to_string(),
        actor_key_ref: state.actor_key_ref.clone(),
        placement_ref: state.placement_ref.clone(),
        extension_generation: state.extension_generation,
        expected_lifecycle_sequence: state.lifecycle_sequence,
        logical_tick,
        admission: admission(state),
        operation,
    }
}

pub(super) fn message_wake() -> WakeReason {
    WakeReason::Message {
        delivery_item_ref: reference("delivery-item"),
        delivery_token_ref: reference("delivery-token"),
    }
}

pub(super) fn empty_expected() -> ExpectedActorState {
    ExpectedActorState {
        state_ref: None,
        revision: ADDRESSABLE_ACTOR_INITIAL_REVISION,
    }
}

pub(super) fn expected(published: &PublishedActorState) -> ExpectedActorState {
    ExpectedActorState {
        state_ref: Some(published.state_ref.clone()),
        revision: published.revision,
    }
}

pub(super) fn host_binding(profile: &AddressableActorProfile, state: &ActorState) -> ActorHostBindingFacts {
    ActorHostBindingFacts {
        schema: ACTOR_HOST_BINDING_SCHEMA.to_string(),
        actor_key_ref: state.actor_key_ref.clone(),
        profile_ref: state.profile_ref.clone(),
        system_extension_manifest_ref: state.system_extension_manifest_ref.clone(),
        placement_ref: state.placement_ref.clone(),
        extension_generation: state.extension_generation,
        system_extension_generation: state.extension_generation,
        system_extension_phase: extension_phase(state.phase),
        system_extension_checkpoint_ref: state.checkpoint_ref.clone(),
        delivery_profile_ref: profile.delivery_profile_ref.clone(),
        policy_current: true,
        capability_current: true,
        placement_current: true,
        resources_admitted: true,
        adapter_admitted: true,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CommitMode {
    Apply,
    UnknownBefore,
    UnknownAfter,
    Stale,
}

pub(super) struct MemoryCommitPort {
    pub(super) head: Option<PublishedActorState>,
    pub(super) mode: CommitMode,
    pub(super) compare_calls: u32,
}

impl MemoryCommitPort {
    pub(super) const fn new(mode: CommitMode) -> Self {
        Self {
            head: None,
            mode,
            compare_calls: 0,
        }
    }
}

impl ActorCommitPort for MemoryCommitPort {
    fn load(&self, _actor_key_ref: &str) -> ActorPortResult<Option<PublishedActorState>> {
        Ok(self.head.clone())
    }

    fn compare_and_commit(&mut self, request: &ActorCommitRequest) -> ActorPortResult<ActorCommitObservation> {
        self.compare_calls = self.compare_calls.saturating_add(1);
        match self.mode {
            CommitMode::Apply => {
                self.head = Some(request.next.clone());
                Ok(commit_observation(ActorCommitDisposition::Applied, Some(request.next.state_ref.clone())))
            }
            CommitMode::UnknownBefore => Err(ActorPortError::new("scripted-unknown", "unknown before apply", true)),
            CommitMode::UnknownAfter => {
                self.head = Some(request.next.clone());
                Err(ActorPortError::new("scripted-unknown", "unknown after apply", true))
            }
            CommitMode::Stale => Ok(commit_observation(
                ActorCommitDisposition::Stale,
                self.head.as_ref().map(|head| head.state_ref.clone()),
            )),
        }
    }
}

fn commit_observation(disposition: ActorCommitDisposition, state_ref: Option<String>) -> ActorCommitObservation {
    ActorCommitObservation {
        disposition,
        currentness: ActorCommitCurrentness::Linearizable,
        durability: ActorDurabilityOutcome::Durable,
        engine_epoch: ENGINE_EPOCH,
        observed_state_ref: state_ref,
    }
}

pub(super) struct MemoryEffectPort {
    pub(super) scripted: VecDeque<ActorEffectDisposition>,
    pub(super) deny_admission_at: Option<usize>,
    pub(super) admission_calls: usize,
    pub(super) execution_calls: usize,
}

impl MemoryEffectPort {
    pub(super) fn succeeding(count: usize) -> Self {
        Self {
            scripted: std::iter::repeat_n(ActorEffectDisposition::Succeeded, count).collect(),
            deny_admission_at: None,
            admission_calls: 0,
            execution_calls: 0,
        }
    }

    pub(super) fn scripted(dispositions: impl IntoIterator<Item = ActorEffectDisposition>) -> Self {
        Self {
            scripted: dispositions.into_iter().collect(),
            deny_admission_at: None,
            admission_calls: 0,
            execution_calls: 0,
        }
    }
}

impl ActorEffectPort for MemoryEffectPort {
    fn observe_admission(&mut self, effect: &ActorEffectIntent) -> ActorPortResult<ActorEffectAdmissionObservation> {
        let call = self.admission_calls;
        self.admission_calls = self.admission_calls.saturating_add(1);
        Ok(ActorEffectAdmissionObservation {
            admission_ref: reference(&format!("admission:{call}:{}", effect.effect_ref)),
            actor_key_ref: effect.actor_key_ref.clone(),
            profile_ref: effect.profile_ref.clone(),
            system_extension_manifest_ref: effect.system_extension_manifest_ref.clone(),
            placement_ref: effect.placement_ref.clone(),
            extension_generation: effect.extension_generation,
            policy_current: true,
            capability_current: true,
            placement_current: true,
            generation_current: self.deny_admission_at != Some(call),
            resources_admitted: true,
            adapter_admitted: true,
        })
    }

    fn execute(
        &mut self,
        effect: &ActorEffectIntent,
        admission: &ActorEffectAdmissionObservation,
    ) -> ActorPortResult<ActorEffectObservation> {
        self.execution_calls = self.execution_calls.saturating_add(1);
        let disposition = self.scripted.pop_front().unwrap_or(ActorEffectDisposition::Failed);
        Ok(ActorEffectObservation {
            effect_ref: effect.effect_ref.clone(),
            admission_ref: admission.admission_ref.clone(),
            disposition,
            outcome_ref: matches!(disposition, ActorEffectDisposition::Succeeded)
                .then(|| reference(&format!("outcome:{}", effect.effect_ref))),
        })
    }
}

#[derive(Default)]
pub(super) struct MemoryStatusPort {
    pub(super) status_refs: Vec<String>,
}

impl ActorStatusPort for MemoryStatusPort {
    fn publish_status(&mut self, status: &ActorStatus) -> ActorPortResult<ActorStatusObservation> {
        let status_ref = identify_canonical_actor_status(status)
            .map_err(|error| ActorPortError::new("actor-status", error.to_string(), false))?;
        self.status_refs.push(status_ref.clone());
        Ok(ActorStatusObservation {
            status_ref: Some(status_ref),
            outcome_unknown: false,
        })
    }
}
