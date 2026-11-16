use std::future::Future;

use super::*;

mod batch;
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

/// What a snapshot persistence leaves in place, taken from the transition's planned state: the hard
/// state and the log entries above the snapshot boundary. A durability port that compacts records
/// the snapshot covers rewrites exactly this state before it truncates them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplicaRetainedState<'a> {
    pub term: u64,
    pub voted_for: Option<&'a str>,
    pub entries: &'a [ReplicatedEntry],
}

impl<'a> ReplicaRetainedState<'a> {
    pub fn after(planned: &'a ReplicaState, snapshot: &ReplicaSnapshot) -> Self {
        let start = planned.log.partition_point(|entry| entry.index <= snapshot.last_included_index);
        Self {
            term: planned.current_term,
            voted_for: planned.voted_for.as_deref(),
            entries: &planned.log[start..],
        }
    }
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
        retained: &ReplicaRetainedState<'_>,
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
    batch::execute_planned_transition(
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
    batch::execute_planned_transition(state, planned, ports).await
}
