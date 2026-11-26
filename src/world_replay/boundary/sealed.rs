//! The sealed observation adapter replay hands to transition execution.
//!
//! During replay a transition asks this adapter for each recorded observation
//! it consumes, in logical order. The adapter answers only from the admitted
//! ledger: it never calls an external service, and it denies every attempt to
//! execute an original effect. Any demand it cannot match exactly fails the
//! replay before the transition is accepted.

use std::collections::BTreeMap;

use molten_core::world_replay::*;

use crate::error::Failure;
use crate::error::Result;

/// A transition's demand for one recorded observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayObservationDemand {
    pub step_position: u64,
    pub logical_position: u64,
    pub source_id: String,
    pub operation_id: String,
    pub request_ref: String,
}

/// The admitted observations of one replay, consumed in logical order.
#[derive(Debug)]
pub struct SealedReplayObservations {
    inventory: ReplaySourceInventory,
    pending: BTreeMap<(u64, u64), RecordedReplayObservation>,
    effect_attempts: u32,
    demand_violations: u32,
}

impl SealedReplayObservations {
    /// Seal the observations of an admitted boundary.
    pub fn new(boundary: &WorldReplayObservationBoundary, trace: &WorldTransitionTrace) -> Result<Self> {
        admit_replay_observations(boundary, trace).map_err(|blockers| {
            Failure::invalid_harness(format!("world replay observation boundary is blocked: {blockers:?}"))
        })?;
        let pending = boundary
            .ledger
            .observations
            .iter()
            .map(|observation| (observation.slot.order_key(), observation.clone()))
            .collect();
        Ok(Self {
            inventory: boundary.inventory.clone(),
            pending,
            effect_attempts: 0,
            demand_violations: 0,
        })
    }

    /// Return the sealed observation for the next slot in logical order.
    ///
    /// The demand is validated against the pending slot before the slot is
    /// consumed. A denied demand leaves the slot pending and is recorded as a
    /// violation, so a transition that ignores the returned error still fails
    /// `finish_step`.
    // r[impl molten.world_replay_boundary.effect_sealing]
    // r[impl molten.world_replay_boundary.ordering]
    // r[impl molten.world_commit.replay_sealed_observation_and_full_comparison]
    pub fn observe(&mut self, demand: &ReplayObservationDemand) -> Result<RecordedReplayObservation> {
        let key = match self.admitted_demand_key(demand) {
            Ok(key) => key,
            Err(reason) => {
                self.demand_violations = self.demand_violations.saturating_add(1);
                return Err(denial(reason, demand));
            }
        };
        self.pending.remove(&key).ok_or_else(|| denial("no recorded observation at this position", demand))
    }

    /// The pending slot key a demand binds exactly, or the denial reason.
    fn admitted_demand_key(&self, demand: &ReplayObservationDemand) -> std::result::Result<(u64, u64), &'static str> {
        match self.inventory.source(&demand.source_id).map(|source| source.handling) {
            None => return Err("unknown source"),
            Some(ReplaySourceHandling::RecordedObservation) => {}
            Some(_) => return Err("source is not replayed from recorded observations"),
        }
        let key = (demand.step_position, demand.logical_position);
        let Some((next_key, observation)) = self.pending.first_key_value() else {
            return Err("no recorded observation remains");
        };
        if *next_key != key {
            return Err("demand is out of logical order");
        }
        let slot = &observation.slot;
        let is_bound = slot.source_id == demand.source_id
            && slot.operation_id == demand.operation_id
            && slot.request_ref == demand.request_ref;
        if !is_bound {
            return Err("recorded observation belongs to another request");
        }
        Ok(key)
    }

    /// Deny an attempt to execute an original effect during replay.
    // r[impl molten.world_replay_boundary.effect_sealing]
    pub fn attempt_effect(&mut self, source_id: &str) -> Result<()> {
        self.effect_attempts = self.effect_attempts.saturating_add(1);
        Err(Failure::invalid_harness(format!(
            "world replay denied the original effect of {source_id}; replay returns sealed observations only"
        )))
    }

    /// Fail unless the step consumed every observation recorded for it and
    /// made no denied demand.
    pub fn finish_step(&self, step_position: u64) -> Result<()> {
        if self.effect_attempts > 0 {
            return Err(Failure::invalid_harness("world replay attempted an original effect"));
        }
        if self.demand_violations > 0 {
            return Err(Failure::invalid_harness(format!(
                "world replay step {step_position} made {} denied observation demand(s)",
                self.demand_violations
            )));
        }
        let is_unconsumed = self.pending.keys().any(|(step, _)| *step <= step_position);
        if is_unconsumed {
            return Err(Failure::invalid_harness(format!(
                "world replay step {step_position} left a recorded observation unconsumed"
            )));
        }
        Ok(())
    }

    pub const fn effect_attempts(&self) -> u32 {
        self.effect_attempts
    }

    pub const fn demand_violations(&self) -> u32 {
        self.demand_violations
    }
}

fn denial(reason: &str, demand: &ReplayObservationDemand) -> Failure {
    Failure::invalid_harness(format!(
        "world replay denied observation {} at step {} position {}: {reason}",
        demand.source_id, demand.step_position, demand.logical_position
    ))
}
