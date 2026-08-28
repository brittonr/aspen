mod comparison;
mod receipt;

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use comparison::compare_case;
pub use comparison::compare_world_fault_observation;
use comparison::observations_by_case;
pub use receipt::validate_world_fault_receipt;
use transactional_reconciliation_core::PersistenceDecision;
use transactional_reconciliation_core::PersistenceState;
use transactional_reconciliation_core::QuarantineReason;
use transactional_reconciliation_core::QuarantineStatus;

use super::*;

pub(super) const PHYSICAL_FAILURE_PROFILE_CASE: &str = "physical-power-loss-profile";

// r[impl molten.world_faults.recovery]
pub fn classify_transactional_recovery(decision: PersistenceDecision) -> RecoveryClass {
    match (decision.state(), decision.quarantine()) {
        (PersistenceState::Published, QuarantineStatus::Clear) => RecoveryClass::AlreadyComplete,
        (PersistenceState::NotPublished, QuarantineStatus::Clear) => RecoveryClass::SafeToRetry,
        (PersistenceState::Conflicting, _) => RecoveryClass::Conflict,
        (PersistenceState::PublicationUnknown, QuarantineStatus::Quarantined(QuarantineReason::Corrupt)) => {
            RecoveryClass::Corrupt
        }
        (PersistenceState::PublicationUnknown, QuarantineStatus::Quarantined(QuarantineReason::Inconsistent)) => {
            RecoveryClass::Conflict
        }
        (
            PersistenceState::PublicationUnknown,
            QuarantineStatus::Quarantined(QuarantineReason::CommitOutcomeUnknown),
        ) => RecoveryClass::Uncertain,
        (
            PersistenceState::PublicationUnknown,
            QuarantineStatus::Quarantined(QuarantineReason::RepairReported | QuarantineReason::Missing),
        ) => RecoveryClass::ManualReview,
        (PersistenceState::PublicationUnknown, QuarantineStatus::Clear)
        | (PersistenceState::NotPublished | PersistenceState::Published, QuarantineStatus::Quarantined(_)) => {
            RecoveryClass::ManualReview
        }
    }
}

