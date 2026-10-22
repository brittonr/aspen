// The time-sliced execution contract.
//
// The time-sliced component profile binds a scheduling quantum and an
// independent total execution budget. This module owns the pure contract
// around that binding: the slice ledger that records the quantum boundaries,
// the pending turn that publishes nothing before the callback completes, and
// the continuation that the admitted scheduler re-selects after a yield.
//
// The guest-side yielding under the pinned cohort's async entrypoints is not
// implemented here: the component's `invoke` export is a synchronous WIT
// export, and the cohort's fuel-based yielding needs the async entrypoints
// that an async component ABI would introduce. This module is the contract
// that execution must satisfy.

/// One recorded quantum boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SliceBoundary {
    pub index: usize,
    pub fuel_consumed: u64,
}

/// The recorded quantum boundaries and the consumed budget of one callback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliceLedger {
    pub quantum_fuel: u64,
    pub total_budget_fuel: u64,
    pub boundaries: Vec<SliceBoundary>,
    pub consumed_fuel: u64,
    next_boundary_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliceIssue {
    ZeroQuantum,
    ZeroBudget,
    BudgetBelowQuantum,
    BoundaryOverflow,
    BudgetExceeded,
}

/// A replay divergence, named by the boundary where the recorded and the
/// recomputed paths differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SliceReplayMismatch {
    pub boundary_index: usize,
    pub recorded_fuel: u64,
    pub recomputed_fuel: u64,
}

// r[impl molten.wasm_component.yield_continuation]
impl SliceLedger {
    /// The ledger for one sliced callback.
    pub fn new(quantum_fuel: u64, total_budget_fuel: u64) -> Result<Self, SliceIssue> {
        if quantum_fuel == 0 {
            return Err(SliceIssue::ZeroQuantum);
        }
        if total_budget_fuel == 0 {
            return Err(SliceIssue::ZeroBudget);
        }
        if total_budget_fuel < quantum_fuel {
            return Err(SliceIssue::BudgetBelowQuantum);
        }
        Ok(Self {
            quantum_fuel,
            total_budget_fuel,
            boundaries: Vec::new(),
            consumed_fuel: 0,
            next_boundary_index: 0,
        })
    }

    /// Record the fuel consumed by one slice.
    pub fn record_boundary(&mut self, fuel_consumed: u64) -> Result<SliceBoundary, SliceIssue> {
        let index = self.next_boundary_index;
        let consumed = self.consumed_fuel.checked_add(fuel_consumed).ok_or(SliceIssue::BudgetExceeded)?;
        if consumed > self.total_budget_fuel {
            return Err(SliceIssue::BudgetExceeded);
        }
        self.consumed_fuel = consumed;
        self.next_boundary_index = self.next_boundary_index.checked_add(1).ok_or(SliceIssue::BoundaryOverflow)?;
        let boundary = SliceBoundary { index, fuel_consumed };
        self.boundaries.push(boundary);
        Ok(boundary)
    }

    /// Whether the total execution budget is spent.
    pub fn is_budget_exhausted(&self) -> bool {
        self.consumed_fuel >= self.total_budget_fuel
    }

    /// Compare the recomputed slicing with the recorded slicing.
    ///
    /// The divergence is named by the first boundary that differs, so a replay
    /// failure points at the exact slice where the paths parted.
    pub fn compare_replay(&self, recorded: &SliceLedger) -> Result<(), SliceReplayMismatch> {
        if self.quantum_fuel != recorded.quantum_fuel || self.total_budget_fuel != recorded.total_budget_fuel {
            return Err(SliceReplayMismatch {
                boundary_index: 0,
                recorded_fuel: recorded.quantum_fuel,
                recomputed_fuel: self.quantum_fuel,
            });
        }
        for boundary in &self.boundaries {
            let Some(recorded_boundary) = recorded.boundaries.get(boundary.index) else {
                return Err(SliceReplayMismatch {
                    boundary_index: boundary.index,
                    recorded_fuel: 0,
                    recomputed_fuel: boundary.fuel_consumed,
                });
            };
            if recorded_boundary.fuel_consumed != boundary.fuel_consumed {
                return Err(SliceReplayMismatch {
                    boundary_index: boundary.index,
                    recorded_fuel: recorded_boundary.fuel_consumed,
                    recomputed_fuel: boundary.fuel_consumed,
                });
            }
        }
        if recorded.boundaries.len() > self.boundaries.len() {
            let index = self.next_boundary_index;
            return Err(SliceReplayMismatch {
                boundary_index: index,
                recorded_fuel: recorded
                    .boundaries
                    .get(self.boundaries.len())
                    .map_or(0, |boundary| boundary.fuel_consumed),
                recomputed_fuel: 0,
            });
        }
        Ok(())
    }
}

/// The pending turn of a sliced callback.
///
/// Nothing is published before the callback completes, and completion publishes
/// exactly once. A resumed continuation never republishes a partial turn and
/// never reruns from the beginning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingTurn<T> {
    pending: T,
    published: bool,
}

// r[impl molten.wasm_component.yield_continuation]
impl<T> PendingTurn<T> {
    pub fn new(pending: T) -> Self {
        Self {
            pending,
            published: false,
        }
    }

    pub fn is_published(&self) -> bool {
        self.published
    }

    /// Publish the completed turn exactly once.
    pub fn publish(mut self) -> Result<T, SliceIssue> {
        if self.published {
            return Err(SliceIssue::BoundaryOverflow);
        }
        self.published = true;
        Ok(self.pending)
    }
}

/// The identity of a yielded continuation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinuationKey {
    pub service_id: String,
    pub generation: u64,
    pub runnable_id: String,
    pub occurrence_sequence: u64,
}

// r[impl molten.wasm_component.yield_continuation]
impl ContinuationKey {
    /// The scheduler's runnable identity for this continuation.
    pub fn runnable_key(&self) -> molten_core::fabric_time::RunnableKey {
        molten_core::fabric_time::RunnableKey {
            service_id: self.service_id.clone(),
            generation: self.generation,
            runnable_id: self.runnable_id.clone(),
            occurrence_sequence: self.occurrence_sequence,
        }
    }

    /// The command that re-enqueues the continuation with the admitted
    /// scheduler after a yield.
    pub fn requeue_command(&self, priority: i32) -> molten_core::fabric_time::SchedulerCommand {
        molten_core::fabric_time::SchedulerCommand::Wake {
            key: self.runnable_key(),
            priority,
        }
    }
}

// r[impl molten.wasm_component.execution_quantum]
/// Admit the requested execution profile against the admitted profile.
///
/// A time-sliced request needs an admitted profile that binds the slicing
/// profile, and a synchronous request must not land on a time-sliced profile.
pub fn admit_requested_profile(
    profile: &super::model::RuntimeProfile,
    requested: super::model::RequestedExecutionProfile,
) -> super::model::Result<()> {
    let is_slicing_admitted = profile.slicing.is_some();
    if requested.requests_time_slicing() && !is_slicing_admitted {
        return Err(super::model::Denial::classified(
            super::model::DenialClass::ProfileDenial,
            format!(
                "requested profile {} is not admitted by profile {}; the time-sliced request needs an admitted slicing profile",
                requested.as_str(),
                profile.profile_id
            ),
        ));
    }
    if !requested.requests_time_slicing() && is_slicing_admitted {
        return Err(super::model::Denial::classified(
            super::model::DenialClass::ProfileDenial,
            format!(
                "requested profile {} is not admitted by the time-sliced profile {}",
                requested.as_str(),
                profile.profile_id
            ),
        ));
    }
    Ok(())
}
