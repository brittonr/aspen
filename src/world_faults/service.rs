use std::collections::BTreeMap;
use std::collections::BTreeSet;

use molten_core::world_faults::*;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;

#[derive(Debug, Clone)]
pub struct WorldFaultHarnessOutcome {
    pub receipt: WorldFaultConformanceReceipt,
    pub record: CanonicalWorldFaultReceipt,
    pub persisted_receipt_ref: String,
}

// r[impl molten.world_faults.interruption]
// r[impl molten.world_faults.recovery]
// r[impl molten.world_faults.receipt]
pub fn run_world_fault_conformance(
    inventory: &WorldMutationInventory,
    profile: &WorldFaultProfile,
    ports: WorldFaultHarnessPorts<'_>,
) -> Result<WorldFaultHarnessOutcome> {
    core_issues(
        validate_world_mutation_inventory(inventory, &registered_world_mutation_names()),
        "world mutation inventory",
    )?;
    core_issues(validate_world_fault_profile(profile), "world fault profile")?;

    let support = inventory.rows.iter().map(|row| (row.mutation, row.support)).collect::<BTreeMap<_, _>>();
    let mut observations = Vec::with_capacity(profile.limits.max_observations);
    let mut restart_count = 0_u32;
    for case in &profile.cases {
        if support.get(&case.mutation) == Some(&MutationSupport::UnsupportedIndependentWitness) {
            continue;
        }
        let interruption = ports.fault_control.interrupt(case)?;
        if matches!(case.phase, FaultPhase::ProcessRestart | FaultPhase::RecoveryReadBack) {
            restart_count = restart_count.saturating_add(1);
            if restart_count > profile.limits.max_restarts {
                return Err(MoltenError::invalid_harness("world fault restart bound exceeded"));
            }
            ports.restart.restart(case)?;
        }
        let read_back = ports.durable_observation.read_back(case)?;
        let owner_decision = ports.owner_decision.decide(case, interruption, &read_back)?;
        observations.push(WorldOperationObservation {
            case_id: case.case_id.clone(),
            operation_id: case.operation_id.clone(),
            phase: case.phase,
            submission: interruption.submission,
            response: interruption.response,
            read_back,
            owner_decision,
            whole_store_rollback: interruption.whole_store_rollback,
            cleanup_authorized: interruption.cleanup_authorized,
        });
    }

    let mut schedules = Vec::with_capacity(profile.schedules.len());
    for schedule in &profile.schedules {
        if schedule.mutation == WorldMutationKind::Witness {
            schedules.push(evaluate_concurrent_schedule(schedule, Vec::new(), Vec::new()));
            continue;
        }
        let positions = schedule.steps.iter().map(|step| step.position).collect::<BTreeSet<_>>();
        let eligible = positions
            .into_iter()
            .map(|position| eligible_world_fault_choices(schedule, position))
            .collect::<Vec<_>>();
        let execution = ports.concurrent_schedule.execute_schedule(schedule, &eligible)?;
        schedules.push(evaluate_concurrent_schedule(schedule, execution.observations, execution.scheduler_choices));
    }

    let receipt = build_world_fault_conformance_receipt(inventory, profile, &observations, schedules)
        .map_err(|issues| core_error("world fault conformance", issues))?;
    let record = canonical_world_fault_receipt(&receipt, profile)?;
    let persisted_receipt_ref = ports.receipts.publish_receipt(&record)?;
    if persisted_receipt_ref != record.record_ref {
        return Err(MoltenError::invalid_harness("world fault receipt port returned a crossed record identity"));
    }
    Ok(WorldFaultHarnessOutcome {
        receipt,
        record,
        persisted_receipt_ref,
    })
}

fn core_issues(issues: Vec<WorldFaultIssue>, context: &str) -> Result<()> {
    if issues.is_empty() {
        Ok(())
    } else {
        Err(core_error(context, issues))
    }
}

fn core_error(context: &str, issues: Vec<WorldFaultIssue>) -> MoltenError {
    MoltenError::invalid_harness(format!("{context} denied: {issues:?}"))
}
