pub(super) enum Backend {
    Live(Box<crate::fabric_durability::RedbDurableStateAdapter>),
    Simulation {
        adapter: Box<crate::fabric_durability::SimulatedDurableStateAdapter>,
        next_fault: Option<crate::fabric_durability::SimulatedFault>,
        is_uncertain: bool,
    },
}
impl Backend {
    pub(super) fn state(&self) -> &crate::fabric_durability::DurableState {
        match self {
            Self::Live(adapter) => adapter.state(),
            Self::Simulation { adapter, .. } => adapter.state(),
        }
    }
    pub(super) fn profile_ref(&self) -> &str {
        match self {
            Self::Live(adapter) => &adapter.profile().profile_ref,
            Self::Simulation { adapter, .. } => &adapter.state().descriptor.profile_ref,
        }
    }
    pub(super) fn health(&self) -> crate::fabric_durability::AdapterHealth {
        match self {
            Self::Live(adapter) => adapter.health(),
            Self::Simulation { is_uncertain: true, .. } => crate::fabric_durability::AdapterHealth::Quarantined(
                crate::fabric_durability::QuarantineReason::OutcomeUnknown,
            ),
            Self::Simulation { .. } => crate::fabric_durability::AdapterHealth::Serving,
        }
    }
    pub(super) fn apply_batch(
        &mut self,
        request: &crate::fabric_durability::AtomicBatchRequest,
    ) -> crate::error::Result<crate::fabric_durability::TransitionEnvelope> {
        match self {
            Self::Live(adapter) => adapter.apply_batch(request),
            Self::Simulation {
                adapter,
                next_fault,
                is_uncertain,
            } => {
                let fault = next_fault.take();
                let outcome = adapter.apply_batch(request, fault.as_ref())?;
                *is_uncertain = matches!(
                    outcome.outcome,
                    crate::fabric_durability::MutationOutcome::Uncertain
                        | crate::fabric_durability::MutationOutcome::FailedAfterPossibleMutation
                );
                Ok(outcome)
            }
        }
    }
    pub(super) fn inject(&mut self, fault: crate::fabric_durability::SimulatedFault) -> crate::error::Result<()> {
        let Self::Simulation {
            adapter, next_fault, ..
        } = self
        else {
            return Err(super::denied(super::Denial::Domain));
        };
        if matches!(
            fault,
            crate::fabric_durability::SimulatedFault::ProcessCrash
                | crate::fabric_durability::SimulatedFault::DelayCompletion { .. }
                | crate::fabric_durability::SimulatedFault::CorruptSnapshot { .. }
        ) {
            adapter.inject_fault(&fault)?;
        } else {
            *next_fault = Some(fault);
        }
        Ok(())
    }
}

impl Backend {
    pub(super) fn mode(&self) -> &str {
        match self {
            Self::Live(adapter) => crate::fabric_durability::DurableCommandShell::profile_id(adapter.as_ref()),
            Self::Simulation { adapter, .. } => {
                crate::fabric_durability::DurableCommandShell::profile_id(adapter.as_ref())
            }
        }
    }
}
