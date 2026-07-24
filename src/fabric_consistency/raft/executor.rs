use std::future::Future;
use std::pin::Pin;

use super::*;
use crate::error::Result;

pub type ReplicaTransportFuture<'a> = Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>>;
use crate::fabric_consistency::ConsistencyReadMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplicaEffectKind {
    PersistHardState,
    PersistEntries,
    FlushLog,
    PersistSnapshot,
    Send,
    ArmElectionTimer,
    ArmHeartbeatTimer,
    ApplyCommitted,
    ProposalOutcome,
    ReadOutcome,
    LifecycleChanged,
}

impl ReplicaEffectKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PersistHardState => "persist-hard-state",
            Self::PersistEntries => "persist-entries",
            Self::FlushLog => "flush-log",
            Self::PersistSnapshot => "persist-snapshot",
            Self::Send => "send",
            Self::ArmElectionTimer => "arm-election-timer",
            Self::ArmHeartbeatTimer => "arm-heartbeat-timer",
            Self::ApplyCommitted => "apply-committed",
            Self::ProposalOutcome => "proposal-outcome",
            Self::ReadOutcome => "read-outcome",
            Self::LifecycleChanged => "lifecycle-changed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaEffectObservation {
    pub sequence: u32,
    pub kind: ReplicaEffectKind,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutedReplicaTransition {
    pub next: ReplicaState,
    pub observations: Vec<ReplicaEffectObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailedReplicaTransition {
    pub retained: ReplicaState,
    pub planned: ReplicaState,
    pub completed: Vec<ReplicaEffectObservation>,
    pub failed_kind: ReplicaEffectKind,
    pub diagnostic: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplicaExecutionOutcome {
    Applied(ExecutedReplicaTransition),
    Denied { retained: ReplicaState, diagnostic: String },
    Failed(Box<FailedReplicaTransition>),
}

pub trait ReplicaDurabilityEffects {
    fn persist_hard_state(&mut self, term: u64, voted_for: Option<&str>) -> Result<String>;

    fn persist_entries(&mut self, truncate_from: Option<u64>, entries: &[ReplicatedEntry]) -> Result<String>;

    fn flush_log(&mut self, through_index: u64) -> Result<String>;

    fn persist_snapshot(&mut self, snapshot: &ReplicaSnapshot) -> Result<String>;
}

pub trait ReplicaTransportEffects {
    fn send<'a>(&'a mut self, envelope: &'a ReplicaMessageEnvelope) -> ReplicaTransportFuture<'a>;
}

pub trait ReplicaTimeEffects {
    fn arm_election_timer(&mut self, timer_ref: &str) -> Result<String>;

    fn arm_heartbeat_timer(&mut self) -> Result<String>;
}

pub trait ReplicaApplicationEffects {
    fn apply_committed(&mut self, entries: &[ReplicatedEntry]) -> Result<String>;
}

pub trait ReplicaControlEffects {
    fn proposal_outcome(
        &mut self,
        request_ref: &str,
        disposition: ProposalDisposition,
        committed_index: Option<u64>,
    ) -> Result<String>;

    fn read_outcome(
        &mut self,
        request_ref: &str,
        mode: ConsistencyReadMode,
        disposition: ReadDisposition,
        observed_index: u64,
    ) -> Result<String>;

    fn lifecycle_changed(&mut self, lifecycle: ReplicaLifecycle) -> Result<String>;
}

pub trait LiveReplicaEffectPorts:
    ReplicaDurabilityEffects
    + ReplicaTransportEffects
    + ReplicaTimeEffects
    + ReplicaApplicationEffects
    + ReplicaControlEffects
{
}

impl<T> LiveReplicaEffectPorts for T where T: ReplicaDurabilityEffects
        + ReplicaTransportEffects
        + ReplicaTimeEffects
        + ReplicaApplicationEffects
        + ReplicaControlEffects
{
}

// r[impl molten.fabric_consistency.live_service_ports]
pub async fn execute_scoped_replica_start<P: LiveReplicaEffectPorts>(
    plan: &ReplicaStartPlan,
    ports: &mut P,
) -> ReplicaExecutionOutcome {
    execute_planned_transition(
        &plan.state,
        ReplicaTransition {
            next: plan.state.clone(),
            effects: plan.initial_effects.clone(),
        },
        ports,
    )
    .await
}

// r[impl molten.fabric_consistency.live_service_ports]
pub async fn execute_replica_event<P: LiveReplicaEffectPorts>(
    state: &ReplicaState,
    event: ReplicaEvent,
    ports: &mut P,
) -> ReplicaExecutionOutcome {
    let planned = match apply_replica_event(state, event) {
        Ok(planned) => planned,
        Err(error) => {
            return ReplicaExecutionOutcome::Denied {
                retained: state.clone(),
                diagnostic: error.to_string(),
            };
        }
    };
    execute_planned_transition(state, planned, ports).await
}

async fn execute_planned_transition<P: LiveReplicaEffectPorts>(
    retained: &ReplicaState,
    planned: ReplicaTransition,
    ports: &mut P,
) -> ReplicaExecutionOutcome {
    let mut completed = Vec::with_capacity(planned.effects.len());
    for (index, effect) in planned.effects.iter().enumerate() {
        let kind = effect_kind(effect);
        let evidence_ref = match execute_effect(ports, effect).await {
            Ok(reference) => reference,
            Err(error) => {
                return ReplicaExecutionOutcome::Failed(Box::new(FailedReplicaTransition {
                    retained: retained.clone(),
                    planned: planned.next,
                    completed,
                    failed_kind: kind,
                    diagnostic: error.to_string(),
                }));
            }
        };
        if let Err(error) = crate::preserves_rail::validate_content_ref(&evidence_ref) {
            return ReplicaExecutionOutcome::Failed(Box::new(FailedReplicaTransition {
                retained: retained.clone(),
                planned: planned.next,
                completed,
                failed_kind: kind,
                diagnostic: format!("live Raft effect returned invalid evidence ref: {error}"),
            }));
        }
        let sequence = match u32::try_from(index) {
            Ok(sequence) => sequence,
            Err(_) => {
                return ReplicaExecutionOutcome::Failed(Box::new(FailedReplicaTransition {
                    retained: retained.clone(),
                    planned: planned.next,
                    completed,
                    failed_kind: kind,
                    diagnostic: "live Raft effect sequence exceeds u32".to_string(),
                }));
            }
        };
        completed.push(ReplicaEffectObservation {
            sequence,
            kind,
            evidence_ref,
        });
    }
    ReplicaExecutionOutcome::Applied(ExecutedReplicaTransition {
        next: planned.next,
        observations: completed,
    })
}

async fn execute_effect<P: LiveReplicaEffectPorts>(ports: &mut P, effect: &ReplicaEffect) -> Result<String> {
    match effect {
        ReplicaEffect::PersistHardState { term, voted_for } => ports.persist_hard_state(*term, voted_for.as_deref()),
        ReplicaEffect::PersistEntries { truncate_from, entries } => ports.persist_entries(*truncate_from, entries),
        ReplicaEffect::FlushLog { through_index } => ports.flush_log(*through_index),
        ReplicaEffect::PersistSnapshot { snapshot } => ports.persist_snapshot(snapshot),
        ReplicaEffect::Send { envelope } => ports.send(envelope).await,
        ReplicaEffect::ArmElectionTimer { timer_ref } => ports.arm_election_timer(timer_ref),
        ReplicaEffect::ArmHeartbeatTimer => ports.arm_heartbeat_timer(),
        ReplicaEffect::ApplyCommitted { entries } => ports.apply_committed(entries),
        ReplicaEffect::ProposalOutcome {
            request_ref,
            disposition,
            committed_index,
        } => ports.proposal_outcome(request_ref, *disposition, *committed_index),
        ReplicaEffect::ReadOutcome {
            request_ref,
            mode,
            disposition,
            observed_index,
        } => ports.read_outcome(request_ref, *mode, *disposition, *observed_index),
        ReplicaEffect::LifecycleChanged { lifecycle } => ports.lifecycle_changed(*lifecycle),
    }
}

fn effect_kind(effect: &ReplicaEffect) -> ReplicaEffectKind {
    match effect {
        ReplicaEffect::PersistHardState { .. } => ReplicaEffectKind::PersistHardState,
        ReplicaEffect::PersistEntries { .. } => ReplicaEffectKind::PersistEntries,
        ReplicaEffect::FlushLog { .. } => ReplicaEffectKind::FlushLog,
        ReplicaEffect::PersistSnapshot { .. } => ReplicaEffectKind::PersistSnapshot,
        ReplicaEffect::Send { .. } => ReplicaEffectKind::Send,
        ReplicaEffect::ArmElectionTimer { .. } => ReplicaEffectKind::ArmElectionTimer,
        ReplicaEffect::ArmHeartbeatTimer => ReplicaEffectKind::ArmHeartbeatTimer,
        ReplicaEffect::ApplyCommitted { .. } => ReplicaEffectKind::ApplyCommitted,
        ReplicaEffect::ProposalOutcome { .. } => ReplicaEffectKind::ProposalOutcome,
        ReplicaEffect::ReadOutcome { .. } => ReplicaEffectKind::ReadOutcome,
        ReplicaEffect::LifecycleChanged { .. } => ReplicaEffectKind::LifecycleChanged,
    }
}
