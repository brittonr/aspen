//! Signed-grant fixtures: verified promote and operator grants and current-facts ports that present
//! them.

use molten_core::world_promotion::*;

use super::super::*;
use super::support::CURRENT_GENERATION;
use crate::error::Result;

pub(super) const OBSERVED_UNIX_SECONDS: u64 = 1_000;
pub(super) const GRANT_EXPIRES_AT: u64 = 2_000;

pub(super) fn grant_nonce(label: &str) -> WorldPromotionGrantNonce {
    WorldPromotionGrantNonce::new(blake3::hash(label.as_bytes()).to_hex().to_string()).expect("grant nonce")
}

/// A verified operator grant for `target`, current for the promoted head of `request`. `label`
/// selects its single-use nonce.
pub(super) fn operator_grant(
    request: &WorldPromotionRequest,
    target: WorldPromotionGrantTarget,
    label: &str,
) -> WorldPromotionGrant {
    WorldPromotionGrant {
        branch_id: request.branch_id.clone(),
        authority_ref: request.authority.authority_ref.clone(),
        policy_ref: request.policy_ref.clone(),
        generation: CURRENT_GENERATION + 1,
        admitted: true,
        target,
        nonce: grant_nonce(label),
        expires_at: GRANT_EXPIRES_AT,
    }
}

pub(super) fn deny_grant(
    request: &WorldPromotionRequest,
    reservation_ref: &WorldReleaseReservationRef,
    label: &str,
) -> WorldPromotionGrant {
    operator_grant(
        request,
        WorldPromotionGrantTarget::DenyReservation {
            reservation_ref: reservation_ref.clone(),
        },
        label,
    )
}

pub(super) fn retry_grant(
    request: &WorldPromotionRequest,
    attempt: &WorldAttemptRecord,
    label: &str,
) -> WorldPromotionGrant {
    operator_grant(
        request,
        WorldPromotionGrantTarget::PlanRetry {
            reservation_ref: attempt.reservation_ref.clone(),
            attempt_ref: attempt.attempt_ref.clone(),
        },
        label,
    )
}

pub(super) fn abandon_grant(
    request: &WorldPromotionRequest,
    attempt: &WorldAttemptRecord,
    label: &str,
) -> WorldPromotionGrant {
    operator_grant(
        request,
        WorldPromotionGrantTarget::AbandonUncertain {
            reservation_ref: attempt.reservation_ref.clone(),
            attempt_ref: attempt.attempt_ref.clone(),
        },
        label,
    )
}

/// The promote grant a plan's operator would sign: exact candidate, predecessor generation, and a
/// nonce unique to the operation.
pub(super) fn promote_grant(plan: &WorldPromotionPlan, is_admitted: bool) -> WorldPromotionGrant {
    WorldPromotionGrant {
        branch_id: plan.before.branch_id.clone(),
        authority_ref: plan.authority_ref.clone(),
        policy_ref: plan.after.policy_ref.clone(),
        generation: plan.before.generation,
        admitted: is_admitted,
        target: WorldPromotionGrantTarget::Promote {
            candidate_head: plan.after.head.clone(),
        },
        nonce: grant_nonce(plan.operation_ref.as_str()),
        expires_at: GRANT_EXPIRES_AT,
    }
}

pub(super) fn transaction_facts(
    plan: &WorldPromotionPlan,
    grant: WorldPromotionGrant,
) -> WorldPromotionTransactionFacts {
    WorldPromotionTransactionFacts {
        grant,
        observed_unix_seconds: OBSERVED_UNIX_SECONDS,
        intent_closure_complete: true,
        intent_material_available: true,
        reservation_refs: plan.reservations.iter().map(|reservation| reservation.reservation_ref.clone()).collect(),
    }
}

pub(super) struct Current {
    pub is_admitted: bool,
}

impl WorldPromotionCurrentPort for Current {
    fn observe_transaction(&mut self, plan: &WorldPromotionPlan) -> Result<WorldPromotionTransactionFacts> {
        Ok(transaction_facts(plan, promote_grant(plan, self.is_admitted)))
    }
}

/// A current-facts port presenting one fixed grant at one observed time.
pub(super) struct PresentedGrant {
    pub grant: WorldPromotionGrant,
    pub observed_unix_seconds: u64,
}

impl WorldPromotionCurrentPort for PresentedGrant {
    fn observe_transaction(&mut self, plan: &WorldPromotionPlan) -> Result<WorldPromotionTransactionFacts> {
        let mut facts = transaction_facts(plan, self.grant.clone());
        facts.observed_unix_seconds = self.observed_unix_seconds;
        Ok(facts)
    }
}
