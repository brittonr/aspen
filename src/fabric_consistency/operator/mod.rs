mod canonical;

#[cfg(test)]
mod tests;

use super::*;
use crate::fabric_consistency::raft::BoundLiveReplicaEffectPorts;

pub const MAX_OPERATOR_EVIDENCE_REFS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Create,
    Inspect,
    Drain,
    Snapshot,
    Recover,
    Remove,
}

impl Action {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Inspect => "inspect",
            Self::Drain => "drain",
            Self::Snapshot => "snapshot",
            Self::Recover => "recover",
            Self::Remove => "remove",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub command: PortCommandInput,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preflight {
    pub action: Action,
    pub dry_run: bool,
    pub plan: PortPlan,
    pub preflight_ref: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionStatus {
    DryRun,
    Applied,
    Denied,
}

impl ExecutionStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DryRun => "dry-run",
            Self::Applied => "applied",
            Self::Denied => "denied",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Execution {
    pub action: Action,
    pub status: ExecutionStatus,
    pub plan_ref: String,
    pub effect_ref: Option<String>,
    pub execution_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaState {
    pub group_binding_ref: String,
    pub service_generation: u64,
    pub node_id: String,
    pub role: crate::fabric_consistency::raft::ReplicaRole,
    pub lifecycle: crate::fabric_consistency::raft::ReplicaLifecycle,
    pub term: u64,
    pub commit_index: u64,
    pub last_applied: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Readback {
    pub binding_ref: String,
    pub group_id: String,
    pub extension_id: String,
    pub service_id: String,
    pub service_generation: u64,
    pub lifecycle: GroupLifecycle,
    pub node_id: String,
    pub role: crate::fabric_consistency::raft::ReplicaRole,
    pub replica_lifecycle: crate::fabric_consistency::raft::ReplicaLifecycle,
    pub term: u64,
    pub commit_index: u64,
    pub last_applied: u64,
    pub selected_evidence_refs: Vec<String>,
    pub evidence_truncated: bool,
    pub aggregate_health_ref: String,
    pub production_admitted: bool,
    pub non_claims: Vec<String>,
    pub readback_ref: String,
}

pub trait ConsistencyOperatorEffects {
    fn apply(&mut self, action: Action, plan: &PortPlan) -> crate::error::Result<String>;
}

// r[impl molten.fabric_consistency.operator_readback]
pub fn plan_action(binding: &Group, request: Request) -> crate::error::Result<Preflight> {
    let action = action_for_operation(&request.command.operation)?;
    let plan = plan(binding, request.command)?;
    let preflight_ref = canonical::preflight_ref(action, request.dry_run, &plan)?;
    Ok(Preflight {
        action,
        dry_run: request.dry_run,
        plan,
        preflight_ref,
    })
}

// r[impl molten.fabric_consistency.operator_readback]
pub fn execute_action<E: ConsistencyOperatorEffects>(
    preflight: &Preflight,
    effects: &mut E,
) -> crate::error::Result<Execution> {
    if preflight.preflight_ref != canonical::preflight_ref(preflight.action, preflight.dry_run, &preflight.plan)? {
        return Err(crate::error::Failure::invalid_harness("consistency operator preflight identity mismatch"));
    }
    let (status, effect_ref) = if !preflight.plan.admitted() {
        (ExecutionStatus::Denied, None)
    } else if preflight.dry_run {
        (ExecutionStatus::DryRun, None)
    } else {
        let effect_ref = effects.apply(preflight.action, &preflight.plan)?;
        crate::preserves_rail::validate_content_ref(&effect_ref)?;
        (ExecutionStatus::Applied, Some(effect_ref))
    };
    let execution_ref = canonical::execution_ref(preflight, status, effect_ref.as_deref())?;
    Ok(Execution {
        action: preflight.action,
        status,
        plan_ref: preflight.plan.plan_ref.clone(),
        effect_ref,
        execution_ref,
    })
}

// r[impl molten.fabric_consistency.operator_readback]
pub fn readback(
    binding: &Group,
    replica: &ReplicaState,
    records: &[crate::fabric_consistency::raft::ReplicaRecord],
    health: &crate::fabric_consistency::raft::ReplicaAggregateHealth,
) -> crate::error::Result<Readback> {
    validate_readback_binding(binding, replica, health)?;
    let is_evidence_truncated = records.len() > MAX_OPERATOR_EVIDENCE_REFS;
    let mut selected_evidence_refs = records
        .iter()
        .rev()
        .take(MAX_OPERATOR_EVIDENCE_REFS)
        .map(|record| record.evidence_ref.clone())
        .collect::<Vec<_>>();
    selected_evidence_refs.reverse();
    let readback_ref =
        canonical::readback_ref(binding, replica, &selected_evidence_refs, is_evidence_truncated, health)?;
    Ok(Readback {
        binding_ref: binding.binding_ref.clone(),
        group_id: binding.group_id.clone(),
        extension_id: binding.extension_id.clone(),
        service_id: binding.service_id.clone(),
        service_generation: binding.service_generation,
        lifecycle: binding.lifecycle,
        node_id: replica.node_id.clone(),
        role: replica.role,
        replica_lifecycle: replica.lifecycle,
        term: replica.term,
        commit_index: replica.commit_index,
        last_applied: replica.last_applied,
        selected_evidence_refs,
        evidence_truncated: is_evidence_truncated,
        aggregate_health_ref: health.evidence_ref.clone(),
        production_admitted: health.production_admitted,
        non_claims: binding.non_claims.clone(),
        readback_ref,
    })
}

// r[impl molten.fabric_consistency.operator_readback]
pub fn live_replica_readback<P: BoundLiveReplicaEffectPorts>(
    binding: &Group,
    service: &crate::fabric_consistency::raft::ScopedLiveReplica<P>,
) -> crate::error::Result<Readback> {
    let state = service.state();
    let replica = ReplicaState {
        group_binding_ref: state.profile.group_binding_ref.clone(),
        service_generation: state.profile.service_generation,
        node_id: state.node_id.clone(),
        role: state.role,
        lifecycle: state.lifecycle,
        term: state.current_term,
        commit_index: state.commit_index,
        last_applied: state.last_applied,
    };
    let health = service.aggregate_health_evidence()?;
    readback(binding, &replica, service.evidence().records(), &health)
}

fn action_for_operation(operation: &crate::fabric_consistency::operation::Request) -> crate::error::Result<Action> {
    match operation {
        crate::fabric_consistency::operation::Request::Open {
            mode: GroupOpenMode::Create,
        } => Ok(Action::Create),
        crate::fabric_consistency::operation::Request::Status => Ok(Action::Inspect),
        crate::fabric_consistency::operation::Request::Drain => Ok(Action::Drain),
        crate::fabric_consistency::operation::Request::Snapshot { .. } => Ok(Action::Snapshot),
        crate::fabric_consistency::operation::Request::Recover { .. } => Ok(Action::Recover),
        crate::fabric_consistency::operation::Request::Remove => Ok(Action::Remove),
        _ => Err(crate::error::Failure::invalid_harness(
            "operation is outside the bounded consistency operator surface",
        )),
    }
}

fn validate_readback_binding(
    binding: &Group,
    replica: &ReplicaState,
    health: &crate::fabric_consistency::raft::ReplicaAggregateHealth,
) -> crate::error::Result<()> {
    if replica.group_binding_ref != binding.binding_ref || replica.service_generation != binding.service_generation {
        return Err(crate::error::Failure::invalid_harness("consistency operator readback binding mismatch"));
    }
    if replica.last_applied > replica.commit_index {
        return Err(crate::error::Failure::invalid_harness(
            "consistency operator readback applied index exceeds commit",
        ));
    }
    crate::preserves_rail::validate_content_ref(&health.evidence_ref)
}
