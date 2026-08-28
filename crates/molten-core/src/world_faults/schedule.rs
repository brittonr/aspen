use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::*;
use crate::fabric_simulation::EligibleChoice;
use crate::fabric_simulation::SchedulerChoiceKind;

// r[impl molten.world_faults.concurrency]
pub fn eligible_world_fault_choices(schedule: &ConcurrentSchedule, position: u32) -> Vec<EligibleChoice> {
    let mut choices = schedule
        .steps
        .iter()
        .filter(|step| step.position == position)
        .map(|step| EligibleChoice {
            kind: scheduler_choice_kind(step.interleaving),
            choice_id: format!(
                "{}:{}:{}:{}",
                schedule.schedule_id,
                step.position,
                step.operation_id,
                step.interleaving.as_str()
            ),
            node_id: step.node_id.clone(),
            generation: step.node_generation,
            ready_at_tick: u64::from(step.position),
        })
        .collect::<Vec<_>>();
    choices.sort();
    choices
}

// r[impl molten.world_faults.concurrency]
#[allow(
    tigerstyle::function_length,
    tigerstyle::unbounded_collection_growth,
    reason = "validated schedules bound maps, sets, diagnostics, and observations by WORLD_FAULT_STEPS_PER_SCHEDULE"
)]
pub fn evaluate_concurrent_schedule(
    schedule: &ConcurrentSchedule,
    observations: Vec<ConcurrentOperationObservation>,
    scheduler_choices: Vec<EligibleChoice>,
) -> ConcurrentScheduleResult {
    if schedule.mutation == WorldMutationKind::Witness {
        return ConcurrentScheduleResult {
            schedule_id: schedule.schedule_id.clone(),
            observations,
            scheduler_choices,
            disposition: ConformanceDisposition::Unsupported,
            diagnostics: Vec::new(),
        };
    }

    let mut diagnostics = Vec::with_capacity(WORLD_FAULT_STEPS_PER_SCHEDULE);
    let expected = schedule.steps.iter().map(|step| step.operation_id.clone()).collect::<BTreeSet<_>>();
    let mut observed = BTreeMap::new();
    for observation in &observations {
        if observed.insert(observation.operation_id.clone(), observation).is_some() {
            diagnostics.push(WorldFaultIssue::ConcurrentObservationUnexpected(observation.operation_id.clone()));
        }
        if !expected.contains(&observation.operation_id) {
            diagnostics.push(WorldFaultIssue::ConcurrentObservationUnexpected(observation.operation_id.clone()));
        }
        if observation.mutation != schedule.mutation
            || schedule.steps.iter().any(|step| {
                step.operation_id == observation.operation_id
                    && (step.expected_generation != observation.expected_generation
                        || step.pre_state_ref != observation.pre_state_ref)
            })
        {
            diagnostics.push(WorldFaultIssue::ConcurrentBindingMismatch(observation.operation_id.clone()));
        }
        if observation.effect_release_count > 1 {
            diagnostics.push(WorldFaultIssue::DuplicateEffectRelease {
                operation_id: observation.operation_id.clone(),
                count: observation.effect_release_count,
            });
        }
    }
    for operation in &expected {
        if !observed.contains_key(operation) {
            diagnostics.push(WorldFaultIssue::ConcurrentObservationMissing(operation.clone()));
        }
    }

    let applied = observations
        .iter()
        .filter(|observation| observation.outcome == ConcurrentOutcome::Applied)
        .collect::<Vec<_>>();
    if applied.len() > 1 {
        diagnostics.push(WorldFaultIssue::MultipleLinearizations {
            schedule_id: schedule.schedule_id.clone(),
            generation: applied
                .first()
                .map_or(WORLD_FAULT_OPERATION_GENERATION, |observation| observation.expected_generation),
        });
    }
    if matches!(schedule.mutation, WorldMutationKind::Promotion | WorldMutationKind::Outbox) {
        let released = observations
            .iter()
            .fold(0_u32, |total, observation| total.saturating_add(observation.effect_release_count));
        if released > 1 {
            diagnostics.push(WorldFaultIssue::DuplicateEffectRelease {
                operation_id: schedule.schedule_id.clone(),
                count: released,
            });
        }
    }
    diagnostics.sort();
    diagnostics.dedup();
    ConcurrentScheduleResult {
        schedule_id: schedule.schedule_id.clone(),
        observations,
        scheduler_choices,
        disposition: if diagnostics.is_empty() {
            ConformanceDisposition::Passed
        } else {
            ConformanceDisposition::Failed
        },
        diagnostics,
    }
}

fn scheduler_choice_kind(interleaving: InterleavingPoint) -> SchedulerChoiceKind {
    match interleaving {
        InterleavingPoint::Prepare | InterleavingPoint::CurrentFactRecheck => SchedulerChoiceKind::Runnable,
        InterleavingPoint::BeforeLinearization | InterleavingPoint::AfterLinearization => {
            SchedulerChoiceKind::StorageCompletion
        }
        InterleavingPoint::DurableReadBack => SchedulerChoiceKind::ProcessLifecycle,
        InterleavingPoint::Finish => SchedulerChoiceKind::Runnable,
    }
}
