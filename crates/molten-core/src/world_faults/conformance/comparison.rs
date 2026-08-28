use std::collections::BTreeMap;

use super::super::*;

#[allow(
    tigerstyle::unbounded_collection_growth,
    reason = "the caller validates observation count against max_observations before building this bounded map"
)]
pub(super) fn observations_by_case(
    observations: &[WorldOperationObservation],
) -> BTreeMap<String, &WorldOperationObservation> {
    let mut by_case = BTreeMap::new();
    for observation in observations {
        by_case.entry(observation.case_id.clone()).or_insert(observation);
    }
    by_case
}

// r[impl molten.world_faults.recovery]
pub fn compare_world_fault_observation(
    case: &WorldFaultCase,
    observation: &WorldOperationObservation,
) -> WorldFaultConformanceResult {
    compare_case(case, Some(observation))
}

pub(super) fn compare_case(
    case: &WorldFaultCase,
    observation: Option<&WorldOperationObservation>,
) -> WorldFaultConformanceResult {
    let is_observation_missing = observation.is_none();
    let observation = observation.cloned().unwrap_or_else(|| missing_observation(case));
    let mut diagnostics = Vec::with_capacity(REQUIRED_FAULT_PHASE_COUNT);
    if is_observation_missing {
        diagnostics.push(WorldFaultIssue::ObservationMissing(case.case_id.clone()));
    }
    if observation.case_id != case.case_id || observation.operation_id != case.operation_id {
        diagnostics.push(WorldFaultIssue::ObservationOperationMismatch(case.case_id.clone()));
    }
    if observation.phase != case.phase {
        diagnostics.push(WorldFaultIssue::ObservationPhaseMismatch(case.case_id.clone()));
    }
    if observation.owner_decision != case.expected_decision {
        diagnostics.push(WorldFaultIssue::OwnerDecisionMismatch {
            case_id: case.case_id.clone(),
            expected: case.expected_decision,
            actual: observation.owner_decision,
        });
    }
    validate_conservative_observation(case, &observation, &mut diagnostics);
    diagnostics.sort();
    diagnostics.dedup();
    WorldFaultConformanceResult {
        case_id: case.case_id.clone(),
        mutation: case.mutation,
        phase: case.phase,
        expected_decision: case.expected_decision,
        observed_decision: observation.owner_decision,
        disposition: if diagnostics.is_empty() {
            ConformanceDisposition::Passed
        } else {
            ConformanceDisposition::Failed
        },
        observation,
        diagnostics,
    }
}

// r[impl molten.world_faults.verification]
fn validate_conservative_observation(
    case: &WorldFaultCase,
    observation: &WorldOperationObservation,
    diagnostics: &mut Vec<WorldFaultIssue>,
) {
    let is_complete = observation.owner_decision == RecoveryClass::AlreadyComplete;
    if is_complete
        && (observation.read_back.status != DurableReadBackStatus::Applied
            || observation.read_back.state_ref.is_none()
            || observation.read_back.record_ref.is_none())
    {
        diagnostics.push(WorldFaultIssue::SuccessWithoutDurableReadBack(case.case_id.clone()));
    }
    if observation.submission == SubmissionObservation::PossiblySubmitted
        && observation.owner_decision == RecoveryClass::SafeToRetry
    {
        diagnostics.push(WorldFaultIssue::UnsafeRetryAfterPossibleSubmit(case.case_id.clone()));
    }
    if observation.read_back.status == DurableReadBackStatus::Missing && is_complete {
        diagnostics.push(WorldFaultIssue::MissingStateBecameSuccess(case.case_id.clone()));
    }
    if observation.read_back.status == DurableReadBackStatus::Corrupt
        && !matches!(observation.owner_decision, RecoveryClass::Corrupt | RecoveryClass::ManualReview)
    {
        diagnostics.push(WorldFaultIssue::CorruptStateMisclassified(case.case_id.clone()));
    }
    if observation.read_back.status == DurableReadBackStatus::Contradictory
        && !matches!(observation.owner_decision, RecoveryClass::Conflict | RecoveryClass::ManualReview)
    {
        diagnostics.push(WorldFaultIssue::ContradictoryStateMisclassified(case.case_id.clone()));
    }
    if observation.whole_store_rollback
        && !observation.read_back.independent_witness
        && !matches!(observation.owner_decision, RecoveryClass::Uncertain | RecoveryClass::ManualReview)
    {
        diagnostics.push(WorldFaultIssue::LocalRollbackDetectionOverclaim(case.case_id.clone()));
    }
    let is_cleanup_facts_complete = is_complete
        && observation.read_back.status == DurableReadBackStatus::Applied
        && observation.read_back.independent_witness;
    if observation.cleanup_authorized && !is_cleanup_facts_complete {
        diagnostics.push(WorldFaultIssue::UnsafeCleanupAuthority(case.case_id.clone()));
    }
}

fn missing_observation(case: &WorldFaultCase) -> WorldOperationObservation {
    WorldOperationObservation {
        case_id: case.case_id.clone(),
        operation_id: case.operation_id.clone(),
        phase: case.phase,
        submission: SubmissionObservation::NotSubmitted,
        response: ResponseObservation::NotExpected,
        read_back: DurableReadBack {
            status: DurableReadBackStatus::Missing,
            state_ref: None,
            record_ref: None,
            observed_generation: None,
            independent_witness: false,
        },
        owner_decision: RecoveryClass::ManualReview,
        whole_store_rollback: false,
        cleanup_authorized: false,
    }
}
