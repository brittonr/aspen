// r[verify molten.world_promotion.dispatch]
#[test]
fn retry_plan_rebuilds_the_original_plan_and_requires_duplicate_risk_acknowledgement() {
    let request = promotion_request();
    let (mut state, reservation, attempt) = promoted_with_uncertain_attempt(&request);
    let grant = retry_grant(&request, &attempt, "retry-one");
    // A refused plan (no acknowledgement) does not consume the grant.
    assert!(
        plan_retry_for_reservation(RetryPlanInput {
            store: &mut state.store,
            request: &request,
            operator: &grant,
            reservation_ref: &reservation.reservation_ref,
            attempt_ref: &attempt.attempt_ref,
            next_attempt_ref: WorldReleaseAttemptRef::new(reference("retry-attempt-three")).expect("next attempt"),
            is_duplicate_risk_acknowledged: false,
            observed_unix_seconds: OBSERVED_UNIX_SECONDS,
        })
        .is_err()
    );
    let next = WorldReleaseAttemptRef::new(reference("retry-attempt-two")).expect("next attempt");
    let plan = plan_retry_for_reservation(RetryPlanInput {
        store: &mut state.store,
        request: &request,
        operator: &grant,
        reservation_ref: &reservation.reservation_ref,
        attempt_ref: &attempt.attempt_ref,
        next_attempt_ref: next.clone(),
        is_duplicate_risk_acknowledged: true,
        observed_unix_seconds: OBSERVED_UNIX_SECONDS,
    })
    .expect("retry plan");
    assert_eq!(plan.next_attempt_ref, next);
    assert!(plan.same_logical_release);
    assert!(!plan.external_completion_proven);
    // The grant was consumed by the plan it authorized.
    assert!(
        plan_retry_for_reservation(RetryPlanInput {
            store: &mut state.store,
            request: &request,
            operator: &grant,
            reservation_ref: &reservation.reservation_ref,
            attempt_ref: &attempt.attempt_ref,
            next_attempt_ref: next,
            is_duplicate_risk_acknowledged: true,
            observed_unix_seconds: OBSERVED_UNIX_SECONDS,
        })
        .is_err()
    );
}

// r[verify molten.world_promotion.dispatch]
// r[verify molten.world_commit.promotion_atomic_dispatch_and_readback]
#[test]
fn deny_transitions_a_committed_reservation_and_abandon_moves_attempt_and_reservation() {
    let request = promotion_request();
    let mut state = test_state(&request);
    promote_world(&request, WorldPromotionPorts {
        current: &mut Current { is_admitted: true },
        transaction: &mut state.store,
        receipts: &mut receipts(),
    })
    .expect("promotion");
    let reservation = state.store.list_reservations().expect("reservations").pop().expect("reservation");
    let denied = deny_reservation(DenyInput {
        store: &mut state.store,
        request: &request,
        operator: &deny_grant(&request, &reservation.reservation_ref, "deny"),
        reservation_ref: &reservation.reservation_ref,
        observed_unix_seconds: OBSERVED_UNIX_SECONDS,
    })
    .expect("deny");
    assert_eq!(denied.state, WorldReleaseState::Denied);
    let stored = state.store.read_reservation(&reservation.reservation_ref).expect("read").expect("reservation");
    assert_eq!(stored.state, WorldReleaseState::Denied);

    let (mut state, reservation, attempt) = promoted_with_uncertain_attempt(&request);
    let grant = abandon_grant(&request, &attempt, "abandon");
    let abandoned = abandon_uncertain_for_attempt(AbandonInput {
        store: &mut state.store,
        request: &request,
        operator: &grant,
        attempt_ref: &attempt.attempt_ref,
        is_unknown_outcome_acknowledged: true,
        observed_unix_seconds: OBSERVED_UNIX_SECONDS,
    })
    .expect("abandon");
    assert_eq!(abandoned.attempt.state, WorldReleaseState::Abandoned);
    assert_eq!(abandoned.reservation.state, WorldReleaseState::Abandoned);
    let stored = state.store.read_attempt(&attempt.attempt_ref).expect("read").expect("attempt");
    assert_eq!(stored.state, WorldReleaseState::Abandoned);
    // The reservation left `Uncertain` in the same transaction.
    let stored = state.store.read_reservation(&reservation.reservation_ref).expect("read").expect("reservation");
    assert_eq!(stored.state, WorldReleaseState::Abandoned);
    assert!(
        abandon_uncertain_for_attempt(AbandonInput {
            store: &mut state.store,
            request: &request,
            operator: &abandon_grant(&request, &attempt, "abandon-again"),
            attempt_ref: &attempt.attempt_ref,
            is_unknown_outcome_acknowledged: true,
            observed_unix_seconds: OBSERVED_UNIX_SECONDS,
        })
        .is_err()
    );
}

