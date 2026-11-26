//! Operator deny, retry planning, and abandon under current operator authority.
//!
//! Each mutation decides a verified grant against the live branch head, the grant's consumption,
//! the stored reservation and the immutable original plan, then applies its writes and consumes
//! the grant in one store write transaction that rechecks the head it was decided against.

use molten_core::world_head::WorldHeadState;
use molten_core::world_promotion::*;

use super::*;
use crate::error::Failure;
use crate::error::Result;

pub struct DenyInput<'a, S> {
    pub store: &'a mut S,
    pub request: &'a WorldPromotionRequest,
    pub operator: &'a WorldPromotionGrant,
    pub reservation_ref: &'a WorldReleaseReservationRef,
    pub observed_unix_seconds: u64,
}

/// Denies one stored reservation under a current admitted `deny-reservation` grant for it.
// r[impl molten.world_promotion.reconciliation]
// r[impl molten.world_commit.promotion_authority_action_and_canonical_binding]
pub fn deny_reservation<S: WorldPromotionTransactionPort>(input: DenyInput<'_, S>) -> Result<WorldReleaseReservation> {
    let plan = rebuild_committed_plan(input.store, input.request)?;
    let reservation = input
        .store
        .read_reservation(input.reservation_ref)?
        .ok_or_else(|| Failure::invalid_harness("world deny reservation is missing"))?;
    let requested = WorldPromotionGrantTarget::DenyReservation {
        reservation_ref: input.reservation_ref.clone(),
    };
    let validated_head = authorize(&*input.store, "deny", OperatorRequest {
        grant: input.operator,
        requested: &requested,
        reservation: &reservation,
        plan: &plan,
        observed_unix_seconds: input.observed_unix_seconds,
    })?;
    if !matches!(
        reservation.state,
        WorldReleaseState::Committed | WorldReleaseState::Blocked | WorldReleaseState::Denied
    ) {
        return Err(Failure::invalid_harness(
            "world deny cannot replace an attempted release; reconcile or acknowledge abandon",
        ));
    }
    let denied = denied_reservation(&reservation);
    input.store.apply_operator_mutation(&WorldOperatorMutation {
        grant: input.operator.clone(),
        validated_head,
        reservation: Some(denied.clone()),
        attempt: None,
    })?;
    Ok(denied)
}

pub struct RetryPlanInput<'a, S> {
    pub store: &'a mut S,
    pub request: &'a WorldPromotionRequest,
    pub operator: &'a WorldPromotionGrant,
    pub reservation_ref: &'a WorldReleaseReservationRef,
    pub attempt_ref: &'a WorldReleaseAttemptRef,
    pub next_attempt_ref: WorldReleaseAttemptRef,
    pub is_duplicate_risk_acknowledged: bool,
    pub observed_unix_seconds: u64,
}

/// Rebuilds the immutable original plan from the operator's request document and plans one retry
/// for a stored uncertain attempt under a current `plan-retry` grant for that reservation and
/// attempt. A successful plan consumes the grant.
// r[impl molten.world_promotion.reconciliation]
// r[impl molten.world_commit.promotion_authority_action_and_canonical_binding]
pub fn plan_retry_for_reservation<S: WorldPromotionTransactionPort>(
    input: RetryPlanInput<'_, S>,
) -> Result<WorldRetryPlan> {
    let plan = rebuild_committed_plan(input.store, input.request)?;
    let reservation = input
        .store
        .read_reservation(input.reservation_ref)?
        .ok_or_else(|| Failure::invalid_harness("world retry reservation is missing"))?;
    let requested = WorldPromotionGrantTarget::PlanRetry {
        reservation_ref: input.reservation_ref.clone(),
        attempt_ref: input.attempt_ref.clone(),
    };
    let validated_head = authorize(&*input.store, "retry", OperatorRequest {
        grant: input.operator,
        requested: &requested,
        reservation: &reservation,
        plan: &plan,
        observed_unix_seconds: input.observed_unix_seconds,
    })?;
    let attempt = input
        .store
        .read_attempt(input.attempt_ref)?
        .ok_or_else(|| Failure::invalid_harness("world retry attempt is missing"))?;
    let retry =
        plan_world_retry(&plan, &reservation, &attempt, input.next_attempt_ref, input.is_duplicate_risk_acknowledged)
            .map_err(|issues| Failure::invalid_harness(format!("world retry denied: {issues:?}")))?;
    input.store.apply_operator_mutation(&WorldOperatorMutation {
        grant: input.operator.clone(),
        validated_head,
        reservation: None,
        attempt: None,
    })?;
    Ok(retry)
}

