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
        outcome: WorldAttemptOutcome::Unknown,
        external_completion_proven: false,
    };
    let canonical = canonical_attempt(&attempt).expect("attempt record");
    assert_eq!(parse_attempt(&canonical.bytes).expect("attempt parse"), attempt);
}

// r[verify molten.world_promotion.non_claims]
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

// r[verify molten.world_promotion.dispatch]
#[test]
fn attempt_record_v2_roundtrips_each_outcome() {
    for outcome in [
        WorldAttemptOutcome::Unknown,
        WorldAttemptOutcome::Succeeded,
        WorldAttemptOutcome::Failed,
    ] {
        let attempt = observed_attempt(outcome);
        let canonical = canonical_attempt(&attempt).expect("attempt record");
        assert_eq!(parse_attempt(&canonical.bytes).expect("attempt parse"), attempt);
        assert_eq!(canonical.bytes, attempt_bytes(WORLD_RELEASE_ATTEMPT_SCHEMA, &attempt, Some(outcome.as_str())));
    }
}

// r[verify molten.world_promotion.dispatch]
#[test]
fn attempt_record_rejects_unsupported_outcome_and_v1_layout() {
    let attempt = observed_attempt(WorldAttemptOutcome::Succeeded);
    assert!(parse_attempt(&attempt_bytes(WORLD_RELEASE_ATTEMPT_SCHEMA, &attempt, Some("bogus"))).is_err());
    // A v1 record cannot say whether the release succeeded, so it is never read as an attempt.
    assert!(parse_attempt(&attempt_bytes("molten.world-release-attempt.v1", &attempt, None)).is_err());
    assert!(parse_attempt(&attempt_bytes("molten.world-release-attempt.v1", &attempt, Some("succeeded"))).is_err());
}

fn observed_attempt(outcome: WorldAttemptOutcome) -> WorldAttemptRecord {
    let plan = plan_world_promotion(&promotion_request()).expect("promotion plan");
    let reservation = plan.reservations[0].clone();
    WorldAttemptRecord {
        attempt_ref: expected_initial_attempt(&reservation.operation_ref, &reservation.promotion_ref),
        reservation_ref: reservation.reservation_ref,
        state: WorldReleaseState::Observed,
        observation_ref: Some(WorldReleaseObservationRef::new(reference("observation")).expect("observation ref")),
        outcome,
        external_completion_proven: false,
    }
}

/// Hand-built attempt bytes; `outcome: None` reproduces the seven-field v1 layout.
fn attempt_bytes(schema: &str, attempt: &WorldAttemptRecord, outcome: Option<&str>) -> Vec<u8> {
    use crate::preserves_rail::record;
    use crate::preserves_rail::string;
    let named = |label: &'static str, value| record(label, vec![value]);
    let observation = attempt
        .observation_ref
        .as_ref()
        .map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value.as_str())]));
    let mut fields = vec![
        string(schema),
        named("reservation-ref", string(attempt.reservation_ref.as_str())),
        named("attempt-ref", string(attempt.attempt_ref.as_str())),
        named("state", string(attempt.state.as_str())),
        named("observation-ref", observation),
    ];
    if let Some(outcome) = outcome {
        fields.push(named("outcome", string(outcome)));
    }
    fields.push(named(
        "external-completion-proven",
        record(
            if attempt.external_completion_proven {
                "true"
            } else {
                "false"
            },
            Vec::new(),
        ),
    ));
    fields.push(named(
        "non-claims",
        crate::preserves_rail::sequence(WORLD_PROMOTION_NON_CLAIMS.iter().copied().map(string).collect()),
    ));
    crate::preserves_rail::canonical_bytes(&record("world-release-attempt", fields)).expect("attempt bytes")
}
