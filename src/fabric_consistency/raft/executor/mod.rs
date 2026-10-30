use std::future::Future;

use super::*;

mod effects;
#[cfg(test)]
mod intent;

pub type ReplicaTransportFuture<'a> = std::pin::Pin<Box<dyn Future<Output = crate::error::Result<String>> + Send + 'a>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplicaEffectKind {
    PersistHardState,
    PersistEntries,
    FlushLog,
    PersistCommit,
    PersistSnapshot,
    Send,
    ArmElectionTimer,
    ArmHeartbeatTimer,
    RestoreApplicationSnapshot,
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
            Self::PersistCommit => "persist-commit",
            Self::PersistSnapshot => "persist-snapshot",
            Self::Send => "send",
            Self::ArmElectionTimer => "arm-election-timer",
            Self::ArmHeartbeatTimer => "arm-heartbeat-timer",
            Self::RestoreApplicationSnapshot => "restore-application-snapshot",
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

/// A transport attempt failed; unlike effect observations, this carries no success evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaSendFailureObservation {
    pub sequence: u32,
    pub diagnostic: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutedReplicaTransition {
    pub next: ReplicaState,
    pub observations: Vec<ReplicaEffectObservation>,
    pub transport_failures: Vec<ReplicaSendFailureObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailedReplicaTransition {
    pub retained: ReplicaState,
    pub planned: ReplicaState,
    pub completed: Vec<ReplicaEffectObservation>,
    pub transport_failures: Vec<ReplicaSendFailureObservation>,
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
    fn persist_hard_state(
        &mut self,
        term: u64,
        voted_for: Option<&str>,
        is_recovery_required: bool,
    ) -> crate::error::Result<String>;

    fn persist_entries(
        &mut self,
        truncate_from: Option<u64>,
        entries: &[ReplicatedEntry],
    ) -> crate::error::Result<String>;

    fn flush_log(&mut self, through_index: u64) -> crate::error::Result<String>;

    fn persist_commit(&mut self, through_index: u64) -> crate::error::Result<String>;

    fn persist_snapshot(
        &mut self,
        snapshot: &ReplicaSnapshot,
        is_suffix_retained: bool,
    ) -> crate::error::Result<String>;
}

pub trait ReplicaTransportEffects {
    fn send<'a>(&'a mut self, envelope: &'a ReplicaMessageEnvelope) -> ReplicaTransportFuture<'a>;
}

pub trait ReplicaTimeEffects {
    fn arm_election_timer(&mut self, timer_ref: &str) -> crate::error::Result<String>;

    fn arm_heartbeat_timer(&mut self) -> crate::error::Result<String>;
}

pub trait ReplicaApplicationEffects {
    fn restore_snapshot(&mut self, snapshot: &ReplicaSnapshot) -> crate::error::Result<String>;

    fn apply_committed(&mut self, entries: &[ReplicatedEntry]) -> crate::error::Result<String>;
}

pub trait ReplicaControlEffects {
    fn proposal_outcome(
        &mut self,
        request_ref: &str,
        disposition: ProposalDisposition,
        committed_index: Option<u64>,
    ) -> crate::error::Result<String>;

    fn read_outcome(
        &mut self,
        request_ref: &str,
        mode: crate::fabric_consistency::ReadMode,
        disposition: ReadDisposition,
        observed_index: u64,
    ) -> crate::error::Result<String>;

    fn lifecycle_changed(&mut self, lifecycle: ReplicaLifecycle) -> crate::error::Result<String>;
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

struct IntentAdmission {
    is_guarded: bool,
    has_prepend: bool,
    effect_count: usize,
}

async fn execute_planned_transition<P: LiveReplicaEffectPorts>(
    retained: &ReplicaState,
    planned: ReplicaTransition,
    ports: &mut P,
) -> ReplicaExecutionOutcome {
    // Borrow the planned payloads: fencing must not duplicate commands or entries.
    let is_guarded = planned.effects.iter().any(effects::requires_durable_intent);
    let has_prepend = is_guarded && !matches!(planned.effects.first(), Some(ReplicaEffect::PersistHardState { .. }));
    let has_clear = is_guarded && !matches!(planned.next.rejoin, ReplicaRejoin::Recovering(_));
    let effect_count = planned
        .effects
        .len()
        .saturating_add(usize::from(has_prepend))
        .saturating_add(usize::from(has_clear));
    if effect_count > retained.profile.max_effects_per_step || u32::try_from(effect_count).is_err() {
        return ReplicaExecutionOutcome::Denied {
            retained: retained.clone(),
            diagnostic: "live Raft durable intent exceeds the admitted effect bound".to_string(),
        };
    }
    execute_intent_batch(retained, planned, ports, IntentAdmission {
        is_guarded,
        has_prepend,
        effect_count,
    })
    .await
}

async fn execute_intent_batch<P: LiveReplicaEffectPorts>(
    retained: &ReplicaState,
    planned: ReplicaTransition,
    ports: &mut P,
    admission: IntentAdmission,
) -> ReplicaExecutionOutcome {
    let mut completed = Vec::with_capacity(admission.effect_count);
    let mut transport_failures = Vec::with_capacity(admission.effect_count);
    for index in 0..admission.effect_count {
        let effect =
            index.checked_sub(usize::from(admission.has_prepend)).and_then(|offset| planned.effects.get(offset));
        let kind = effect.map_or(ReplicaEffectKind::PersistHardState, effects::effect_kind);
        // Admission checked the entire fenced batch fits the sequence domain.
        let sequence = index as u32;
        let result = match effect {
            Some(effect) => effects::execute_effect(ports, effect, admission.is_guarded).await,
            None if admission.has_prepend && index == 0 => {
                ports.persist_hard_state(retained.current_term, retained.voted_for.as_deref(), true)
            }
            None => ports.persist_hard_state(planned.next.current_term, planned.next.voted_for.as_deref(), false),
        };
        let evidence_ref = match result {
            Ok(reference) => reference,
            Err(error) if kind == ReplicaEffectKind::Send => {
                // Raft tolerates message loss. Keep the planned authority and run later
                // effects, especially timers, without claiming successful delivery.
                if transport_failures.len() < admission.effect_count {
                    transport_failures.push(ReplicaSendFailureObservation {
                        sequence,
                        diagnostic: effects::bounded_transport_diagnostic(&error),
                    });
                }
                continue;
            }
            Err(error) => {
                return ReplicaExecutionOutcome::Failed(Box::new(FailedReplicaTransition {
                    retained: effects::quarantine(retained),
                    planned: planned.next,
                    completed,
                    transport_failures,
                    failed_kind: kind,
                    diagnostic: error.to_string(),
                }));
            }
        };
        if let Err(error) = crate::preserves_rail::validate_content_ref(&evidence_ref) {
            return ReplicaExecutionOutcome::Failed(Box::new(FailedReplicaTransition {
                retained: effects::quarantine(retained),
                planned: planned.next,
                completed,
                transport_failures,
                failed_kind: kind,
                diagnostic: format!("live Raft effect returned invalid evidence ref: {error}"),
            }));
        }
        completed.push(ReplicaEffectObservation {
            sequence,
            kind,
            evidence_ref,
        });
    }
    ReplicaExecutionOutcome::Applied(ExecutedReplicaTransition {
        next: planned.next,
        observations: completed,
        transport_failures,
    })
}
