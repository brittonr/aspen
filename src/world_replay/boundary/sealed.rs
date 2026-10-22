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
        })
    }

    /// Return the sealed observation for the next slot in logical order.
    // r[impl molten.world_replay_boundary.effect_sealing]
    // r[impl molten.world_replay_boundary.ordering]
    pub fn observe(&mut self, demand: &ReplayObservationDemand) -> Result<RecordedReplayObservation> {
        let handling = self.inventory.source(&demand.source_id).map(|source| source.handling);
        match handling {
            None => return Err(denial("unknown source", demand)),
            Some(ReplaySourceHandling::RecordedObservation) => {}
            Some(_) => return Err(denial("source is not replayed from recorded observations", demand)),
        }
        let key = (demand.step_position, demand.logical_position);
        let Some((next_key, _)) = self.pending.first_key_value() else {
            return Err(denial("no recorded observation remains", demand));
        };
        if *next_key != key {
            return Err(denial("demand is out of logical order", demand));
        }
        let Some(observation) = self.pending.remove(&key) else {
            return Err(denial("no recorded observation at this position", demand));
        };
        let slot = &observation.slot;
        let is_bound = slot.source_id == demand.source_id
            && slot.operation_id == demand.operation_id
            && slot.request_ref == demand.request_ref;
        if !is_bound {
            return Err(denial("recorded observation belongs to another request", demand));
        }
        Ok(observation)
    }

    /// Deny an attempt to execute an original effect during replay.
    // r[impl molten.world_replay_boundary.effect_sealing]
    pub fn attempt_effect(&mut self, source_id: &str) -> Result<()> {
        self.effect_attempts = self.effect_attempts.saturating_add(1);
        Err(Failure::invalid_harness(format!(
            "world replay denied the original effect of {source_id}; replay returns sealed observations only"
        )))
    }

    /// Fail unless the step consumed every observation recorded for it.
    pub fn finish_step(&self, step_position: u64) -> Result<()> {
        if self.effect_attempts > 0 {
            return Err(Failure::invalid_harness("world replay attempted an original effect"));
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
}

fn denial(reason: &str, demand: &ReplayObservationDemand) -> Failure {
    Failure::invalid_harness(format!(
        "world replay denied observation {} at step {} position {}: {reason}",
        demand.source_id, demand.step_position, demand.logical_position
    ))
}
