//! Fallible scheduler-capacity activation and runtime ownership.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReservationKind {
    Runnables,
    Queue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivationError {
    Plan(molten_core::fabric_time::capacity::PlanIssue),
    CountUnrepresentable(ReservationKind),
    ReservationFailed(ReservationKind),
}

pub struct Runtime {
    plan: molten_core::fabric_time::capacity::Plan,
    pub(super) state: molten_core::fabric_time::capacity::UseState,
    pub(super) scheduler: molten_core::fabric_time::SchedulerState,
    pub(super) ready: Vec<usize>,
}

impl Runtime {
    // r[impl molten.fabric_time.scheduler_capacity.activation]
    pub fn activate(
        profile: &molten_core::fabric_time::AdmittedTimeProfile,
        generation: u64,
    ) -> Result<Self, ActivationError> {
        activate_with(profile, generation, |_kind, _slots| Ok(()))
    }

    pub fn plan(&self) -> &molten_core::fabric_time::capacity::Plan {
        &self.plan
    }

    pub fn state(&self) -> &molten_core::fabric_time::capacity::UseState {
        &self.state
    }

    #[cfg(test)]
    fn runnable_capacity(&self) -> usize {
        self.scheduler.runnables.capacity()
    }

    #[cfg(test)]
    fn queue_capacity(&self) -> usize {
        self.ready.capacity()
    }

    // r[impl molten.fabric_time.scheduler_capacity.steady_state]
    // r[impl molten.fabric_time.scheduler_capacity.boundary]
    pub fn apply(
        &mut self,
        profile_ref: &str,
        generation: u64,
        request: molten_core::fabric_time::capacity::UseRequest,
    ) -> molten_core::fabric_time::capacity::UseDecisionKind {
        let decision =
            molten_core::fabric_time::capacity::apply_use(&self.plan, &self.state, profile_ref, generation, request);
        self.state = decision.next;
        decision.kind
    }

    // r[impl molten.fabric_time.scheduler_capacity.observation]
    pub fn observation(&self) -> molten_core::fabric_time::capacity::Observation {
        molten_core::fabric_time::capacity::observe(&self.state)
    }

    pub fn release(&mut self) {
        self.scheduler.runnables = Vec::new();
        self.ready = Vec::new();
        self.state = molten_core::fabric_time::capacity::release(&self.state);
    }
}

fn activate_with<Reserve>(
    profile: &molten_core::fabric_time::AdmittedTimeProfile,
    generation: u64,
    mut reserve: Reserve,
) -> Result<Runtime, ActivationError>
where
    Reserve: FnMut(ReservationKind, usize) -> Result<(), ()>,
{
    let plan = molten_core::fabric_time::capacity::derive(profile, generation).map_err(ActivationError::Plan)?;
    let runnable_slots = usize::try_from(plan.runnable_slots)
        .map_err(|_| ActivationError::CountUnrepresentable(ReservationKind::Runnables))?;
    let queue_slots =
        usize::try_from(plan.queue_slots).map_err(|_| ActivationError::CountUnrepresentable(ReservationKind::Queue))?;

    reserve(ReservationKind::Runnables, runnable_slots)
        .map_err(|()| ActivationError::ReservationFailed(ReservationKind::Runnables))?;
    let mut runnable_reservation = Vec::new();
    runnable_reservation
        .try_reserve_exact(runnable_slots)
        .map_err(|_| ActivationError::ReservationFailed(ReservationKind::Runnables))?;

    reserve(ReservationKind::Queue, queue_slots)
        .map_err(|()| ActivationError::ReservationFailed(ReservationKind::Queue))?;
    let mut queue_reservation = Vec::new();
    queue_reservation
        .try_reserve_exact(queue_slots)
        .map_err(|_| ActivationError::ReservationFailed(ReservationKind::Queue))?;

    let state = molten_core::fabric_time::capacity::UseState::activated(&plan);
    let mut scheduler = molten_core::fabric_time::new_scheduler_state(profile, generation);
    scheduler.runnables = runnable_reservation;
    Ok(Runtime {
        plan,
        state,
        scheduler,
        ready: queue_reservation,
    })
}

#[cfg(test)]
mod tests;