// r[impl molten.world_faults.receipt]
#[allow(
    tigerstyle::function_length,
    tigerstyle::unbounded_collection_growth,
    reason = "validated profile limits bound every map, set, result, schedule, and unsupported row in this assembly"
)]
pub fn build_world_fault_conformance_receipt(
    inventory: &WorldMutationInventory,
    profile: &WorldFaultProfile,
    observations: &[WorldOperationObservation],
    schedule_results: Vec<ConcurrentScheduleResult>,
) -> Result<WorldFaultConformanceReceipt, Vec<WorldFaultIssue>> {
    let mut structural = validate_world_mutation_inventory(inventory, &registered_world_mutation_names());
    structural.extend(validate_world_fault_profile(profile));
    if observations.len() > profile.limits.max_observations {
        structural.push(WorldFaultIssue::BoundExceeded {
            field: "observations",
            actual: observations.len(),
            maximum: profile.limits.max_observations,
        });
    }
    let mut observed_case_ids = BTreeSet::new();
    for observation in observations {
        if !observed_case_ids.insert(observation.case_id.as_str()) {
            structural.push(WorldFaultIssue::DuplicateObservation(observation.case_id.clone()));
        }
    }
    let inventory_ref = identify_world_mutation_inventory(inventory)?;
    if inventory_ref != profile.inventory_ref {
        structural.push(WorldFaultIssue::MalformedReference {
            field: "profile-inventory-ref",
            value: profile.inventory_ref.clone(),
        });
    }
    if !structural.is_empty() {
        structural.sort();
        structural.dedup();
        return Err(structural);
    }

    let profile_ref = identify_world_fault_profile(profile)?;
    let observations = observations_by_case(observations);
    let rows = inventory.rows.iter().map(|row| (row.mutation, row)).collect::<BTreeMap<_, _>>();
    let mut results = Vec::with_capacity(profile.cases.len());
    let mut unsupported_rows = Vec::with_capacity(profile.limits.max_unsupported_rows);
    for case in &profile.cases {
        let Some(row) = rows.get(&case.mutation) else {
            return Err(vec![WorldFaultIssue::MissingMutation(case.mutation)]);
        };
        if row.support == MutationSupport::UnsupportedIndependentWitness {
            unsupported_rows.push(UnsupportedConformanceRow {
                mutation: case.mutation,
                case_id: case.case_id.clone(),
                reason: UnsupportedReason::IndependentWitnessOwnerUnavailable,
            });
            continue;
        }
        results.push(compare_case(case, observations.get(&case.case_id).copied()));
    }
    unsupported_rows.push(UnsupportedConformanceRow {
        mutation: WorldMutationKind::Capture,
        case_id: PHYSICAL_FAILURE_PROFILE_CASE.to_string(),
        reason: UnsupportedReason::PhysicalFailureProfileNotExercised,
    });

    let expected_schedules =
        profile.schedules.iter().map(|schedule| schedule.schedule_id.as_str()).collect::<BTreeSet<_>>();
    let observed_schedules = schedule_results.iter().map(|result| result.schedule_id.clone()).collect::<BTreeSet<_>>();
    let mut normalized_schedule_results = schedule_results;
    normalized_schedule_results.reserve(profile.schedules.len());
    for missing in expected_schedules.iter().filter(|schedule_id| !observed_schedules.contains(**schedule_id)) {
        let Some(schedule) = profile.schedules.iter().find(|schedule| schedule.schedule_id == *missing) else {
            continue;
        };
        normalized_schedule_results.push(missing_schedule_result(missing, schedule.mutation));
    }
    normalized_schedule_results.sort_by(|left, right| left.schedule_id.cmp(&right.schedule_id));

    let is_failed = results.iter().any(|result| result.disposition == ConformanceDisposition::Failed)
        || normalized_schedule_results
            .iter()
            .any(|result| result.disposition == ConformanceDisposition::Failed);
    let schedule_refs = profile
        .schedules
        .iter()
        .map(identify_concurrent_schedule)
        .collect::<Result<Vec<_>, Vec<WorldFaultIssue>>>()?;
    let receipt = WorldFaultConformanceReceipt {
        schema: WORLD_FAULT_RECEIPT_SCHEMA,
        source_revision: profile.source_revision.clone(),
        inventory_ref,
        profile_ref,
        adapter_refs: profile.adapters.iter().map(|adapter| adapter.implementation_ref.clone()).collect(),
        schedule_refs,
        limits: profile.limits,
        results,
        schedules: normalized_schedule_results,
        unsupported_rows,
        decision: if is_failed {
            ConformanceDisposition::Failed
        } else {
            ConformanceDisposition::Passed
        },
        mutation_authorized_by_evidence: false,
        cleanup_authorized_by_evidence: false,
        non_claims: REQUIRED_WORLD_FAULT_NON_CLAIMS.to_vec(),
    };
    let issues = validate_world_fault_receipt(&receipt, profile);
    if issues.is_empty() { Ok(receipt) } else { Err(issues) }
}

fn missing_schedule_result(schedule_id: &str, mutation: WorldMutationKind) -> ConcurrentScheduleResult {
    let is_witness = mutation == WorldMutationKind::Witness;
    ConcurrentScheduleResult {
        schedule_id: schedule_id.to_string(),
        observations: Vec::new(),
        scheduler_choices: Vec::new(),
        disposition: if is_witness {
            ConformanceDisposition::Unsupported
        } else {
            ConformanceDisposition::Failed
        },
        diagnostics: if is_witness {
            Vec::new()
        } else {
            vec![WorldFaultIssue::ConcurrentObservationMissing(schedule_id.to_string())]
        },
    }
}
