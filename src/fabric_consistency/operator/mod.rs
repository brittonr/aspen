mod canonical;

#[cfg(test)]
mod tests;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;
use crate::fabric_consistency::raft::BoundLiveReplicaEffectPorts;
use crate::fabric_consistency::raft::ReplicaAggregateHealthEvidence;
use crate::fabric_consistency::raft::ReplicaEvidenceRecord;
use crate::fabric_consistency::raft::ReplicaLifecycle;
use crate::fabric_consistency::raft::ReplicaRole;
use crate::fabric_consistency::raft::ScopedLiveReplicaService;

pub const MAX_OPERATOR_EVIDENCE_REFS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsistencyOperatorAction {
    Create,
    Inspect,
    Drain,
    Snapshot,
    Recover,
    Remove,
}

impl ConsistencyOperatorAction {
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
pub struct ConsistencyOperatorRequest {
    pub command: ConsistencyPortCommandInput,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsistencyOperatorPreflight {
    pub action: ConsistencyOperatorAction,
    pub dry_run: bool,
    pub plan: ConsistencyPortPlan,
    pub preflight_ref: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsistencyOperatorExecutionStatus {
    DryRun,
    Applied,
    Denied,
}

impl ConsistencyOperatorExecutionStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DryRun => "dry-run",
            Self::Applied => "applied",
            Self::Denied => "denied",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsistencyOperatorExecution {
    pub action: ConsistencyOperatorAction,
    pub status: ConsistencyOperatorExecutionStatus,
    pub plan_ref: String,
    pub effect_ref: Option<String>,
    pub execution_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsistencyOperatorReplicaState {
    pub group_binding_ref: String,
    pub service_generation: u64,
    pub node_id: String,
    pub role: ReplicaRole,
    pub lifecycle: ReplicaLifecycle,
    pub term: u64,
    pub commit_index: u64,
    pub last_applied: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsistencyOperatorReadback {
    pub binding_ref: String,
    pub group_id: String,
    pub extension_id: String,
    pub service_id: String,
    pub service_generation: u64,
    pub lifecycle: ConsistencyGroupLifecycle,
    pub node_id: String,
    pub role: ReplicaRole,
    pub replica_lifecycle: ReplicaLifecycle,
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
    fn apply(&mut self, action: ConsistencyOperatorAction, plan: &ConsistencyPortPlan) -> Result<String>;
}

// r[impl molten.fabric_consistency.operator_readback]
pub fn plan_consistency_operator_action(
    binding: &ConsistencyGroupBinding,
    request: ConsistencyOperatorRequest,
) -> Result<ConsistencyOperatorPreflight> {
    let action = action_for_operation(&request.command.operation)?;
    let plan = plan_consistency_operation(binding, request.command)?;
    let preflight_ref = canonical::preflight_ref(action, request.dry_run, &plan)?;
    Ok(ConsistencyOperatorPreflight {
        action,
        dry_run: request.dry_run,
        plan,
        preflight_ref,
    })
}

// r[impl molten.fabric_consistency.operator_readback]
pub fn execute_consistency_operator_action<E: ConsistencyOperatorEffects>(
    preflight: &ConsistencyOperatorPreflight,
    effects: &mut E,
) -> Result<ConsistencyOperatorExecution> {
    if preflight.preflight_ref != canonical::preflight_ref(preflight.action, preflight.dry_run, &preflight.plan)? {
        return Err(MoltenError::invalid_harness("consistency operator preflight identity mismatch"));
    }
    let (status, effect_ref) = if !preflight.plan.admitted() {
        (ConsistencyOperatorExecutionStatus::Denied, None)
    } else if preflight.dry_run {
        (ConsistencyOperatorExecutionStatus::DryRun, None)
    } else {
        let effect_ref = effects.apply(preflight.action, &preflight.plan)?;
        crate::preserves_rail::validate_content_ref(&effect_ref)?;
        (ConsistencyOperatorExecutionStatus::Applied, Some(effect_ref))
    };
    let execution_ref = canonical::execution_ref(preflight, status, effect_ref.as_deref())?;
    Ok(ConsistencyOperatorExecution {
        action: preflight.action,
        status,
        plan_ref: preflight.plan.plan_ref.clone(),
        effect_ref,
        execution_ref,
    })
}

// r[impl molten.fabric_consistency.operator_readback]
pub fn consistency_operator_readback(
    binding: &ConsistencyGroupBinding,
    replica: &ConsistencyOperatorReplicaState,
    records: &[ReplicaEvidenceRecord],
    health: &ReplicaAggregateHealthEvidence,
) -> Result<ConsistencyOperatorReadback> {
    validate_readback_binding(binding, replica, health)?;
    let evidence_truncated = records.len() > MAX_OPERATOR_EVIDENCE_REFS;
    let mut selected_evidence_refs = records
        .iter()
        .rev()
        .take(MAX_OPERATOR_EVIDENCE_REFS)
        .map(|record| record.evidence_ref.clone())
        .collect::<Vec<_>>();
    selected_evidence_refs.reverse();
    let readback_ref = canonical::readback_ref(binding, replica, &selected_evidence_refs, evidence_truncated, health)?;
    Ok(ConsistencyOperatorReadback {
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
        evidence_truncated,
        aggregate_health_ref: health.evidence_ref.clone(),
        production_admitted: health.production_admitted,
        non_claims: binding.non_claims.clone(),
        readback_ref,
    })
}

// r[impl molten.fabric_consistency.operator_readback]
pub fn live_replica_operator_readback<P: BoundLiveReplicaEffectPorts>(
    binding: &ConsistencyGroupBinding,
    service: &ScopedLiveReplicaService<P>,
) -> Result<ConsistencyOperatorReadback> {
    let state = service.state();
    let replica = ConsistencyOperatorReplicaState {
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
    consistency_operator_readback(binding, &replica, service.evidence().records(), &health)
}

fn action_for_operation(operation: &ConsistencyOperation) -> Result<ConsistencyOperatorAction> {
    match operation {
        ConsistencyOperation::Open {
            mode: GroupOpenMode::Create,
        } => Ok(ConsistencyOperatorAction::Create),
        ConsistencyOperation::Status => Ok(ConsistencyOperatorAction::Inspect),
        ConsistencyOperation::Drain => Ok(ConsistencyOperatorAction::Drain),
        ConsistencyOperation::Snapshot { .. } => Ok(ConsistencyOperatorAction::Snapshot),
        ConsistencyOperation::Recover { .. } => Ok(ConsistencyOperatorAction::Recover),
        ConsistencyOperation::Remove => Ok(ConsistencyOperatorAction::Remove),
        _ => Err(MoltenError::invalid_harness("operation is outside the bounded consistency operator surface")),
    }
}

fn validate_readback_binding(
    binding: &ConsistencyGroupBinding,
    replica: &ConsistencyOperatorReplicaState,
    health: &ReplicaAggregateHealthEvidence,
) -> Result<()> {
    if replica.group_binding_ref != binding.binding_ref || replica.service_generation != binding.service_generation {
        return Err(MoltenError::invalid_harness("consistency operator readback binding mismatch"));
    }
    if replica.last_applied > replica.commit_index {
        return Err(MoltenError::invalid_harness("consistency operator readback applied index exceeds commit"));
    }
    crate::preserves_rail::validate_content_ref(&health.evidence_ref)
}
