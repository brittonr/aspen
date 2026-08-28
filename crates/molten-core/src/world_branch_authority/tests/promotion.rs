use super::super::*;
use super::content_ref;
use super::current;
use super::observation;
use super::plan;

pub(super) fn admission(plan: &WorldBranchAuthorityPlan) -> WorldBranchPromotionReservationAdmission {
    plan_world_branch_promotion_admission(plan, &WorldBranchPromotionReservationFacts {
        authority_plan_ref: plan.plan_ref.clone(),
        promotion_plan_ref: content_ref("promotion-plan"),
        reservation_ref: content_ref("promotion-reservation"),
        candidate_head_ref: content_ref("promotion-candidate"),
        capability_ref: plan.capability_ref.clone(),
        reservation_committed: true,
        complete_reservation_set: true,
        reservation_matches_plan: true,
        candidate_matches: true,
        external_effects_completed: false,
        dispatch_authorized: false,
    })
    .expect("promotion admission")
}

// r[verify molten.world_branch_authority.activation]
#[test]
fn promotion_admission_rejects_missing_incomplete_and_dispatching_reservations() {
    let promotion = plan(CapabilityKind::DeferredEffect, WorldBranchAction::Promote);
    let mut missing_reservation = observation(&promotion);
    missing_reservation.promotion_admission = None;
    assert_eq!(
        decide_world_branch_activation(&promotion, &missing_reservation, &current()).diagnostic,
        WorldBranchAuthorityDiagnostic::PromotionReservationMissing
    );

    let mut dispatch_overclaim = observation(&promotion);
    dispatch_overclaim.promotion_admission.as_mut().expect("promotion admission").dispatch_authorized = true;
    assert_eq!(
        decide_world_branch_activation(&promotion, &dispatch_overclaim, &current()).diagnostic,
        WorldBranchAuthorityDiagnostic::PromotionDispatchOverclaim
    );

    let mut incomplete_reservations = observation(&promotion);
    incomplete_reservations
        .promotion_admission
        .as_mut()
        .expect("promotion admission")
        .complete_reservation_set = false;
    assert_eq!(
        decide_world_branch_activation(&promotion, &incomplete_reservations, &current()).diagnostic,
        WorldBranchAuthorityDiagnostic::PromotionReservationMissing
    );
}
