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
        fn read_promotion_identity(&self, operation_ref: &WorldPromotionOperationRef) -> crate::error::Result<Option<StoredPromotionIdentity>> {
            self.inner.read_promotion_identity(operation_ref)
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
    }

    struct FailingDispatcher;
    impl WorldEffectDispatcherPort for FailingDispatcher {
        fn dispatch(&mut self, _plan: &WorldDispatchPlan) -> crate::error::Result<WorldAttemptObservation> {
            Err(crate::error::Failure::invalid_harness("dispatcher boom"))
        }
    }
