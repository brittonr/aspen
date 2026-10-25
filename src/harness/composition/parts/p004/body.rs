
// The CPU-saturation scenario under the time-sliced execution profile.
//
// A saturating callback of the stateful service and one runnable of the
// independent sibling are both ready under the admitted fabric-time
// scheduler. Each slice of the callback consumes at most the admitted
// quantum; on a yield the continuation is re-enqueued with the scheduler's
// own command, so the sibling is selected between slices. The property holds
// when the sibling completes before the callback and the callback still ends
// within its budget.

/// What one slice of saturating work did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaturationSlice {
    /// The callback consumed one quantum and yielded.
    Yielded,
    /// The callback ended: completed, or stopped at its budget.
    Ended { is_within_budget: bool },
}

/// A saturating callback driven one slice at a time.
pub trait SaturatingWork {
    fn run_slice(&mut self) -> SaturationSlice;
}

/// Saturating work modeled with the admitted profile's slice ledger: it needs
/// a fixed amount of fuel and spends one quantum per slice.
pub struct ModeledSaturation {
    ledger: crate::wasm_component::SliceLedger,
    required_fuel: u64,
}

impl ModeledSaturation {
    /// Work that needs `quanta` full quanta of the admitted time-sliced profile.
    pub fn for_quanta(quanta: u64) -> Result<Self, crate::error::Failure> {
        let profile = crate::wasm_component::time_sliced()
            .map_err(|denial| crate::error::Failure::invalid_harness(format!("time-sliced profile: {denial:?}")))?;
        let slicing = profile
            .slicing
            .ok_or_else(|| crate::error::Failure::invalid_harness("the time-sliced profile binds no slicing"))?;
        let ledger = crate::wasm_component::SliceLedger::new(slicing.quantum_fuel, slicing.total_budget_fuel)
            .map_err(|issue| crate::error::Failure::invalid_harness(format!("slice ledger: {issue:?}")))?;
        let required_fuel = slicing
            .quantum_fuel
            .checked_mul(quanta)
            .ok_or_else(|| crate::error::Failure::invalid_harness("saturating work overflows its fuel"))?;
        Ok(Self { ledger, required_fuel })
    }
}

impl SaturatingWork for ModeledSaturation {
    fn run_slice(&mut self) -> SaturationSlice {
        let remaining = self.required_fuel.saturating_sub(self.ledger.consumed_fuel);
        let slice = remaining.min(self.ledger.quantum_fuel);
        let is_recorded = self.ledger.record_boundary(slice).is_ok();
        if !is_recorded {
            return SaturationSlice::Ended { is_within_budget: false };
        }
        if self.ledger.consumed_fuel >= self.required_fuel {
            SaturationSlice::Ended { is_within_budget: true }
        } else {
            SaturationSlice::Yielded
        }
    }
}

const SATURATION_SLICE_LIMIT: usize = 1_024;
const SATURATING_QUANTA: u64 = 8;

fn saturation_time_profile() -> Result<molten_core::fabric_time::AdmittedTimeProfile, crate::error::Failure> {
    let descriptor = molten_core::fabric_time::TimeProfileDescriptor {
        schema: molten_core::fabric_time::FABRIC_TIME_PROFILE_SCHEMA.to_string(),
        profile_id: "molten.composition.saturation".to_string(),
        profile_ref: crate::preserves_rail::content_ref_from_bytes(b"molten.composition.saturation"),
        kind: molten_core::fabric_time::TimeProfileKind::DeterministicSimulation,
        supported_domains: molten_core::fabric_time::REQUIRED_TIME_DOMAINS.to_vec(),
        max_duration_ticks: PROFILE_LIMIT,
        max_uncertainty_ticks: PROFILE_LIMIT,
        max_timers: PROFILE_LIMIT,
        max_runnables: PROFILE_LIMIT,
        max_entropy_request_bytes: PROFILE_LIMIT,
        max_entropy_total_bytes: PROFILE_LIMIT,
        max_scheduler_concurrency: PROFILE_LIMIT,
        max_scheduler_queue_depth: PROFILE_LIMIT,
        fairness_bound_turns: None,
        scheduler_policy: saturation_policy(),
        evidence_mode: molten_core::fabric_time::TimeEvidenceMode::Aggregate,
        non_claims: molten_core::fabric_time::REQUIRED_TIME_NON_CLAIMS.to_vec(),
    };
    crate::fabric_time::admit_profile(&descriptor).map(|admitted| admitted.profile)
}

