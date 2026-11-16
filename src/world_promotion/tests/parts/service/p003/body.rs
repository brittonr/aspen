// r[verify molten.world_promotion.dispatch]
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
        outcome: WorldAttemptOutcome::Unknown,
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
        outcome: WorldAttemptOutcome::Unknown,
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
    struct FailingDispatcher;
    impl WorldEffectDispatcherPort for FailingDispatcher {
        fn dispatch(&mut self, _plan: &WorldDispatchPlan) -> crate::error::Result<WorldAttemptObservation> {
            Err(crate::error::Failure::invalid_harness("dispatcher boom"))
        }
    }
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
    let stored_reservation = state.store.read_reservation(&reservation.reservation_ref).expect("reservation read").expect("reservation");
    assert_eq!(stored_reservation.state, WorldReleaseState::Uncertain);

    // retry/abandon paths are reachable from the uncertain state
    let next = WorldReleaseAttemptRef::new(reference("retry-after-error")).expect("next");
    let retry = plan_retry_for_reservation(RetryPlanInput {
        store: &state.store,
        request: &request,
        reservation_ref: &reservation.reservation_ref,
        attempt_ref: &stored_attempt.attempt_ref,
        next_attempt_ref: next.clone(),
        is_duplicate_risk_acknowledged: true,
    })
    .expect("retry plan");
    assert_eq!(retry.next_attempt_ref, next);

    let abandoned = abandon_uncertain_for_attempt(AbandonInput {
        store: &mut state.store,
        request: &request,
        attempt_ref: &stored_attempt.attempt_ref,
        is_unknown_outcome_acknowledged: true,
    })
    .expect("abandon");
    assert_eq!(abandoned.state, WorldReleaseState::Abandoned);
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

    struct FailingStore {
        inner: LocalWorldPromotionStore,
    }
    impl WorldPromotionCommitPort for FailingStore {
        fn commit_promotion(
            &mut self,
            plan: &WorldPromotionPlan,
            canonical_plan: &CanonicalWorldPromotionRecord,
            reservations: &[CanonicalWorldPromotionRecord],
            facts: &WorldPromotionTransactionFacts,
        ) -> crate::error::Result<WorldPromotionCommitObservation> {
            self.inner.commit_promotion(plan, canonical_plan, reservations, facts)
        }
        fn read_back_promotion(&self, plan: &WorldPromotionPlan) -> crate::error::Result<WorldPromotionReadBackObservation> {
            self.inner.read_back_promotion(plan)
        }
    }
    impl WorldPromotionBookkeepingPort for FailingStore {
        fn read_reservation(&self, reservation_ref: &WorldReleaseReservationRef) -> crate::error::Result<Option<WorldReleaseReservation>> {
            self.inner.read_reservation(reservation_ref)
        }
        fn list_reservations(&self) -> crate::error::Result<Vec<WorldReleaseReservation>> {
            self.inner.list_reservations()
        }
        fn claim_reservation(&mut self, reservation_ref: &WorldReleaseReservationRef) -> crate::error::Result<Option<WorldReleaseReservation>> {
            self.inner.claim_reservation(reservation_ref)
        }
        fn update_reservation(&mut self, reservation: &WorldReleaseReservation) -> crate::error::Result<()> {
            self.inner.update_reservation(reservation)
        }
        fn store_attempt(&mut self, attempt: &WorldAttemptRecord) -> crate::error::Result<()> {
            // Admit the pre-dispatch `attempting` write; fail only the post-error uncertain record.
            if attempt.state == WorldReleaseState::Uncertain {
                return Err(crate::error::Failure::invalid_harness("store boom"));
            }
            self.inner.store_attempt(attempt)
        }
        fn read_attempt(&self, attempt_ref: &WorldReleaseAttemptRef) -> crate::error::Result<Option<WorldAttemptRecord>> {
            self.inner.read_attempt(attempt_ref)
        }
        fn list_attempts_in_progress(&self) -> crate::error::Result<Vec<WorldAttemptRecord>> {
            self.inner.list_attempts_in_progress()
        }
        fn read_promotion_identity(&self, operation_ref: &WorldPromotionOperationRef) -> crate::error::Result<Option<StoredPromotionIdentity>> {
            self.inner.read_promotion_identity(operation_ref)
        }
    }

    // move store out
    let mut failing_store = FailingStore { inner: state.store };
    let mut admission = Admission { facts: dispatch_facts() };
    struct FailingDispatcher;
    impl WorldEffectDispatcherPort for FailingDispatcher {
        fn dispatch(&mut self, _plan: &WorldDispatchPlan) -> crate::error::Result<WorldAttemptObservation> {
            Err(crate::error::Failure::invalid_harness("dispatcher boom"))
        }
    }
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
    state.store.update_reservation(&attempting).expect("attempting reservation");
    state
        .store
        .store_attempt(&WorldAttemptRecord {
            reservation_ref: reservation_ref.clone(),
            attempt_ref: attempt_ref.clone(),
            state: WorldReleaseState::Attempting,
            observation_ref: None,
            outcome: WorldAttemptOutcome::Unknown,
            external_completion_proven: false,
        })
        .expect("attempting attempt");

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
        attempt_ref: &attempt_ref,
        is_unknown_outcome_acknowledged: true,
    })
    .expect("abandon");
    assert_eq!(abandoned.state, WorldReleaseState::Abandoned);
}
