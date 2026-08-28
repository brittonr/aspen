use molten_core::world_branch_authority::WorldBranchAuthorityPlan;
use molten_core::world_branch_authority::WorldBranchPromotionReservationAdmission;
use molten_core::world_branch_authority::WorldBranchPromotionReservationFacts;
use molten_core::world_branch_authority::plan_world_branch_promotion_admission;
use molten_core::world_head::WorldBranchClass;
use molten_core::world_promotion::WorldPromotionPlan;
use molten_core::world_promotion::WorldReleaseReservation;
use molten_core::world_promotion::WorldReleaseReservationRef;
use molten_core::world_promotion::WorldReleaseState;
use molten_core::world_promotion::validate_reservation_set;

use crate::error::MoltenError;
use crate::error::Result;

// r[impl molten.world_branch_authority.activation]
pub fn bind_world_branch_promotion_reservation(
    authority_plan: &WorldBranchAuthorityPlan,
    promotion_plan: &WorldPromotionPlan,
    committed_reservations: &[WorldReleaseReservation],
    selected_reservation_ref: &WorldReleaseReservationRef,
) -> Result<WorldBranchPromotionReservationAdmission> {
    let selected = committed_reservations
        .iter()
        .find(|reservation| &reservation.reservation_ref == selected_reservation_ref)
        .ok_or_else(|| MoltenError::invalid_harness("promotion reservation observation is missing"))?;
    let observed_refs = committed_reservations
        .iter()
        .map(|reservation| reservation.reservation_ref.clone())
        .collect::<Vec<_>>();
    let expected = promotion_plan
        .reservations
        .iter()
        .find(|reservation| reservation.reservation_ref == selected.reservation_ref);
    let facts = WorldBranchPromotionReservationFacts {
        authority_plan_ref: authority_plan.plan_ref.clone(),
        promotion_plan_ref: promotion_plan.plan_ref.as_str().to_string(),
        reservation_ref: selected.reservation_ref.as_str().to_string(),
        candidate_head_ref: selected.candidate_head.as_str().to_string(),
        capability_ref: authority_plan.capability_ref.clone(),
        reservation_committed: selected.state == WorldReleaseState::Committed,
        complete_reservation_set: validate_reservation_set(promotion_plan, &observed_refs).is_ok(),
        reservation_matches_plan: expected.is_some_and(|expected| reservation_matches(expected, selected)),
        candidate_matches: selected.candidate_head == promotion_plan.after.head
            && promotion_plan.after.branch_class == WorldBranchClass::Release,
        external_effects_completed: promotion_plan.external_effects_completed,
        dispatch_authorized: false,
    };
    plan_world_branch_promotion_admission(authority_plan, &facts).map_err(|diagnostic| {
        MoltenError::invalid_harness(format!("promotion reservation admission denied: {diagnostic:?}"))
    })
}

fn reservation_matches(expected: &WorldReleaseReservation, observed: &WorldReleaseReservation) -> bool {
    expected.reservation_ref == observed.reservation_ref
        && expected.promotion_ref == observed.promotion_ref
        && expected.operation_ref == observed.operation_ref
        && expected.candidate_head == observed.candidate_head
        && expected.intent_ref == observed.intent_ref
        && expected.semantic_ref == observed.semantic_ref
        && expected.handler_ref == observed.handler_ref
        && expected.adapter_ref == observed.adapter_ref
        && expected.generation == observed.generation
}
