use molten_core::world_promotion::*;

use super::super::*;
use super::support::*;

#[test]
fn reservation_and_attempt_records_roundtrip_with_complete_nonclaims() {
    let plan = plan_world_promotion(&promotion_request()).expect("promotion plan");
    let mut reservation = plan.reservations[0].clone();
    reservation.state = WorldReleaseState::Committed;
    let canonical = canonical_reservation(&reservation).expect("reservation record");
    assert_eq!(parse_reservation(&canonical.bytes).expect("reservation parse"), reservation);

    let attempt = WorldAttemptRecord {
        reservation_ref: reservation.reservation_ref,
        attempt_ref: expected_initial_attempt(&reservation.operation_ref, &reservation.promotion_ref),
        state: WorldReleaseState::Uncertain,
        observation_ref: None,
        external_completion_proven: false,
    };
    let canonical = canonical_attempt(&attempt).expect("attempt record");
    assert_eq!(parse_attempt(&canonical.bytes).expect("attempt parse"), attempt);
}

#[test]
fn malformed_records_and_incomplete_nonclaims_fail_closed() {
    assert!(parse_reservation(b"not-preserves").is_err());
    assert!(parse_attempt(b"not-preserves").is_err());

    let plan = plan_world_promotion(&promotion_request()).expect("promotion plan");
    let persistence =
        classify_promotion_commit(&plan, &WorldPromotionCommitObservation::OutcomeUnknown).expect("persistence");
    assert!(canonical_persistence(&plan, &persistence).is_ok());
    let mut weakened = persistence;
    weakened.non_claims.pop();
    assert!(canonical_persistence(&plan, &weakened).is_err());
}

fn expected_initial_attempt(
    operation_ref: &WorldPromotionOperationRef,
    promotion_ref: &WorldPromotionPlanRef,
) -> WorldReleaseAttemptRef {
    WorldReleaseAttemptRef::new(reference(&format!("attempt:{operation_ref}:{promotion_ref}"))).expect("attempt ref")
}
