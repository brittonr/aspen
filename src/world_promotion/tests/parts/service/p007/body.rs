// r[verify molten.world_commit.promotion_atomic_dispatch_and_readback]
#[test]
fn reservation_and_attempt_transition_commits_together_or_not_at_all() {
    let request = promotion_request();
    let mut state = test_state(&request);
    let outcome = promote(&mut state, &request);
    let reservation_ref = outcome.committed_reservations[0].reservation_ref.clone();
    let mut claimed = state.store.claim_reservation(&reservation_ref).expect("claim").expect("claimed");
    let attempt_ref = expected_initial_attempt(&claimed);
    // A durable attempt row that the `attempting` attempt write cannot legally replace.
    let settled = WorldAttemptRecord {
        reservation_ref: reservation_ref.clone(),
        attempt_ref: attempt_ref.clone(),
        state: WorldReleaseState::Acknowledged,
        observation_ref: Some(WorldReleaseObservationRef::new(reference("settled")).expect("observation")),
        outcome: WorldAttemptOutcome::Succeeded,
        external_completion_proven: true,
    };
    state.store.store_attempt(&settled).expect("settled attempt");
    claimed.state = WorldReleaseState::Attempting;
    let attempting = WorldAttemptRecord {
        reservation_ref: reservation_ref.clone(),
        attempt_ref: attempt_ref.clone(),
        state: WorldReleaseState::Attempting,
        observation_ref: None,
        outcome: WorldAttemptOutcome::Unknown,
        external_completion_proven: false,
    };
    assert!(state.store.record_attempt_transition(&claimed, &attempting).is_err());
    // The reservation write staged before the failing attempt write was rolled back with it.
    let stored = state.store.read_reservation(&reservation_ref).expect("read").expect("reservation");
    assert_eq!(stored.state, WorldReleaseState::Claimed);
    assert_eq!(state.store.read_attempt(&attempt_ref).expect("read"), Some(settled));
}

// r[verify molten.world_commit.promotion_atomic_dispatch_and_readback]
#[test]
fn legacy_crash_between_reservation_and_attempt_writes_is_recoverable() {
    let request = promotion_request();

    // The reservation became `attempting` but its attempt row was never written.
    let mut state = test_state(&request);
    let outcome = promote(&mut state, &request);
    let reservation_ref = outcome.committed_reservations[0].reservation_ref.clone();
    let mut reservation = state.store.claim_reservation(&reservation_ref).expect("claim").expect("claimed");
    reservation.state = WorldReleaseState::Attempting;
    state.store.update_reservation(&reservation).expect("attempting reservation only");
    let attempt_ref = expected_initial_attempt(&reservation);
    assert_eq!(state.store.read_attempt(&attempt_ref).expect("read"), None);
    let mut recovery_receipts = receipts();
    let recovered = recover_interrupted_attempts(&mut state.store, &mut recovery_receipts).expect("recovery");
    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered[0].attempt_ref, attempt_ref);
    assert_eq!(recovered[0].state, WorldReleaseState::Uncertain);
    assert_eq!(recovery_receipts.count, 1);
    let stored = state.store.read_reservation(&reservation_ref).expect("read").expect("reservation");
    assert_eq!(stored.state, WorldReleaseState::Uncertain);
    assert!(recover_interrupted_attempts(&mut state.store, &mut receipts()).expect("idle").is_empty());
    let abandoned = abandon_uncertain_for_attempt(AbandonInput {
        store: &mut state.store,
        request: &request,
        operator: &abandon_grant(&request, &recovered[0], "legacy-abandon"),
        attempt_ref: &attempt_ref,
        is_unknown_outcome_acknowledged: true,
        observed_unix_seconds: OBSERVED_UNIX_SECONDS,
    })
    .expect("abandon");
    assert_eq!(abandoned.reservation.state, WorldReleaseState::Abandoned);

    // The attempt's observed result was stored but the reservation stayed `attempting`.
    let mut state = test_state(&request);
    let outcome = promote(&mut state, &request);
    let mut reservation = state
        .store
        .claim_reservation(&outcome.committed_reservations[0].reservation_ref)
        .expect("claim")
        .expect("claimed");
    let attempt_ref = expected_initial_attempt(&reservation);
    reservation.state = WorldReleaseState::Attempting;
    let mut attempt = WorldAttemptRecord {
        reservation_ref: reservation.reservation_ref.clone(),
        attempt_ref: attempt_ref.clone(),
        state: WorldReleaseState::Attempting,
        observation_ref: None,
        outcome: WorldAttemptOutcome::Unknown,
        external_completion_proven: false,
    };
    state.store.record_attempt_transition(&reservation, &attempt).expect("attempting");
    attempt.state = WorldReleaseState::Observed;
    attempt.outcome = WorldAttemptOutcome::Succeeded;
    attempt.observation_ref = Some(WorldReleaseObservationRef::new(reference("legacy-observed")).expect("observation"));
    state.store.store_attempt(&attempt).expect("observed attempt only");
    let recovered = recover_interrupted_attempts(&mut state.store, &mut receipts()).expect("recovery");
    assert_eq!(recovered, vec![attempt]);
    let stored = state.store.read_reservation(&reservation.reservation_ref).expect("read").expect("reservation");
    assert_eq!(stored.state, WorldReleaseState::Observed);
}

