//! Operator deny, retry planning, and abandon under current operator authority.
//!
//! Each mutation rechecks a verified operator grant against the stored
//! reservation and immutable original plan before changing bookkeeping.

use molten_core::world_promotion::*;

use super::*;
use crate::error::Failure;
use crate::error::Result;

pub struct DenyInput<'a, S> {
    pub store: &'a mut S,
    pub request: &'a WorldPromotionRequest,
    pub operator: &'a WorldOperatorAuthority,
    pub reservation_ref: &'a WorldReleaseReservationRef,
}

/// Denies one stored reservation under a current admitted operator grant.
// r[impl molten.world_promotion.reconciliation]
pub fn deny_reservation<S: WorldPromotionTransactionPort>(input: DenyInput<'_, S>) -> Result<WorldReleaseReservation> {
    let plan = rebuild_committed_plan(input.store, input.request)?;
    let reservation = input
        .store
        .read_reservation(input.reservation_ref)?
        .ok_or_else(|| Failure::invalid_harness("world deny reservation is missing"))?;
    validate_operator_authority(input.operator, &reservation, &plan)
        .map_err(|issues| Failure::invalid_harness(format!("world deny authority denied: {issues:?}")))?;
    if !matches!(
        reservation.state,
        WorldReleaseState::Committed | WorldReleaseState::Blocked | WorldReleaseState::Denied
    ) {
        return Err(Failure::invalid_harness(
            "world deny cannot replace an attempted release; reconcile or acknowledge abandon",
        ));
    }
    let denied = denied_reservation(&reservation);
    input.store.update_reservation(&denied)?;
    Ok(denied)
}

pub struct RetryPlanInput<'a, S> {
    pub store: &'a S,
    pub request: &'a WorldPromotionRequest,
    pub operator: &'a WorldOperatorAuthority,
    pub reservation_ref: &'a WorldReleaseReservationRef,
    pub attempt_ref: &'a WorldReleaseAttemptRef,
    pub next_attempt_ref: WorldReleaseAttemptRef,
    pub is_duplicate_risk_acknowledged: bool,
}

/// Rebuilds the immutable original plan from the operator's request document and
/// plans one retry for a stored uncertain attempt under current operator authority.
// r[impl molten.world_promotion.reconciliation]
pub fn plan_retry_for_reservation<S: WorldPromotionTransactionPort>(
    input: RetryPlanInput<'_, S>,
) -> Result<WorldRetryPlan> {
    let plan = rebuild_committed_plan(input.store, input.request)?;
    let reservation = input
        .store
        .read_reservation(input.reservation_ref)?
        .ok_or_else(|| Failure::invalid_harness("world retry reservation is missing"))?;
    validate_operator_authority(input.operator, &reservation, &plan)
        .map_err(|issues| Failure::invalid_harness(format!("world retry authority denied: {issues:?}")))?;
    let attempt = input
        .store
        .read_attempt(input.attempt_ref)?
        .ok_or_else(|| Failure::invalid_harness("world retry attempt is missing"))?;
    plan_world_retry(&plan, &reservation, &attempt, input.next_attempt_ref, input.is_duplicate_risk_acknowledged)
        .map_err(|issues| Failure::invalid_harness(format!("world retry denied: {issues:?}")))
}

pub struct AbandonInput<'a, S> {
    pub store: &'a mut S,
    pub request: &'a WorldPromotionRequest,
    pub operator: &'a WorldOperatorAuthority,
    pub attempt_ref: &'a WorldReleaseAttemptRef,
    pub is_unknown_outcome_acknowledged: bool,
}

/// Rebuilds the original plan and abandons one stored uncertain attempt under
/// current operator authority.
// r[impl molten.world_promotion.reconciliation]
pub fn abandon_uncertain_for_attempt<S: WorldPromotionTransactionPort>(
    input: AbandonInput<'_, S>,
) -> Result<WorldAttemptRecord> {
    let plan = rebuild_committed_plan(input.store, input.request)?;
    let attempt = input
        .store
        .read_attempt(input.attempt_ref)?
        .ok_or_else(|| Failure::invalid_harness("world abandon attempt is missing"))?;
    let reservation = input
        .store
        .read_reservation(&attempt.reservation_ref)?
        .ok_or_else(|| Failure::invalid_harness("world abandon reservation is missing"))?;
    validate_operator_authority(input.operator, &reservation, &plan)
        .map_err(|issues| Failure::invalid_harness(format!("world abandon authority denied: {issues:?}")))?;
    let abandoned = abandon_uncertain_attempt(&plan, &reservation, &attempt, input.is_unknown_outcome_acknowledged)
        .map_err(|issues| Failure::invalid_harness(format!("world abandon denied: {issues:?}")))?;
    input.store.store_attempt(&abandoned)?;
    Ok(abandoned)
}

/// Rebuilds the plan that a committed promotion published.
///
/// The request document's `authority_admitted` and `intent_closure_complete`
/// fields are planning inputs, never authority. The plan is rebuilt with the
/// facts its committed transaction validated, and accepted only when the store
/// holds that exact committed identity for the operation; a request that differs
/// in any identity field cannot name a different plan.
fn rebuild_committed_plan<S: WorldPromotionCommitPort>(
    store: &S,
    request: &WorldPromotionRequest,
) -> Result<WorldPromotionPlan> {
    let mut original = request.clone();
    original.authority.admitted = true;
    original.authority.observed_generation = original.expected_generation;
    original.intent_closure_complete = true;
    let plan = plan_world_promotion(&original)
        .map_err(|issues| Failure::invalid_harness(format!("world original plan cannot be rebuilt: {issues:?}")))?;
    let is_committed = match store.read_promotion_identity(&plan.operation_ref)? {
        Some(StoredPromotionIdentity::Current(identity)) => identity == plan.identity,
        Some(StoredPromotionIdentity::Legacy { plan_ref, .. }) => plan_ref == plan.plan_ref,
        None => false,
    };
    if is_committed {
        Ok(plan)
    } else {
        Err(Failure::invalid_harness("world operator request does not match a committed promotion"))
    }
}
