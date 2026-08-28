use super::super::planning::expected_attempt_ref;
use super::super::*;
use super::commit;
use super::dispatch_facts;
use super::promotion_request;
use super::reference;
use crate::world_commit::SnapshotProfileRef;

const EXPECTED_OBSERVATION_STEPS: usize = 1;
const OBSERVATION_BYTES: u64 = 64;

// r[verify molten.world_promotion.observation_commit]
#[test]
fn acknowledged_observation_plans_one_recorded_effect_successor() {
    let request = observation_commit_request();
    let first = plan_world_promotion_observation_commit(&request).expect("observation commit plan");
    let repeated = plan_world_promotion_observation_commit(&request).expect("repeated observation commit plan");
    assert_eq!(first, repeated);
    assert_eq!(first.parent_commit, request.reservation.candidate_head);
    assert_eq!(first.successor_commit, request.successor_commit);
    assert_eq!(first.trace.steps.len(), EXPECTED_OBSERVATION_STEPS);
    assert_eq!(first.trace.steps[0].input.kind, crate::world_replay::WorldTransitionInputKind::RecordedEffect);
    assert_eq!(first.trace.steps[0].input.input_ref, first.observation_ref.as_str());
    assert!(!first.mutates_promoted_commit);
    assert!(!first.grants_dispatch_authority);
    assert!(first.external_completion_proven);
    assert!(
        crate::world_replay::validate_world_transition_trace(
            &first.trace,
            &crate::world_replay::WorldReplayBounds::default(),
        )
        .is_empty()
    );
}

// r[verify molten.world_promotion.observation_commit]
#[test]
fn unacknowledged_mismatched_and_malformed_observations_fail_closed() {
    let request = observation_commit_request();

    let mut unacknowledged = request.clone();
    unacknowledged.attempt.state = WorldReleaseState::Uncertain;
    unacknowledged.attempt.external_completion_proven = false;
    unacknowledged.attempt.observation_ref = None;
    assert!(matches!(
        plan_world_promotion_observation_commit(&unacknowledged),
        Err(issues)
            if issues.contains(&WorldPromotionIssue::ObservationNotAcknowledged)
                && issues.contains(&WorldPromotionIssue::ObservationReferenceMissing)
    ));

    let mut uncommitted = request.clone();
    uncommitted.reservation.state = WorldReleaseState::Planned;
    assert!(matches!(
        plan_world_promotion_observation_commit(&uncommitted),
        Err(issues) if issues.contains(&WorldPromotionIssue::ReservationNotCommitted)
    ));

    let mut mismatched = request.clone();
    mismatched.attempt.reservation_ref =
        WorldReleaseReservationRef::new(reference("other-reservation")).expect("reservation ref");
    assert!(matches!(
        plan_world_promotion_observation_commit(&mismatched),
        Err(issues) if issues.contains(&WorldPromotionIssue::ObservationReservationMismatch)
    ));

    let mut unchanged = request.clone();
    unchanged.successor_commit = unchanged.reservation.candidate_head.clone();
    assert!(matches!(
        plan_world_promotion_observation_commit(&unchanged),
        Err(issues) if issues.contains(&WorldPromotionIssue::ObservationSuccessorUnchanged)
    ));

    let mut malformed = request;
    malformed.observation_byte_length = 0;
    assert!(matches!(
        plan_world_promotion_observation_commit(&malformed),
        Err(issues) if issues.contains(&WorldPromotionIssue::ObservationTraceInvalid)
    ));
}

fn observation_commit_request() -> WorldPromotionObservationCommitRequest {
    let plan = plan_world_promotion(&promotion_request()).expect("promotion plan");
    let mut reservation = plan.reservations[0].clone();
    reservation.state = WorldReleaseState::Committed;
    let attempt_ref = expected_attempt_ref(&reservation).expect("attempt ref");
    let dispatch = plan_world_dispatch(&plan, &reservation, &dispatch_facts(), &attempt_ref).expect("dispatch plan");
    let observed = classify_attempt_observation(
        &dispatch,
        WorldAttemptObservation::Succeeded(
            WorldReleaseObservationRef::new(reference("observation-success")).expect("observation ref"),
        ),
    );
    let attempt = acknowledge_attempt(&observed).expect("acknowledged attempt");
    WorldPromotionObservationCommitRequest {
        reservation,
        attempt,
        successor_commit: commit("observation-successor"),
        logical_profile_ref: SnapshotProfileRef::new(reference("logical-profile")).expect("profile ref"),
        observation_schema_ref: reference("observation-schema"),
        observation_byte_length: OBSERVATION_BYTES,
    }
}
