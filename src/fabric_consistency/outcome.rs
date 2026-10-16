#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
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

impl Kind {
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
pub struct Input {
    pub request_ref: String,
    pub binding_ref: String,
    pub service_generation: u64,
    pub config_epoch: u64,
    pub fencing_epoch: u64,
    pub kind: Kind,
    pub result_ref: Option<String>,
    pub evidence_refs: Vec<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Port {
    pub outcome_ref: String,
    pub plan_ref: String,
    pub request_ref: String,
    pub binding_ref: String,
    pub kind: Kind,
    pub result_ref: Option<String>,
    pub evidence_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: preserves::IOValue,
}

// r[impl molten.fabric_consistency.extension_port]
// r[impl molten.fabric_consistency.group_isolation]
pub fn normalize(binding: &super::Group, plan: &super::PortPlan, input: Input) -> crate::error::Result<Port> {
    validate_input(&input)?;
    validate_binding(binding, plan, &input)?;
    validate_outcome_kind(plan, input.kind)?;
    validate_shape(&input)?;
    let value = super::canonical::outcome_value(&plan.plan_ref, binding, &input, input.kind);
    let outcome_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(Port {
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

fn validate_input(input: &Input) -> crate::error::Result<()> {
    super::binding::validate_content_ref(&input.request_ref, "consistency outcome request ref")?;
    super::binding::validate_content_ref(&input.binding_ref, "consistency outcome binding ref")?;
    if let Some(result_ref) = &input.result_ref {
        super::binding::validate_content_ref(result_ref, "consistency outcome result ref")?;
    }
    super::binding::validate_content_refs(
        &input.evidence_refs,
        super::MAX_CONSISTENCY_EVIDENCE_REFS,
        "consistency outcome evidence refs",
        true,
    )?;
    if input.diagnostics.len() > super::MAX_CONSISTENCY_DIAGNOSTICS {
        return Err(crate::error::Failure::invalid_harness(
            "consistency outcome diagnostics exceed the bounded maximum",
        ));
    }
    for diagnostic in &input.diagnostics {
        super::binding::validate_identifier(diagnostic, "consistency outcome diagnostic")?;
    }
    Ok(())
}

fn validate_binding(binding: &super::Group, plan: &super::PortPlan, input: &Input) -> crate::error::Result<()> {
    if plan.request_ref != input.request_ref
        || plan.binding_ref != input.binding_ref
        || binding.binding_ref != input.binding_ref
        || binding.service_generation != input.service_generation
        || binding.config_epoch != input.config_epoch
        || binding.fencing_epoch != input.fencing_epoch
    {
        return Err(crate::error::Failure::invalid_harness(
            "consistency outcome binding, generation, or epoch mismatch",
        ));
    }
    Ok(())
}

fn validate_outcome_kind(plan: &super::PortPlan, kind: Kind) -> crate::error::Result<()> {
    if plan.decision == super::PlanDecision::Denied {
        if kind == Kind::Denied {
            return Ok(());
        }
        return Err(crate::error::Failure::invalid_harness(
            "denied consistency plan cannot produce a non-denial outcome",
        ));
    }
    let is_compatible = kind.is_non_mutating_failure()
        || matches!(
            (&plan.operation, kind),
            (super::Request::Open { .. }, Kind::Opened)
                | (super::Request::Propose { .. }, Kind::Committed)
                | (
                    super::Request::Read {
                        mode: super::ReadMode::Linearizable,
                        ..
                    },
                    Kind::ReadCurrent,
                )
                | (
                    super::Request::Read {
                        mode: super::ReadMode::LocalStale,
                        ..
                    },
                    Kind::ReadLocal,
                )
                | (super::Request::Snapshot { .. }, Kind::SnapshotCreated,)
                | (super::Request::Recover { .. }, Kind::Recovered)
                | (super::Request::Configure { .. }, Kind::ConfigurationApplied,)
                | (super::Request::Health, Kind::HealthObserved)
                | (super::Request::Drain, Kind::Drained)
                | (super::Request::Status, Kind::StatusObserved)
                | (super::Request::Remove, Kind::Removed)
        );
    if is_compatible {
        Ok(())
    } else {
        Err(crate::error::Failure::invalid_harness(
            "consistency outcome kind does not match the admitted operation",
        ))
    }
}

fn validate_shape(input: &Input) -> crate::error::Result<()> {
    if input.kind.is_non_mutating_failure() {
        if input.diagnostics.is_empty() || input.result_ref.is_some() {
            return Err(crate::error::Failure::invalid_harness(
                "failure outcome requires diagnostics and excludes a result ref",
            ));
        }
    } else if input.result_ref.is_none() || !input.diagnostics.is_empty() {
        return Err(crate::error::Failure::invalid_harness(
            "successful outcome requires a result ref and no diagnostics",
        ));
    }
    Ok(())
}
