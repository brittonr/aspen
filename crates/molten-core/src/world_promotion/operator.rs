//! Pure currentness decisions for operator mutations and intent material.
//!
//! The shell verifies signatures and reads stored material; these functions only
//! compare the verified facts with the immutable original plan.

use super::*;
use crate::world_head::WorldBranchId;
use crate::world_head::WorldHeadPolicyRef;

/// One verified operator authority grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldOperatorAuthority {
    pub branch_id: WorldBranchId,
    pub authority_ref: WorldPromotionAuthorityRef,
    pub policy_ref: WorldHeadPolicyRef,
    pub generation: u64,
    pub admitted: bool,
}

/// Decides whether a verified operator grant is current for one reservation.
///
/// Every operator mutation requires an admitted grant at the reservation's
/// generation and matching the immutable original plan's branch, authority and
/// policy. The reservation must belong to that plan.
// r[impl molten.world_promotion.reconciliation]
pub fn validate_operator_authority(
    authority: &WorldOperatorAuthority,
    reservation: &WorldReleaseReservation,
    plan: &WorldPromotionPlan,
) -> Result<(), Vec<WorldPromotionIssue>> {
    let mut issues = Vec::with_capacity(MAX_WORLD_PROMOTION_DIAGNOSTICS);
    if !authority.admitted {
        issues.push(WorldPromotionIssue::AuthorityDenied);
    }
    if authority.generation != reservation.generation {
        issues.push(WorldPromotionIssue::AuthorityGenerationMismatch);
    }
    let is_planned = plan.reservations.iter().any(|planned| planned.reservation_ref == reservation.reservation_ref);
    if !is_planned || reservation.promotion_ref != plan.plan_ref || reservation.operation_ref != plan.operation_ref {
        issues.push(WorldPromotionIssue::ReservationSetMismatch);
    }
    if authority.branch_id != plan.after.branch_id || authority.authority_ref != plan.authority_ref {
        issues.push(WorldPromotionIssue::AuthorityDenied);
    }
    if authority.policy_ref != plan.after.policy_ref {
        issues.push(WorldPromotionIssue::AuthorityPolicyMismatch);
    }
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
