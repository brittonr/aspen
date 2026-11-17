// SPDX-License-Identifier: AGPL-3.0-or-later
use crate::*;

pub(super) struct State<'a> {
    pub(super) limits: &'a Limits,
    pub(super) fuel: u64,
    pub(super) hook: &'a mut dyn FnMut(&Observation) -> Control,
    pub(super) counters: Counters,
    pub(super) trace: Vec<Observation>,
    pub(super) last_phase: Option<Phase>,
    pub(super) last_work: u64,
    pub(super) stopped: Option<Error>,
}
impl State<'_> {
    pub(super) fn charge(&mut self, phase: Phase, units: u64, interactions: u64) -> Result<(), Error> {
        if let Some(error) = &self.stopped {
            return Err(error.clone());
        }
        let result = self.charge_inner(phase, units, interactions);
        if let Err(error) = &result {
            self.stopped = Some(error.clone());
        }
        result
    }
    fn charge_inner(&mut self, phase: Phase, units: u64, interactions: u64) -> Result<(), Error> {
        if units > self.limits.max_atomic_work {
            return Err(Error::Exhausted(match phase {
                Phase::Decode => Resource::Decode,
                Phase::Validate => Resource::Validation,
                Phase::Reduce => Resource::Reduction,
                Phase::Reference => Resource::Reference,
                Phase::Readback => Resource::Readback,
                _ => Resource::Setup,
            }));
        }
        self.counters.total_work =
            self.counters.total_work.checked_add(units).ok_or(Error::Exhausted(Resource::TotalWork))?;
        if self.counters.total_work > self.limits.max_total_work.min(self.fuel) {
            return Err(Error::Exhausted(Resource::TotalWork));
        }
        let (counter, cap, resource) = match phase {
            Phase::ParseCheck | Phase::Extract | Phase::Lower | Phase::Setup => {
                (&mut self.counters.setup_work, self.limits.max_setup_work, Resource::Setup)
            }
            Phase::Validate => {
                (&mut self.counters.validation_work, self.limits.max_validation_work, Resource::Validation)
            }
            Phase::Readback => (&mut self.counters.readback_work, self.limits.max_readback_work, Resource::Readback),
            Phase::Reduce => (
                &mut self.counters.queue_pops,
                self.limits.interaction_fuel.saturating_mul(8).saturating_add(self.limits.max_agent_slots),
                Resource::Reduction,
            ),
            Phase::Reference => {
                (&mut self.counters.reference_nodes, self.limits.reference_fuel.min(self.fuel), Resource::Reference)
            }
            Phase::Decode | Phase::Complete => (&mut self.counters.total_work, u64::MAX, Resource::TotalWork),
        };
        // Decode/Complete have no second counter; work was charged above.
        if !matches!(phase, Phase::Decode | Phase::Complete) {
            *counter = counter.checked_add(units).ok_or(Error::Exhausted(resource))?;
            if *counter > cap {
                return Err(Error::Exhausted(resource));
            }
        }
        self.counters.interactions = interactions;
        if self.last_phase != Some(phase)
            || self.counters.total_work.saturating_sub(self.last_work) >= self.limits.quantum_work
            || phase == Phase::Complete
        {
            if u32::try_from(self.trace.len()).map_err(|_| Error::Exhausted(Resource::Observations))?
                >= self.limits.max_observations
            {
                return Err(Error::Exhausted(Resource::Observations));
            }
            let observation = Observation {
                phase,
                work: self.counters.total_work,
                interactions,
            };
            self.trace.push(observation.clone());
            self.last_work = self.counters.total_work;
            self.last_phase = Some(phase);
            if (self.hook)(&observation) == Control::Cancel {
                return Err(Error::Cancelled);
            }
        }
        Ok(())
    }
}
