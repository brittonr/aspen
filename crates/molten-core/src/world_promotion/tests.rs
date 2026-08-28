use super::planning::expected_attempt_ref;
use super::*;
use crate::world_commit::WorldCommitRef;
use crate::world_head::WorldBranchClass;
use crate::world_head::WorldBranchId;
use crate::world_head::WorldHeadPolicyRef;

const CURRENT_GENERATION: u64 = 1;
const EXPECTED_RESERVATIONS: usize = 1;

mod observation;

// r[verify molten.world_promotion.plan]
// r[verify molten.world_promotion.transaction]
#[test]
fn complete_promotion_is_stable_and_binds_one_atomic_reservation_set() {
    let request = promotion_request();
    let first = plan_world_promotion(&request).expect("promotion plan");
    let repeated = plan_world_promotion(&request).expect("repeated promotion plan");
    assert_eq!(first, repeated);
    assert_eq!(first.reservations.len(), EXPECTED_RESERVATIONS);
    assert_eq!(first.after.branch_class, WorldBranchClass::Release);
    assert_eq!(first.after.generation, CURRENT_GENERATION + 1);
    assert!(!first.external_effects_completed);
    let refs = first.reservations.iter().map(|reservation| reservation.reservation_ref.clone()).collect::<Vec<_>>();
    assert_eq!(validate_reservation_set(&first, &refs), Ok(()));
    assert_eq!(first.transaction.release_operations.len(), EXPECTED_RESERVATIONS);
}

// r[verify molten.world_promotion.verification]
#[test]
fn incomplete_stale_simulated_and_duplicate_inputs_fail_closed() {
    let mut incomplete = promotion_request();
    incomplete.intent_closure_complete = false;
    assert!(matches!(
        plan_world_promotion(&incomplete),
        Err(issues) if issues.contains(&WorldPromotionIssue::IntentClosureIncomplete)
    ));

    let mut unclassified = promotion_request();
    unclassified.intents[0].release_class = None;
    assert!(matches!(
        plan_world_promotion(&unclassified),
        Err(issues) if issues.iter().any(|issue| matches!(issue, WorldPromotionIssue::IntentUnclassified(_)))
    ));

    let mut simulated = promotion_request();
    simulated.simulation_only = true;
    assert!(matches!(
        plan_world_promotion(&simulated),
        Err(issues) if issues.contains(&WorldPromotionIssue::SimulationBranchDenied)
    ));

    let mut denied = promotion_request();
    denied.authority.admitted = false;
    assert!(matches!(
        plan_world_promotion(&denied),
        Err(issues) if issues.contains(&WorldPromotionIssue::AuthorityDenied)
    ));

    let mut duplicate = promotion_request();
    duplicate.intents.push(duplicate.intents[0].clone());
    assert!(matches!(
        plan_world_promotion(&duplicate),
        Err(issues) if issues.iter().any(|issue| matches!(issue, WorldPromotionIssue::DuplicateIntent(_)))
    ));

    let plan = plan_world_promotion(&promotion_request()).expect("promotion plan");
    assert_eq!(validate_reservation_set(&plan, &[]), Err(WorldPromotionIssue::ReservationSetMismatch));
}

// r[verify molten.world_promotion.dispatch]
#[test]
fn dispatch_rechecks_current_facts_and_keeps_retry_identity_stable() {
    let plan = plan_world_promotion(&promotion_request()).expect("promotion plan");
    let mut reservation = plan.reservations[0].clone();
    reservation.state = WorldReleaseState::Committed;
    let attempt_ref = expected_attempt_ref(&reservation).expect("attempt ref");
    let dispatch = plan_world_dispatch(&plan, &reservation, &dispatch_facts(), &attempt_ref).expect("dispatch plan");
    assert!(dispatch.dispatch_authorized);
    assert_eq!(dispatch.idempotency_ref, reservation.reservation_ref);

    let uncertain = classify_attempt_observation(&dispatch, WorldAttemptObservation::Unknown);
    assert_eq!(uncertain.state, WorldReleaseState::Uncertain);

    let duplicate_observation =
        WorldReleaseObservationRef::new(reference("duplicate-observation")).expect("observation ref");
    let duplicate =
        classify_attempt_observation(&dispatch, WorldAttemptObservation::Duplicate(duplicate_observation.clone()));
    assert_eq!(duplicate.state, WorldReleaseState::Uncertain);
    assert_eq!(duplicate.observation_ref, Some(duplicate_observation));
    assert!(!duplicate.external_completion_proven);
    assert_eq!(acknowledge_attempt(&duplicate), Err(WorldPromotionIssue::ReservationNotCommitted));

    let conflict_observation =
        WorldReleaseObservationRef::new(reference("conflict-observation")).expect("observation ref");
    let conflict =
        classify_attempt_observation(&dispatch, WorldAttemptObservation::Conflict(conflict_observation.clone()));
    assert_eq!(conflict.state, WorldReleaseState::Conflict);
    assert_eq!(conflict.observation_ref, Some(conflict_observation));
    assert!(!conflict.external_completion_proven);

    let next_attempt = WorldReleaseAttemptRef::new(reference("retry-attempt")).expect("retry ref");
    assert!(matches!(
        plan_world_retry(&plan, &reservation, &uncertain, next_attempt.clone(), false),
        Err(issues) if issues.contains(&WorldPromotionIssue::RetryAcknowledgementRequired)
    ));
    let retry = plan_world_retry(&plan, &reservation, &uncertain, next_attempt, true).expect("acknowledged retry");
    assert!(retry.same_logical_release);
    assert!(!retry.external_completion_proven);
}

