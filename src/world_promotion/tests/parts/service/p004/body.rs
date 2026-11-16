fn promoted_with_uncertain_attempt(
    request: &WorldPromotionRequest,
) -> (TestState, WorldReleaseReservation, WorldAttemptRecord) {
    let mut state = test_state(request);
    promote_world(request, WorldPromotionPorts {
        current: &mut Current { is_admitted: true },
        transaction: &mut state.store,
        receipts: &mut receipts(),
    })
    .expect("promotion");
    let reservation = state.store.list_reservations().expect("reservations").pop().expect("reservation");
    let attempt = WorldAttemptRecord {
        reservation_ref: reservation.reservation_ref.clone(),
        attempt_ref: WorldReleaseAttemptRef::new(reference("operator-attempt")).expect("attempt"),
        state: WorldReleaseState::Uncertain,
        observation_ref: None,
        outcome: WorldAttemptOutcome::Unknown,
        external_completion_proven: false,
    };
    state.store.store_attempt(&attempt).expect("store attempt");
    (state, reservation, attempt)
}

// r[verify molten.world_promotion.reconciliation]
#[test]
fn stale_revoked_or_foreign_operator_authority_cannot_deny_retry_or_abandon() {
    let request = promotion_request();
    let (mut state, reservation, attempt) = promoted_with_uncertain_attempt(&request);
    let mut revoked = current_operator(&request);
    revoked.admitted = false;
    let mut stale = current_operator(&request);
    stale.generation = CURRENT_GENERATION;
    let mut foreign = current_operator(&request);
    foreign.authority_ref = WorldPromotionAuthorityRef::new(reference("foreign-authority")).expect("authority");
    for operator in [&revoked, &stale, &foreign] {
        assert!(
            plan_retry_for_reservation(RetryPlanInput {
                store: &state.store,
                request: &request,
                operator,
                reservation_ref: &reservation.reservation_ref,
                attempt_ref: &attempt.attempt_ref,
                next_attempt_ref: WorldReleaseAttemptRef::new(reference("operator-next")).expect("next"),
                is_duplicate_risk_acknowledged: true,
            })
            .is_err()
        );
        assert!(
            abandon_uncertain_for_attempt(AbandonInput {
                store: &mut state.store,
                request: &request,
                operator,
                attempt_ref: &attempt.attempt_ref,
                is_unknown_outcome_acknowledged: true,
            })
            .is_err()
        );
    }
    for operator in [&revoked, &stale, &foreign] {
        assert!(
            deny_reservation(DenyInput {
                store: &mut state.store,
                request: &request,
                operator,
                reservation_ref: &reservation.reservation_ref,
            })
            .is_err()
        );
    }
    let stored_attempt = state.store.read_attempt(&attempt.attempt_ref).expect("read").expect("attempt");
    assert_eq!(stored_attempt, attempt);
    let stored_reservation =
        state.store.read_reservation(&reservation.reservation_ref).expect("read").expect("reservation");
    assert_eq!(stored_reservation, reservation);
}

// r[verify molten.world_promotion.reconciliation]
// r[verify molten.promotion_identity.restart_binding]
#[test]
fn operator_requests_rebuild_only_the_committed_original_plan() {
    let request = promotion_request();
    let (mut state, reservation, attempt) = promoted_with_uncertain_attempt(&request);

    // The document's authority and closure booleans are planning inputs, not authority.
    let mut unadmitted_document = request.clone();
    unadmitted_document.authority.admitted = false;
    unadmitted_document.intent_closure_complete = false;
    let retry = plan_retry_for_reservation(RetryPlanInput {
        store: &state.store,
        request: &unadmitted_document,
        operator: &current_operator(&request),
        reservation_ref: &reservation.reservation_ref,
        attempt_ref: &attempt.attempt_ref,
        next_attempt_ref: WorldReleaseAttemptRef::new(reference("operator-next")).expect("next"),
        is_duplicate_risk_acknowledged: true,
    })
    .expect("retry plan");
    assert!(retry.same_logical_release);
    assert!(retry.duplicate_risk_acknowledged);

    // A request naming another candidate or operation is not the committed plan.
    let mut other_candidate = request.clone();
    other_candidate.candidate_head = WorldCommitRef::new(reference("other-candidate")).expect("candidate");
    let mut other_operation = request.clone();
    other_operation.operation_ref =
        WorldPromotionOperationRef::new(reference("replacement-operation")).expect("operation");
    for replacement in [&other_candidate, &other_operation] {
        assert!(
            plan_retry_for_reservation(RetryPlanInput {
                store: &state.store,
                request: replacement,
                operator: &current_operator(&request),
                reservation_ref: &reservation.reservation_ref,
                attempt_ref: &attempt.attempt_ref,
                next_attempt_ref: WorldReleaseAttemptRef::new(reference("operator-next")).expect("next"),
                is_duplicate_risk_acknowledged: true,
            })
            .is_err()
        );
        assert!(
            abandon_uncertain_for_attempt(AbandonInput {
                store: &mut state.store,
                request: replacement,
                operator: &current_operator(&request),
                attempt_ref: &attempt.attempt_ref,
                is_unknown_outcome_acknowledged: true,
            })
            .is_err()
        );
    }
    let stored_attempt = state.store.read_attempt(&attempt.attempt_ref).expect("read").expect("attempt");
    assert_eq!(stored_attempt.state, WorldReleaseState::Uncertain);
}

// r[verify molten.world_promotion.reconciliation]
#[test]
fn current_operator_cannot_deny_inflight_or_uncertain_effects() {
    let request = promotion_request();
    let paths: &[&[WorldReleaseState]] = &[
        &[],
        &[WorldReleaseState::Attempting],
        &[WorldReleaseState::Attempting, WorldReleaseState::Uncertain],
        &[WorldReleaseState::Attempting, WorldReleaseState::Observed],
        &[WorldReleaseState::Attempting, WorldReleaseState::Observed, WorldReleaseState::Acknowledged],
        &[WorldReleaseState::Attempting, WorldReleaseState::Uncertain, WorldReleaseState::Reconciled],
        &[WorldReleaseState::Attempting, WorldReleaseState::Uncertain, WorldReleaseState::Abandoned],
        &[WorldReleaseState::Attempting, WorldReleaseState::Conflict],
    ];
    for path in paths {
        let (mut state, reservation, attempt) = promoted_with_uncertain_attempt(&request);
        let mut existing = state.store.claim_reservation(&reservation.reservation_ref)
            .expect("claim").expect("committed reservation");
        for next in *path {
            existing.state = *next;
            state.store.update_reservation(&existing).expect("valid effect state transition");
        }
        assert!(deny_reservation(DenyInput {
            store: &mut state.store,
            request: &request,
            operator: &current_operator(&request),
            reservation_ref: &reservation.reservation_ref,
        }).is_err());
        assert_eq!(state.store.read_reservation(&reservation.reservation_ref).expect("read"), Some(existing));
        assert_eq!(state.store.read_attempt(&attempt.attempt_ref).expect("read"), Some(attempt.clone()));
    }
}