// r[verify molten.world_promotion.dispatch]
#[test]
fn dispatcher_error_records_uncertain_attempt_and_reservation() {
    let request = promotion_request();
    let mut state = test_state(&request);
    let mut current = Current { is_admitted: true };
    let mut promotion_receipts = receipts();
    let promoted = promote_world(&request, WorldPromotionPorts {
        current: &mut current,
        transaction: &mut state.store,
        receipts: &mut promotion_receipts,
    })
    .expect("promotion");
    let reservation = promoted.committed_reservations[0].clone();
    let attempt_ref = expected_initial_attempt(&reservation);
    let mut admission = Admission { facts: dispatch_facts() };
    let mut dispatcher = FailingDispatcher;
    let mut dispatch_receipts = receipts();
    let result = dispatch_world_reservation(&promoted.plan, &reservation.reservation_ref, &attempt_ref, WorldDispatchPorts {
        transaction: &mut state.store,
        admission: &mut admission,
        dispatcher: &mut dispatcher,
        receipts: &mut dispatch_receipts,
    });
    assert!(result.is_err());
    let stored_attempt = state.store.read_attempt(&attempt_ref).expect("attempt read").expect("attempt");
    assert_eq!(stored_attempt.state, WorldReleaseState::Uncertain);
    let stored_reservation =
        state.store.read_reservation(&reservation.reservation_ref).expect("reservation read").expect("reservation");
    assert_eq!(stored_reservation.state, WorldReleaseState::Uncertain);

    // retry/abandon paths are reachable from the uncertain state
    let next = WorldReleaseAttemptRef::new(reference("retry-after-error")).expect("next");
    let retry = plan_retry_for_reservation(RetryPlanInput {
        store: &mut state.store,
        request: &request,
        operator: &retry_grant(&request, &stored_attempt, "retry-after-error"),
        reservation_ref: &reservation.reservation_ref,
        attempt_ref: &stored_attempt.attempt_ref,
        next_attempt_ref: next.clone(),
        is_duplicate_risk_acknowledged: true,
        observed_unix_seconds: OBSERVED_UNIX_SECONDS,
    })
    .expect("retry plan");
    assert_eq!(retry.next_attempt_ref, next);

    let abandoned = abandon_uncertain_for_attempt(AbandonInput {
        store: &mut state.store,
        request: &request,
        operator: &abandon_grant(&request, &stored_attempt, "abandon-after-error"),
        attempt_ref: &stored_attempt.attempt_ref,
        is_unknown_outcome_acknowledged: true,
        observed_unix_seconds: OBSERVED_UNIX_SECONDS,
    })
    .expect("abandon");
    assert_eq!(abandoned.attempt.state, WorldReleaseState::Abandoned);
    assert_eq!(abandoned.reservation.state, WorldReleaseState::Abandoned);
}