// r[verify molten.world_commit.promotion_atomic_dispatch_and_readback]
#[test]
fn update_reservation_rejects_immutable_field_changes() {
    let request = promotion_request();
    let mut state = test_state(&request);
    let outcome = promote(&mut state, &request);
    let committed = outcome.committed_reservations[0].clone();
    fn other(label: &str) -> String {
        reference(&format!("mutated:{label}"))
    }
    let mutations: [fn(&mut WorldReleaseReservation); 8] = [
        |reservation| reservation.promotion_ref = WorldPromotionPlanRef::new(other("plan")).expect("plan"),
        |reservation| {
            reservation.operation_ref = WorldPromotionOperationRef::new(other("operation")).expect("operation");
        },
        |reservation| reservation.candidate_head = WorldCommitRef::new(other("candidate")).expect("candidate"),
        |reservation| reservation.intent_ref = WorldEffectIntentRef::new(other("intent")).expect("intent"),
        |reservation| reservation.semantic_ref = WorldSemanticIntentRef::new(other("semantic")).expect("semantic"),
        |reservation| reservation.handler_ref = WorldPromotionHandlerRef::new(other("handler")).expect("handler"),
        |reservation| reservation.adapter_ref = WorldPromotionAdapterRef::new(other("adapter")).expect("adapter"),
        |reservation| reservation.generation += 1,
    ];
    for mutate in &mutations {
        let mut changed = committed.clone();
        changed.state = WorldReleaseState::Claimed;
        mutate(&mut changed);
        assert!(state.store.update_reservation(&changed).is_err());
        assert_eq!(state.store.read_reservation(&committed.reservation_ref).expect("read"), Some(committed.clone()));
    }
    let mut claimed = committed.clone();
    claimed.state = WorldReleaseState::Claimed;
    state.store.update_reservation(&claimed).expect("state-only transition");
}

// r[verify molten.world_commit.promotion_atomic_dispatch_and_readback]
#[test]
fn partial_promotion_persistence_reads_back_as_corrupt() {
    let request = promotion_request();
    let reservations_table = crate::world_promotion::store::RESERVATIONS_TABLE;
    let promotions_table = crate::world_promotion::store::PROMOTIONS_TABLE;

    // The successor head without every planned reservation.
    let mut state = test_state(&request);
    let outcome = promote(&mut state, &request);
    assert_eq!(state.store.read_back_promotion(&outcome.plan).expect("read back"), WorldPromotionReadBackObservation::Reservation);
    remove_row(&state.store, reservations_table, outcome.plan.reservations[0].reservation_ref.as_str());
    assert_eq!(state.store.read_back_promotion(&outcome.plan).expect("read back"), WorldPromotionReadBackObservation::Corrupt);

    // The successor head and reservations without the promotion record.
    let mut state = test_state(&request);
    let outcome = promote(&mut state, &request);
    remove_row(&state.store, promotions_table, outcome.plan.plan_ref.as_str());
    assert_eq!(state.store.read_back_promotion(&outcome.plan).expect("read back"), WorldPromotionReadBackObservation::Corrupt);

    // Reservations and record without the successor head.
    let mut state = test_state(&request);
    let outcome = promote(&mut state, &request);
    force_head(&state.store, &outcome.plan.before);
    assert_eq!(state.store.read_back_promotion(&outcome.plan).expect("read back"), WorldPromotionReadBackObservation::Corrupt);

    // Nothing written: the prior head is reported.
    let state = test_state(&request);
    let plan = plan_world_promotion(&request).expect("plan");
    assert_eq!(
        state.store.read_back_promotion(&plan).expect("read back"),
        WorldPromotionReadBackObservation::Prior {
            head: plan.before.head.clone(),
            generation: plan.before.generation,
        }
    );
}

