use alloc::format;

use super::super::*;
use super::support::*;

const EXPECTED_CLEANUP_OBLIGATIONS: usize = 6;

// r[verify vm_cohort.transactional_creation]
// r[verify vm_cohort.receipts]
#[test]
fn complete_effect_set_activates_atomically_and_projects_stable_bounded_receipt() {
    let plan = plan_cohort(&request()).expect("plan");
    let mut state = new_cohort_state(plan.clone());
    for (index, effect) in plan.effects.iter().enumerate() {
        state = reduce_effect_observation(
            &state,
            &EffectObservation {
                cohort_ref: plan.cohort_ref.clone(),
                clone_ref: effect.clone_ref.clone(),
                operation_ref: effect.operation_ref.clone(),
                kind: effect.kind,
                outcome: OperationOutcome::Success,
                observation_ref: observation_ref(index),
            },
        )
        .expect("effect observation");
    }
    assert_eq!(state.phase, CohortPhase::Active);
    assert_eq!(state.activated_clones.len(), plan.clones.len());
    let first = project_receipt(&state).expect("receipt");
    let second = project_receipt(&state).expect("stable receipt");
    assert_eq!(first, second);
    assert_eq!(first.non_claims.len(), RECEIPT_NON_CLAIMS.len());
}

// r[verify vm_cohort.transactional_creation.partial_failure]
// r[verify vm_cohort.cleanup.uncertain]
#[test]
fn partial_failure_never_activates_and_cleanup_stays_exact_until_observed() {
    let plan = plan_cohort(&request()).expect("plan");
    let first_effect = &plan.effects[0];
    let failed_effect = &plan.effects[1];
    let state = reduce_effect_observation(
        &new_cohort_state(plan.clone()),
        &effect_observation(&plan, first_effect, OperationOutcome::Success, 0),
    )
    .expect("first prepared effect");
    let failed = reduce_effect_observation(
        &state,
        &effect_observation(&plan, failed_effect, OperationOutcome::PermanentFailure, 1),
    )
    .expect("failure reduction");
    assert_eq!(failed.phase, CohortPhase::CleanupRequired);
    assert_eq!(
        failed.cleanup_obligations.len(),
        EXPECTED_CLEANUP_OBLIGATIONS
    );
    assert!(failed.activated_clones.is_empty());

    let obligation = &failed.cleanup_obligations[0];
    let uncertain = reduce_cleanup_observation(
        &failed,
        &CleanupObservation {
            cohort_ref: plan.cohort_ref.clone(),
            clone_ref: obligation.clone_ref.clone(),
            cleanup_operation_ref: obligation.cleanup_operation_ref.clone(),
            resource_ref: obligation.resource_ref.clone(),
            outcome: CleanupOutcome::OutcomeUnknown,
            observation_ref: receipt('9'),
        },
    )
    .expect("uncertain cleanup");
    assert_eq!(uncertain.phase, CohortPhase::CleanupUncertain);

    let mut cleaned = failed;
    for (index, obligation) in cleaned.cleanup_obligations.clone().iter().enumerate() {
        cleaned = reduce_cleanup_observation(
            &cleaned,
            &CleanupObservation {
                cohort_ref: plan.cohort_ref.clone(),
                clone_ref: obligation.clone_ref.clone(),
                cleanup_operation_ref: obligation.cleanup_operation_ref.clone(),
                resource_ref: obligation.resource_ref.clone(),
                outcome: CleanupOutcome::Released,
                observation_ref: observation_ref(index + plan.effects.len()),
            },
        )
        .expect("cleanup observation");
    }
    assert_eq!(cleaned.phase, CohortPhase::Cleaned);
}

// r[verify vm_cohort.retry_policy]
#[test]
fn retry_requires_transient_class_current_budget_and_no_cancellation() {
    let operation_ref = OperationRef::new(reference('a')).expect("operation ref");
    let mut input = RetryInput {
        operation_ref: operation_ref.clone(),
        failure: FailureClass::TransientKvm,
        attempts_completed: 1,
        cancelled: false,
        limits: limits(),
    };
    let retry = plan_retry(&input).expect("bounded retry");
    assert_eq!(retry.operation_ref, operation_ref);
    assert_eq!(retry.delay_ticks, RETRY_DELAY_TICKS);

    input.failure = FailureClass::OutcomeUnknown;
    assert!(plan_retry(&input).is_none());
    input.failure = FailureClass::Permanent;
    assert!(plan_retry(&input).is_none());
    input.failure = FailureClass::TransientKvm;
    input.cancelled = true;
    assert!(plan_retry(&input).is_none());
    input.cancelled = false;
    input.attempts_completed = MAX_ATTEMPTS;
    assert!(plan_retry(&input).is_none());
}

fn effect_observation(
    plan: &CohortPlan,
    effect: &EffectRequest,
    outcome: OperationOutcome,
    index: usize,
) -> EffectObservation {
    EffectObservation {
        cohort_ref: plan.cohort_ref.clone(),
        clone_ref: effect.clone_ref.clone(),
        operation_ref: effect.operation_ref.clone(),
        kind: effect.kind,
        outcome,
        observation_ref: observation_ref(index),
    }
}

fn observation_ref(index: usize) -> ReceiptRef {
    let digest = blake3::hash(format!("observation-{index}").as_bytes());
    ReceiptRef::new(format!("blake3:{}", digest.to_hex())).expect("observation ref")
}