pub struct AbandonInput<'a, S> {
    pub store: &'a mut S,
    pub request: &'a WorldPromotionRequest,
    pub operator: &'a WorldPromotionGrant,
    pub attempt_ref: &'a WorldReleaseAttemptRef,
    pub is_unknown_outcome_acknowledged: bool,
    pub observed_unix_seconds: u64,
}

/// Rebuilds the original plan and abandons one stored uncertain attempt and its uncertain
/// reservation together under a current `abandon-uncertain` grant for them.
// r[impl molten.world_promotion.reconciliation]
// r[impl molten.world_commit.promotion_authority_action_and_canonical_binding]
// r[impl molten.world_commit.promotion_atomic_dispatch_and_readback]
pub fn abandon_uncertain_for_attempt<S: WorldPromotionTransactionPort>(
    input: AbandonInput<'_, S>,
) -> Result<WorldAbandonment> {
    let plan = rebuild_committed_plan(input.store, input.request)?;
    let attempt = input
        .store
        .read_attempt(input.attempt_ref)?
        .ok_or_else(|| Failure::invalid_harness("world abandon attempt is missing"))?;
    let reservation = input
        .store
        .read_reservation(&attempt.reservation_ref)?
        .ok_or_else(|| Failure::invalid_harness("world abandon reservation is missing"))?;
    let requested = WorldPromotionGrantTarget::AbandonUncertain {
        reservation_ref: attempt.reservation_ref.clone(),
        attempt_ref: input.attempt_ref.clone(),
    };
    let validated_head = authorize(&*input.store, "abandon", OperatorRequest {
        grant: input.operator,
        requested: &requested,
        reservation: &reservation,
        plan: &plan,
        observed_unix_seconds: input.observed_unix_seconds,
    })?;
    let abandonment = abandon_uncertain_attempt(&plan, &reservation, &attempt, input.is_unknown_outcome_acknowledged)
        .map_err(|issues| Failure::invalid_harness(format!("world abandon denied: {issues:?}")))?;
    input.store.apply_operator_mutation(&WorldOperatorMutation {
        grant: input.operator.clone(),
        validated_head,
        reservation: Some(abandonment.reservation.clone()),
        attempt: Some(abandonment.attempt.clone()),
    })?;
    Ok(abandonment)
}

struct OperatorRequest<'a> {
    grant: &'a WorldPromotionGrant,
    requested: &'a WorldPromotionGrantTarget,
    reservation: &'a WorldReleaseReservation,
    plan: &'a WorldPromotionPlan,
    observed_unix_seconds: u64,
}

/// Decides the grant against the live branch head and its consumption, returning the head it was
/// decided against so the store can reject the mutation if the head moves before it commits.
fn authorize<S: WorldPromotionTransitionPort>(
    store: &S,
    label: &str,
    request: OperatorRequest<'_>,
) -> Result<WorldHeadState> {
    let live = store.observe_live(&request.plan.after.branch_id, &request.grant.nonce)?;
    validate_operator_authority(WorldOperatorAuthorityInput {
        grant: request.grant,
        requested: request.requested,
        reservation: request.reservation,
        plan: request.plan,
        live: &live,
        observed_unix_seconds: request.observed_unix_seconds,
    })
    .map_err(|issues| Failure::invalid_harness(format!("world {label} authority denied: {issues:?}")))?;
    live.head
        .ok_or_else(|| Failure::invalid_harness(format!("world {label} authority denied: branch head missing")))
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
