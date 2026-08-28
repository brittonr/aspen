use transactional_reconciliation_core::AttemptState;
use transactional_reconciliation_core::DuplicateRiskAcknowledgement;
use transactional_reconciliation_core::ReservationObservation;
use transactional_reconciliation_core::UnknownOutcomeAcknowledgement;
use transactional_reconciliation_core::abandon_attempt;
use transactional_reconciliation_core::admit_dispatch;

use super::planning::expected_attempt_ref;
use super::planning::identity;
use super::planning::transaction_current_facts;
use super::*;

// r[impl molten.world_promotion.dispatch]
pub fn plan_world_dispatch(
    plan: &WorldPromotionPlan,
    reservation: &WorldReleaseReservation,
    facts: &WorldDispatchFacts,
    attempt_ref: &WorldReleaseAttemptRef,
) -> Result<WorldDispatchPlan, Vec<WorldPromotionIssue>> {
    validate_dispatch(plan, reservation, facts)?;
    let expected_attempt = expected_attempt_ref(reservation)?;
    if &expected_attempt != attempt_ref {
        return Err(vec![WorldPromotionIssue::AttemptIdentityReused]);
    }
    let shared = shared_release_operation(plan, &reservation.reservation_ref)?;
    let current = transaction_current_facts(plan)?;
    admit_dispatch(
        &plan.transaction.shared_plan,
        &current,
        shared.initial_attempt,
        ReservationObservation::Durable(shared.initial_attempt),
    )
    .map_err(transaction_error)?;
    Ok(WorldDispatchPlan {
        reservation_ref: reservation.reservation_ref.clone(),
        attempt_ref: attempt_ref.clone(),
        operation_ref: reservation.operation_ref.clone(),
        intent_ref: reservation.intent_ref.clone(),
        idempotency_ref: reservation.reservation_ref.clone(),
        dispatch_authorized: true,
        non_claims: promotion_non_claims(),
    })
}

// r[impl molten.world_promotion.dispatch]
pub fn plan_world_retry(
    plan: &WorldPromotionPlan,
    reservation: &WorldReleaseReservation,
    previous: &WorldAttemptRecord,
    next_attempt_ref: WorldReleaseAttemptRef,
    is_duplicate_risk_acknowledged: bool,
) -> Result<WorldRetryPlan, Vec<WorldPromotionIssue>> {
    if previous.reservation_ref != reservation.reservation_ref || previous.state != WorldReleaseState::Uncertain {
        return Err(vec![WorldPromotionIssue::ReservationNotCommitted]);
    }
    if previous.attempt_ref == next_attempt_ref {
        return Err(vec![WorldPromotionIssue::AttemptIdentityReused]);
    }
    if !is_duplicate_risk_acknowledged {
        return Err(vec![WorldPromotionIssue::RetryAcknowledgementRequired]);
    }
    let shared = shared_release_operation(plan, &reservation.reservation_ref)?;
    let current = transaction_current_facts(plan)?;
    let next_shared = transactional_reconciliation_core::plan_retry(
        &plan.transaction.shared_plan,
        &current,
        AttemptState::PendingUnobserved(shared.initial_attempt),
        identity(next_attempt_ref.as_str())?,
        DuplicateRiskAcknowledgement::Accepted,
    )
    .map_err(transaction_error)?;
    Ok(WorldRetryPlan {
        reservation_ref: reservation.reservation_ref.clone(),
        previous_attempt_ref: previous.attempt_ref.clone(),
        next_attempt_ref,
        shared_attempt: next_shared,
        duplicate_risk_acknowledged: true,
        same_logical_release: true,
        external_completion_proven: false,
        non_claims: promotion_non_claims(),
    })
}

pub fn classify_attempt_observation(
    dispatch: &WorldDispatchPlan,
    observation: WorldAttemptObservation,
) -> WorldAttemptRecord {
    let (state, observation_ref) = match observation {
        WorldAttemptObservation::Succeeded(reference) | WorldAttemptObservation::Failed(reference) => {
            (WorldReleaseState::Observed, Some(reference))
        }
        WorldAttemptObservation::Unknown => (WorldReleaseState::Uncertain, None),
        WorldAttemptObservation::Duplicate(reference) => (WorldReleaseState::Uncertain, Some(reference)),
        WorldAttemptObservation::Conflict(reference) => (WorldReleaseState::Conflict, Some(reference)),
    };
    WorldAttemptRecord {
        reservation_ref: dispatch.reservation_ref.clone(),
        attempt_ref: dispatch.attempt_ref.clone(),
        state,
        observation_ref,
        external_completion_proven: false,
    }
}