const fn saturation_policy() -> molten_core::fabric_time::SchedulerPolicy {
    molten_core::fabric_time::SchedulerPolicy {
        ordering: molten_core::fabric_time::SchedulerOrdering::Fifo,
        replay: molten_core::fabric_time::SchedulerReplayPolicy::Deterministic,
        overload: molten_core::fabric_time::SchedulerOverloadPolicy::Reject,
    }
}

fn continuation_for(service_id: &str, occurrence_sequence: u64) -> crate::wasm_component::ContinuationKey {
    crate::wasm_component::ContinuationKey {
        service_id: service_id.to_string(),
        generation: INITIAL_GENERATION,
        runnable_id: format!("{service_id}.callback"),
        occurrence_sequence,
    }
}

/// Run the CPU-saturation scenario with the given saturating work.
///
/// The `SlicingDisabled` variant ignores the callback's yields and runs it to
/// its end before the scheduler selects again, so the sibling cannot progress
/// first and the property fails.
// r[impl molten.testing.service_recovery_composition]
pub fn run_saturation_scenario(
    variant: ScenarioVariant,
    work: &mut dyn SaturatingWork,
) -> Result<ScenarioFacts, crate::error::Failure> {
    let profile = saturation_time_profile()?;
    let heavy = continuation_for(STATEFUL_SERVICE_ID, 1);
    let sibling_key = continuation_for(SIBLING_SERVICE_ID, 2).runnable_key();
    let apply = |state: &molten_core::fabric_time::SchedulerState, command| {
        molten_core::fabric_time::apply_scheduler_command(
            &profile,
            saturation_policy(),
            state,
            INITIAL_GENERATION,
            &command,
        )
        .map(|transition| transition.next)
        .map_err(|error| crate::error::Failure::invalid_harness(format!("scheduler denied: {error:?}")))
    };
    let mut state = molten_core::fabric_time::new_scheduler_state(&profile, INITIAL_GENERATION);
    for key in [heavy.runnable_key(), sibling_key.clone()] {
        state = apply(&state, molten_core::fabric_time::SchedulerCommand::Wake { key, priority: 0 })?;
    }
    let mut sibling_completed_at = None;
    let mut heavy_ended = None;
    for step in 0..SATURATION_SLICE_LIMIT {
        if heavy_ended.is_some() && sibling_completed_at.is_some() {
            break;
        }
        let selection =
            molten_core::fabric_time::choose_runnable(&profile, saturation_policy(), &state, INITIAL_GENERATION, None)
                .map_err(|error| crate::error::Failure::invalid_harness(format!("no runnable: {error:?}")))?;
        state = selection.next;
        let command = if selection.selected == sibling_key {
            sibling_completed_at = Some(step);
            molten_core::fabric_time::SchedulerCommand::Complete { key: selection.selected }
        } else {
            let mut slice = work.run_slice();
            if variant == ScenarioVariant::SlicingDisabled {
                for _ in 0..SATURATION_SLICE_LIMIT {
                    if slice != SaturationSlice::Yielded {
                        break;
                    }
                    slice = work.run_slice();
                }
            }
            match slice {
                SaturationSlice::Yielded => heavy.requeue_command(),
                SaturationSlice::Ended { is_within_budget } => {
                    heavy_ended = Some((step, is_within_budget));
                    molten_core::fabric_time::SchedulerCommand::Complete { key: selection.selected }
                }
            }
        };
        state = apply(&state, command)?;
    }
    let mut facts = ScenarioFacts::empty();
    if let (Some(sibling_step), Some((heavy_step, is_within_budget))) = (sibling_completed_at, heavy_ended) {
        facts.sibling_progressed = sibling_step < heavy_step;
        facts.saturating_work_bounded = is_within_budget;
    }
    Ok(facts)
}

fn cpu_saturation(variant: ScenarioVariant) -> Result<ScenarioFacts, crate::error::Failure> {
    let mut work = ModeledSaturation::for_quanta(SATURATING_QUANTA)?;
    run_saturation_scenario(variant, &mut work)
}
