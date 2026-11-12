// r[verify molten.world_promotion.retry]
#[test]
fn retry_plan_rebuilds_the_original_plan_and_requires_duplicate_risk_acknowledgement() {
    let request = promotion_request();
    let mut state = test_state(&request);
    promote_world(&request, WorldPromotionPorts {
        current: &mut Current { is_admitted: true },
        transaction: &mut state.store,
        receipts: &mut receipts(),
    })
    .expect("promotion");
    let reservation = state.store.list_reservations().expect("reservations").pop().expect("reservation");
    let attempt = WorldAttemptRecord {
        reservation_ref: reservation.reservation_ref.clone(),
        attempt_ref: WorldReleaseAttemptRef::new(reference("retry-attempt-one")).expect("attempt"),
        state: WorldReleaseState::Uncertain,
        observation_ref: None,
        external_completion_proven: false,
    };
    state.store.store_attempt(&attempt).expect("store attempt");
    let next = WorldReleaseAttemptRef::new(reference("retry-attempt-two")).expect("next attempt");
    let plan = plan_retry_for_reservation(RetryPlanInput {
        store: &state.store,
        request: &request,
        reservation_ref: &reservation.reservation_ref,
        attempt_ref: &attempt.attempt_ref,
        next_attempt_ref: next.clone(),
        is_duplicate_risk_acknowledged: true,
    })
    .expect("retry plan");
    assert_eq!(plan.next_attempt_ref, next);
    assert!(plan.same_logical_release);
    assert!(!plan.external_completion_proven);
    assert!(
        plan_retry_for_reservation(RetryPlanInput {
            store: &state.store,
            request: &request,
            reservation_ref: &reservation.reservation_ref,
            attempt_ref: &attempt.attempt_ref,
            next_attempt_ref: WorldReleaseAttemptRef::new(reference("retry-attempt-three"))
                .expect("next attempt"),
            is_duplicate_risk_acknowledged: false,
        })
        .is_err()
    );
}

// r[verify molten.world_promotion.dispatch]
#[test]
fn deny_transitions_a_committed_reservation_and_abandon_updates_the_attempt() {
    let request = promotion_request();
    let mut state = test_state(&request);
    promote_world(&request, WorldPromotionPorts {
        current: &mut Current { is_admitted: true },
        transaction: &mut state.store,
        receipts: &mut receipts(),
    })
    .expect("promotion");
    let reservation = state.store.list_reservations().expect("reservations").pop().expect("reservation");
    let denied = denied_reservation(&reservation);
    state.store.update_reservation(&denied).expect("deny");
    let stored = state
        .store
        .read_reservation(&reservation.reservation_ref)
        .expect("read")
        .expect("reservation");
    assert_eq!(stored.state, WorldReleaseState::Denied);

    let attempt = WorldAttemptRecord {
        reservation_ref: reservation.reservation_ref.clone(),
        attempt_ref: WorldReleaseAttemptRef::new(reference("abandon-attempt")).expect("attempt"),
        state: WorldReleaseState::Uncertain,
        observation_ref: None,
        external_completion_proven: false,
    };
    state.store.store_attempt(&attempt).expect("store attempt");
    let abandoned = abandon_uncertain_for_attempt(AbandonInput {
        store: &mut state.store,
        request: &request,
        attempt_ref: &attempt.attempt_ref,
        is_unknown_outcome_acknowledged: true,
    })
    .expect("abandon");
    assert_eq!(abandoned.state, WorldReleaseState::Abandoned);
    let stored = state
        .store
        .read_attempt(&attempt.attempt_ref)
        .expect("read")
        .expect("attempt");
    assert_eq!(stored.state, WorldReleaseState::Abandoned);
    assert!(
        abandon_uncertain_for_attempt(AbandonInput {
            store: &mut state.store,
            request: &request,
            attempt_ref: &attempt.attempt_ref,
            is_unknown_outcome_acknowledged: true,
        })
        .is_err()
    );
}