#[test]
fn denied_after_promotion_blocks_dispatch_without_rewriting_the_candidate() {
    let plan = plan_world_promotion(&promotion_request()).expect("promotion plan");
    let mut reservation = plan.reservations[0].clone();
    reservation.state = WorldReleaseState::Committed;
    let attempt_ref = expected_attempt_ref(&reservation).expect("attempt ref");
    let mut denied = dispatch_facts();
    denied.capability_admitted = false;
    assert!(matches!(
        plan_world_dispatch(&plan, &reservation, &denied, &attempt_ref),
        Err(issues) if issues.contains(&WorldPromotionIssue::DispatchCapabilityDenied)
    ));
    let blocked = blocked_reservation(&reservation);
    assert_eq!(blocked.state, WorldReleaseState::Blocked);
    assert_eq!(plan.after.head, promotion_request().candidate_head);
}

// r[verify molten.world_promotion.reconciliation]
#[test]
fn uncertain_publication_requires_readback_before_dispatch_eligibility() {
    let plan = plan_world_promotion(&promotion_request()).expect("promotion plan");
    let uncertain = classify_promotion_commit(&plan, &WorldPromotionCommitObservation::OutcomeUnknown)
        .expect("uncertain classification");
    assert!(!uncertain.dispatch_eligible);
    assert!(!uncertain.mutation_authorized_by_evidence);
    let reconciled = reconcile_promotion_read_back(&plan, &uncertain, &WorldPromotionReadBackObservation::Reservation)
        .expect("reservation readback");
    assert!(reconciled.dispatch_eligible);

    let conflicting = classify_promotion_commit(&plan, &WorldPromotionCommitObservation::NotApplied {
        current_head: commit("foreign-head"),
        current_generation: CURRENT_GENERATION + 1,
    })
    .expect("conflicting classification");
    assert!(!conflicting.dispatch_eligible);
}

#[test]
fn terminal_acknowledgment_and_abandonment_do_not_claim_exactly_once() {
    let plan = plan_world_promotion(&promotion_request()).expect("promotion plan");
    let mut reservation = plan.reservations[0].clone();
    reservation.state = WorldReleaseState::Committed;
    let attempt_ref = expected_attempt_ref(&reservation).expect("attempt ref");
    let dispatch = plan_world_dispatch(&plan, &reservation, &dispatch_facts(), &attempt_ref).expect("dispatch plan");
    let observed = classify_attempt_observation(
        &dispatch,
        WorldAttemptObservation::Succeeded(
            WorldReleaseObservationRef::new(reference("success")).expect("observation ref"),
        ),
    );
    let acknowledged = acknowledge_attempt(&observed).expect("acknowledged attempt");
    assert!(acknowledged.external_completion_proven);

    let uncertain = classify_attempt_observation(&dispatch, WorldAttemptObservation::Unknown);
    let abandoned =
        abandon_uncertain_attempt(&plan, &reservation, &uncertain, true).expect("abandoned unknown attempt");
    assert_eq!(abandoned.state, WorldReleaseState::Abandoned);
    assert!(!abandoned.external_completion_proven);
    assert!(plan.non_claims.iter().any(|claim| claim.contains("exactly-once")));
}

fn promotion_request() -> WorldPromotionRequest {
    let policy_ref = WorldHeadPolicyRef::new(reference("promotion-policy")).expect("policy ref");
    WorldPromotionRequest {
        operation_ref: WorldPromotionOperationRef::new(reference("promotion-operation")).expect("operation ref"),
        branch_id: WorldBranchId::new("release").expect("branch"),
        branch_class: WorldBranchClass::Candidate,
        expected_head: commit("active"),
        candidate_head: commit("candidate"),
        expected_generation: CURRENT_GENERATION,
        policy_ref: policy_ref.clone(),
        authority: WorldPromotionAuthorityObservation {
            authority_ref: WorldPromotionAuthorityRef::new(reference("promotion-authority")).expect("authority ref"),
            policy_ref,
            observed_generation: CURRENT_GENERATION,
            admitted: true,
        },
        intent_closure_complete: true,
        simulation_only: false,
        intents: vec![
            intent("release", WorldIntentReleaseClass::Release),
            intent("retain", WorldIntentReleaseClass::Retain),
        ],
        bounds: WorldPromotionBounds::standard(),
    }
}

fn intent(label: &str, class: WorldIntentReleaseClass) -> WorldEffectIntent {
    WorldEffectIntent {
        intent_ref: WorldEffectIntentRef::new(reference(&format!("intent:{label}"))).expect("intent ref"),
        semantic_ref: WorldSemanticIntentRef::new(reference(&format!("semantic:{label}"))).expect("semantic ref"),
        handler_ref: WorldPromotionHandlerRef::new(reference(&format!("handler:{label}"))).expect("handler ref"),
        adapter_ref: WorldPromotionAdapterRef::new(reference(&format!("adapter:{label}"))).expect("adapter ref"),
        release_class: Some(class),
    }
}

fn dispatch_facts() -> WorldDispatchFacts {
    WorldDispatchFacts {
        observed_generation: CURRENT_GENERATION + 1,
        authority_admitted: true,
        policy_admitted: true,
        capability_admitted: true,
        handler_matches: true,
        adapter_matches: true,
    }
}

fn commit(label: &str) -> WorldCommitRef {
    WorldCommitRef::new(reference(label)).expect("commit ref")
}

fn reference(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}
