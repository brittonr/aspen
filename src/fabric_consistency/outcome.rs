use preserves::IOValue;

use super::ConsistencyGroupBinding;
use super::ConsistencyOperation;
use super::ConsistencyPlanDecision;
use super::ConsistencyPortPlan;
use super::ConsistencyReadMode;
use super::MAX_CONSISTENCY_DIAGNOSTICS;
use super::MAX_CONSISTENCY_EVIDENCE_REFS;
use super::binding::validate_content_ref;
use super::binding::validate_content_refs;
use super::binding::validate_identifier;
use super::canonical::outcome_value;
use crate::error::MoltenError;
use crate::error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsistencyOutcomeKind {
    Opened,
    Committed,
    ReadCurrent,
    ReadLocal,
    SnapshotCreated,
    Recovered,
    ConfigurationApplied,
    HealthObserved,
    Drained,
    StatusObserved,
    Removed,
    Denied,
    Retryable,
    Cancelled,
    Uncertain,
}

impl ConsistencyOutcomeKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Opened => "opened",
            Self::Committed => "committed",
            Self::ReadCurrent => "read-current",
            Self::ReadLocal => "read-local",
            Self::SnapshotCreated => "snapshot-created",
            Self::Recovered => "recovered",
            Self::ConfigurationApplied => "configuration-applied",
            Self::HealthObserved => "health-observed",
            Self::Drained => "drained",
            Self::StatusObserved => "status-observed",
            Self::Removed => "removed",
            Self::Denied => "denied",
            Self::Retryable => "retryable",
            Self::Cancelled => "cancelled",
            Self::Uncertain => "uncertain",
        }
    }

    pub(crate) const fn is_non_mutating_failure(self) -> bool {
        matches!(self, Self::Denied | Self::Retryable | Self::Cancelled | Self::Uncertain)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsistencyOutcomeInput {
    pub request_ref: String,
    pub binding_ref: String,
    pub service_generation: u64,
    pub config_epoch: u64,
    pub fencing_epoch: u64,
    pub kind: ConsistencyOutcomeKind,
    pub result_ref: Option<String>,
    pub evidence_refs: Vec<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsistencyPortOutcome {
    pub outcome_ref: String,
    pub plan_ref: String,
    pub request_ref: String,
    pub binding_ref: String,
    pub kind: ConsistencyOutcomeKind,
    pub result_ref: Option<String>,
    pub evidence_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

// r[impl molten.fabric_consistency.extension_port]
// r[impl molten.fabric_consistency.group_isolation]
pub fn normalize_consistency_outcome(
    binding: &ConsistencyGroupBinding,
    plan: &ConsistencyPortPlan,
    input: ConsistencyOutcomeInput,
) -> Result<ConsistencyPortOutcome> {
    validate_outcome_input(&input)?;
    validate_outcome_binding(binding, plan, &input)?;
    validate_outcome_kind(plan, input.kind)?;
    validate_outcome_shape(&input)?;
    let value = outcome_value(&plan.plan_ref, binding, &input, input.kind);
    let outcome_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(ConsistencyPortOutcome {
        outcome_ref,
        plan_ref: plan.plan_ref.clone(),
        request_ref: input.request_ref,
        binding_ref: input.binding_ref,
        kind: input.kind,
        result_ref: input.result_ref,
        evidence_refs: input.evidence_refs,
        diagnostics: input.diagnostics,
        value,
    })
}

fn validate_outcome_input(input: &ConsistencyOutcomeInput) -> Result<()> {
    validate_content_ref(&input.request_ref, "consistency outcome request ref")?;
    validate_content_ref(&input.binding_ref, "consistency outcome binding ref")?;
    if let Some(result_ref) = &input.result_ref {
        validate_content_ref(result_ref, "consistency outcome result ref")?;
    }
    validate_content_refs(
        &input.evidence_refs,
        MAX_CONSISTENCY_EVIDENCE_REFS,
        "consistency outcome evidence refs",
        true,
    )?;
    if input.diagnostics.len() > MAX_CONSISTENCY_DIAGNOSTICS {
        return Err(MoltenError::invalid_harness("consistency outcome diagnostics exceed the bounded maximum"));
    }
    for diagnostic in &input.diagnostics {
        validate_identifier(diagnostic, "consistency outcome diagnostic")?;
    }
    Ok(())
}

fn validate_outcome_binding(
    binding: &ConsistencyGroupBinding,
    plan: &ConsistencyPortPlan,
    input: &ConsistencyOutcomeInput,
) -> Result<()> {
    if plan.request_ref != input.request_ref
        || plan.binding_ref != input.binding_ref
        || binding.binding_ref != input.binding_ref
        || binding.service_generation != input.service_generation
        || binding.config_epoch != input.config_epoch
        || binding.fencing_epoch != input.fencing_epoch
    {
        return Err(MoltenError::invalid_harness("consistency outcome binding, generation, or epoch mismatch"));
    }
    Ok(())
}

fn validate_outcome_kind(plan: &ConsistencyPortPlan, kind: ConsistencyOutcomeKind) -> Result<()> {
    if plan.decision == ConsistencyPlanDecision::Denied {
        if kind == ConsistencyOutcomeKind::Denied {
            return Ok(());
        }
        return Err(MoltenError::invalid_harness("denied consistency plan cannot produce a non-denial outcome"));
    }
    let compatible = kind.is_non_mutating_failure()
        || matches!(
            (&plan.operation, kind),
            (ConsistencyOperation::Open { .. }, ConsistencyOutcomeKind::Opened)
                | (ConsistencyOperation::Propose { .. }, ConsistencyOutcomeKind::Committed)
                | (
                    ConsistencyOperation::Read {
                        mode: ConsistencyReadMode::Linearizable,
                        ..
                    },
                    ConsistencyOutcomeKind::ReadCurrent,
                )
                | (
                    ConsistencyOperation::Read {
                        mode: ConsistencyReadMode::LocalStale,
                        ..
                    },
                    ConsistencyOutcomeKind::ReadLocal,
                )
                | (ConsistencyOperation::Snapshot { .. }, ConsistencyOutcomeKind::SnapshotCreated,)
                | (ConsistencyOperation::Recover { .. }, ConsistencyOutcomeKind::Recovered)
                | (ConsistencyOperation::Configure { .. }, ConsistencyOutcomeKind::ConfigurationApplied,)
                | (ConsistencyOperation::Health, ConsistencyOutcomeKind::HealthObserved)
                | (ConsistencyOperation::Drain, ConsistencyOutcomeKind::Drained)
                | (ConsistencyOperation::Status, ConsistencyOutcomeKind::StatusObserved)
                | (ConsistencyOperation::Remove, ConsistencyOutcomeKind::Removed)
        );
    if compatible {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness("consistency outcome kind does not match the admitted operation"))
    }
}

fn validate_outcome_shape(input: &ConsistencyOutcomeInput) -> Result<()> {
    if input.kind.is_non_mutating_failure() {
        if input.diagnostics.is_empty() || input.result_ref.is_some() {
            return Err(MoltenError::invalid_harness("failure outcome requires diagnostics and excludes a result ref"));
        }
    } else if input.result_ref.is_none() || !input.diagnostics.is_empty() {
        return Err(MoltenError::invalid_harness("successful outcome requires a result ref and no diagnostics"));
    }
    Ok(())
}