#[test]
fn dispatcher_error_with_recording_failure_surfaces_both_errors() {
    let request = promotion_request();
    let mut state = test_state(&request);
    let mut current = Current { is_admitted: true };
    let mut promotion_receipts = receipts();
    let promoted = promote_world(&request, WorldPromotionPorts {
        current: &mut current,
        transaction: &mut state.store,
        receipts: &mut promotion_receipts,
    })
    .expect("promotion");
    let reservation = promoted.committed_reservations[0].clone();
    let attempt_ref = expected_initial_attempt(&reservation);

    let mut failing_store = FailingStore { inner: state.store };
    let mut admission = Admission { facts: dispatch_facts() };
    let mut dispatcher = FailingDispatcher;
    let mut dispatch_receipts = receipts();
    let result = dispatch_world_reservation(&promoted.plan, &reservation.reservation_ref, &attempt_ref, WorldDispatchPorts {
        transaction: &mut failing_store,
        admission: &mut admission,
        dispatcher: &mut dispatcher,
        receipts: &mut dispatch_receipts,
    });
    let err = result.expect_err("expected error");
    let message = format!("{err}");
    assert!(message.contains("dispatcher boom"));
    assert!(message.contains("store boom"));
    state.store = failing_store.inner;
    // The failed uncertain write left both records `attempting` together, which recovery settles.
    let stored = state.store.read_reservation(&reservation.reservation_ref).expect("read").expect("reservation");
    assert_eq!(stored.state, WorldReleaseState::Attempting);
    let attempt = state.store.read_attempt(&attempt_ref).expect("read").expect("attempt");
    assert_eq!(attempt.state, WorldReleaseState::Attempting);
}

// r[verify molten.world_promotion.reconciliation]
#[test]
fn recovery_classifies_attempts_interrupted_before_their_result_was_recorded() {
    let request = promotion_request();
    let mut state = test_state(&request);
    let mut current = Current { is_admitted: true };
    let mut promotion_receipts = receipts();
    let promoted = promote_world(&request, WorldPromotionPorts {
        current: &mut current,
        transaction: &mut state.store,
        receipts: &mut promotion_receipts,
    })
    .expect("promotion");
    let reservation_ref = promoted.committed_reservations[0].reservation_ref.clone();
    let attempt_ref = expected_initial_attempt(&promoted.committed_reservations[0]);
    // A previous process claimed the reservation and stored the attempt, then stopped before the
    // dispatcher's result was recorded.
    let mut attempting = state.store.claim_reservation(&reservation_ref).expect("claim").expect("reservation");
    attempting.state = WorldReleaseState::Attempting;
    let attempting_attempt = WorldAttemptRecord {
        reservation_ref: reservation_ref.clone(),
        attempt_ref: attempt_ref.clone(),
        state: WorldReleaseState::Attempting,
        observation_ref: None,
        outcome: WorldAttemptOutcome::Unknown,
        external_completion_proven: false,
    };
    state.store.record_attempt_transition(&attempting, &attempting_attempt).expect("attempting records");

    let mut recovery_receipts = receipts();
    let recovered = recover_interrupted_attempts(&mut state.store, &mut recovery_receipts).expect("recovery");
    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered[0].attempt_ref, attempt_ref);
    assert_eq!(recovered[0].outcome, WorldAttemptOutcome::Unknown);
    assert_eq!(recovery_receipts.count, 1);
    let stored = state.store.read_attempt(&attempt_ref).expect("attempt read").expect("attempt");
    assert_eq!(stored.state, WorldReleaseState::Uncertain);
    let reservation = state.store.read_reservation(&reservation_ref).expect("reservation read").expect("reservation");
    assert_eq!(reservation.state, WorldReleaseState::Uncertain);

    // Nothing is in progress any more, and the uncertain attempt can now be abandoned.
    assert!(recover_interrupted_attempts(&mut state.store, &mut recovery_receipts).expect("second recovery").is_empty());
    assert_eq!(recovery_receipts.count, 1);
    let abandoned = abandon_uncertain_for_attempt(AbandonInput {
        store: &mut state.store,
        request: &request,
        operator: &abandon_grant(&request, &stored, "recovered-abandon"),
        attempt_ref: &attempt_ref,
        is_unknown_outcome_acknowledged: true,
        observed_unix_seconds: OBSERVED_UNIX_SECONDS,
    })
    .expect("abandon");
    assert_eq!(abandoned.attempt.state, WorldReleaseState::Abandoned);
    assert_eq!(abandoned.reservation.state, WorldReleaseState::Abandoned);
}