pub fn acknowledge_attempt(record: &WorldAttemptRecord) -> Result<WorldAttemptRecord, WorldPromotionIssue> {
    if record.state != WorldReleaseState::Observed || record.observation_ref.is_none() {
        return Err(WorldPromotionIssue::ReservationNotCommitted);
    }
    let mut acknowledged = record.clone();
    acknowledged.state = WorldReleaseState::Acknowledged;
    acknowledged.external_completion_proven = true;
    Ok(acknowledged)
}

pub fn abandon_uncertain_attempt(
    plan: &WorldPromotionPlan,
    reservation: &WorldReleaseReservation,
    record: &WorldAttemptRecord,
    is_unknown_outcome_acknowledged: bool,
) -> Result<WorldAttemptRecord, Vec<WorldPromotionIssue>> {
    if record.state != WorldReleaseState::Uncertain || record.reservation_ref != reservation.reservation_ref {
        return Err(vec![WorldPromotionIssue::ReservationNotCommitted]);
    }
    let shared = shared_release_operation(plan, &reservation.reservation_ref)?;
    abandon_attempt(
        AttemptState::PendingUnobserved(shared.initial_attempt),
        if is_unknown_outcome_acknowledged {
            UnknownOutcomeAcknowledgement::Accepted
        } else {
            UnknownOutcomeAcknowledgement::Missing
        },
    )
    .map_err(transaction_error)?;
    let mut abandoned = record.clone();
    abandoned.state = WorldReleaseState::Abandoned;
    abandoned.external_completion_proven = false;
    Ok(abandoned)
}

pub fn blocked_reservation(reservation: &WorldReleaseReservation) -> WorldReleaseReservation {
    let mut blocked = reservation.clone();
    blocked.state = WorldReleaseState::Blocked;
    blocked
}

fn validate_dispatch(
    plan: &WorldPromotionPlan,
    reservation: &WorldReleaseReservation,
    facts: &WorldDispatchFacts,
) -> Result<(), Vec<WorldPromotionIssue>> {
    let mut issues = Vec::with_capacity(MAX_WORLD_PROMOTION_DIAGNOSTICS);
    let is_known = plan.reservations.iter().any(|expected| {
        expected.reservation_ref == reservation.reservation_ref && expected.intent_ref == reservation.intent_ref
    });
    if !is_known || !matches!(reservation.state, WorldReleaseState::Committed | WorldReleaseState::Claimed) {
        issues.push(WorldPromotionIssue::ReservationNotCommitted);
    }
    if facts.observed_generation != reservation.generation {
        issues.push(WorldPromotionIssue::DispatchGenerationMismatch);
    }
    if !facts.authority_admitted {
        issues.push(WorldPromotionIssue::DispatchAuthorityDenied);
    }
    if !facts.policy_admitted {
        issues.push(WorldPromotionIssue::DispatchPolicyDenied);
    }
    if !facts.capability_admitted {
        issues.push(WorldPromotionIssue::DispatchCapabilityDenied);
    }
    if !facts.handler_matches {
        issues.push(WorldPromotionIssue::DispatchHandlerMismatch);
    }
    if !facts.adapter_matches {
        issues.push(WorldPromotionIssue::DispatchAdapterMismatch);
    }
    if issues.is_empty() {
        Ok(())
    } else {
        issues.sort();
        issues.dedup();
        Err(issues)
    }
}

fn shared_release_operation<'a>(
    plan: &'a WorldPromotionPlan,
    reservation_ref: &WorldReleaseReservationRef,
) -> Result<&'a TransactionalReleaseOperation, Vec<WorldPromotionIssue>> {
    plan.transaction
        .release_operations
        .iter()
        .find(|operation| &operation.reservation_ref == reservation_ref)
        .ok_or_else(|| {
            vec![WorldPromotionIssue::TransactionalMapping(
                "release-operation-missing".to_string(),
            )]
        })
}

fn transaction_error(error: transactional_reconciliation_core::CoreError) -> Vec<WorldPromotionIssue> {
    vec![WorldPromotionIssue::TransactionalMapping(format!("{error:?}"))]
}