// r[verify molten.world_commit.promotion_atomic_dispatch_and_readback]
// r[verify molten.promotion_identity.restart_binding]
#[test]
fn idempotent_replay_survives_reservations_leaving_their_initial_state() {
    let request = promotion_request();
    let (mut state, reservation, _attempt) = promoted_with_uncertain_attempt(&request);
    assert_eq!(reservation.state, WorldReleaseState::Uncertain);
    let plan = plan_world_promotion(&request).expect("plan");
    assert_eq!(state.store.read_back_promotion(&plan).expect("read back"), WorldPromotionReadBackObservation::Reservation);
    let replay = promote(&mut state, &request);
    assert!(replay.replayed);
    assert!(replay.persistence.shared.quarantine() == transactional_reconciliation_core::QuarantineStatus::Clear);
    assert!(replay.persistence.dispatch_eligible);

    // A later transition superseded the promoted head; the promotion still reads back as applied.
    let mut advanced = plan.after.clone();
    advanced.generation += 1;
    advanced.head = WorldCommitRef::new(reference("superseding-head")).expect("head");
    force_head(&state.store, &advanced);
    assert_eq!(state.store.read_back_promotion(&plan).expect("read back"), WorldPromotionReadBackObservation::Reservation);
    let replay = promote(&mut state, &request);
    assert!(replay.replayed);
    assert!(replay.persistence.shared.quarantine() == transactional_reconciliation_core::QuarantineStatus::Clear);
}

// r[verify molten.world_commit.promotion_atomic_dispatch_and_readback]
#[test]
fn legacy_abandoned_attempts_settle_their_reservations_on_recovery() {
    let request = promotion_request();

    // A pre-change abandon stored the abandoned attempt and left its reservation `uncertain`.
    let (mut state, reservation, mut attempt) = promoted_with_uncertain_attempt(&request);
    attempt.state = WorldReleaseState::Abandoned;
    state.store.store_attempt(&attempt).expect("abandoned attempt only");
    let mut recovery_receipts = receipts();
    let recovered = recover_interrupted_attempts(&mut state.store, &mut recovery_receipts).expect("recovery");
    assert_eq!(recovered, vec![attempt.clone()]);
    assert_eq!(recovery_receipts.count, 1);
    let stored = state.store.read_reservation(&reservation.reservation_ref).expect("read").expect("reservation");
    assert_eq!(stored.state, WorldReleaseState::Abandoned);
    assert!(recover_interrupted_attempts(&mut state.store, &mut receipts()).expect("idle").is_empty());

    // A pre-change store left the reservation `attempting` while its initial attempt was abandoned.
    let mut state = test_state(&request);
    let outcome = promote(&mut state, &request);
    let mut reservation = state
        .store
        .claim_reservation(&outcome.committed_reservations[0].reservation_ref)
        .expect("claim")
        .expect("claimed");
    reservation.state = WorldReleaseState::Attempting;
    state.store.update_reservation(&reservation).expect("attempting reservation");
    let abandoned = WorldAttemptRecord {
        reservation_ref: reservation.reservation_ref.clone(),
        attempt_ref: expected_initial_attempt(&reservation),
        state: WorldReleaseState::Abandoned,
        observation_ref: None,
        outcome: WorldAttemptOutcome::Unknown,
        external_completion_proven: false,
    };
    state.store.store_attempt(&abandoned).expect("abandoned attempt row");
    let recovered = recover_interrupted_attempts(&mut state.store, &mut receipts()).expect("recovery");
    assert_eq!(recovered, vec![abandoned.clone()]);
    let stored = state.store.read_reservation(&reservation.reservation_ref).expect("read").expect("reservation");
    assert_eq!(stored.state, WorldReleaseState::Abandoned);
    assert_eq!(state.store.read_attempt(&abandoned.attempt_ref).expect("read"), Some(abandoned));
}
