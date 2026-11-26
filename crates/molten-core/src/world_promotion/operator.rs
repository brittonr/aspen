//! Pure currentness decisions for promotion and operator grants and intent material.
//!
//! The shell verifies signatures, reads the live branch head and grant consumption inside its
//! store write transaction, and reads stored material; these functions only compare those
//! verified facts with the immutable original plan.

use super::*;

/// One requested operator mutation and the facts it is decided against.
#[derive(Debug, Clone, Copy)]
pub struct WorldOperatorAuthorityInput<'a> {
    pub grant: &'a WorldPromotionGrant,
    /// The exact action and object the operator is asking to change.
    pub requested: &'a WorldPromotionGrantTarget,
    pub reservation: &'a WorldReleaseReservation,
    pub plan: &'a WorldPromotionPlan,
    pub live: &'a WorldPromotionLiveFacts,
    pub observed_unix_seconds: u64,
}

/// Decides whether a verified grant is current for one operator mutation.
///
/// The grant must be admitted, unexpired and unconsumed, name exactly the requested action and
/// target, and be issued at the live branch head's generation under its policy. It must match the
/// immutable original plan's branch, authority and policy, and the reservation must belong to that
/// plan.
// r[impl molten.world_promotion.reconciliation]
// r[impl molten.world_commit.promotion_authority_action_and_canonical_binding]
pub fn validate_operator_authority(input: WorldOperatorAuthorityInput<'_>) -> Result<(), Vec<WorldPromotionIssue>> {
    let mut issues = Vec::with_capacity(MAX_WORLD_PROMOTION_DIAGNOSTICS);
    push_grant_issues(input.grant, input.requested, input.live, input.observed_unix_seconds, &mut issues);
    match &input.live.head {
        Some(head) if head.branch_id == input.plan.after.branch_id => {
            if input.grant.generation != head.generation {
                issues.push(WorldPromotionIssue::AuthorityGenerationMismatch);
            }
            if input.grant.policy_ref != head.policy_ref {
                issues.push(WorldPromotionIssue::AuthorityPolicyMismatch);
            }
            if input.reservation.generation > head.generation {
                issues.push(WorldPromotionIssue::AuthorityHeadStale);
            }
        }
        _ => issues.push(WorldPromotionIssue::AuthorityHeadStale),
    }
    let plan = input.plan;
    let reservation = input.reservation;
    let is_planned = plan.reservations.iter().any(|planned| planned.reservation_ref == reservation.reservation_ref);
    if !is_planned || reservation.promotion_ref != plan.plan_ref || reservation.operation_ref != plan.operation_ref {
        issues.push(WorldPromotionIssue::ReservationSetMismatch);
    }
    if input.grant.branch_id != plan.after.branch_id || input.grant.authority_ref != plan.authority_ref {
        issues.push(WorldPromotionIssue::AuthorityDenied);
    }
    if input.grant.policy_ref != plan.after.policy_ref {
        issues.push(WorldPromotionIssue::AuthorityPolicyMismatch);
    }
    finish(issues)
}

/// Checks shared by every grant use: admitted, unexpired, unconsumed, and naming exactly the
/// expected action and target.
#[allow(
    tigerstyle::borrowed_argument_types,
    reason = "the bounded validator appends typed issues to one preallocated local sink"
)]
pub(super) fn push_grant_issues(
    grant: &WorldPromotionGrant,
    expected: &WorldPromotionGrantTarget,
    live: &WorldPromotionLiveFacts,
    observed_unix_seconds: u64,
    issues: &mut Vec<WorldPromotionIssue>,
) {
    if !grant.admitted {
        issues.push(WorldPromotionIssue::AuthorityDenied);
    }
    if observed_unix_seconds >= grant.expires_at {
        issues.push(WorldPromotionIssue::AuthorityExpired);
    }
    if live.grant_nonce_consumed {
        issues.push(WorldPromotionIssue::AuthorityNonceConsumed);
    }
    if grant.target.action() != expected.action() {
        issues.push(WorldPromotionIssue::AuthorityActionMismatch);
    } else if grant.target != *expected {
        issues.push(WorldPromotionIssue::AuthorityTargetMismatch);
    }
}

fn finish(mut issues: Vec<WorldPromotionIssue>) -> Result<(), Vec<WorldPromotionIssue>> {
    if issues.is_empty() {
        Ok(())
    } else {
        issues.sort();
        issues.dedup();
        Err(issues)
    }
}

/// True when the material read for a plan names exactly the plan's intents:
/// one material per intent, each equal in identity, semantic, handler, adapter,
/// and release classification.
// r[impl molten.world_promotion.transaction]
pub fn intent_material_matches(plan: &WorldPromotionPlan, material: &[WorldEffectIntent]) -> bool {
    plan.intents.len() == material.len() && plan.intents.iter().all(|intent| material.contains(intent))
}
