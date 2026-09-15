use molten_core::world_promotion::*;

use super::CanonicalWorldPromotionRecord;
use crate::error::Result;

pub trait WorldPromotionCurrentPort {
    fn observe_transaction(&mut self, plan: &WorldPromotionPlan) -> Result<WorldPromotionTransactionFacts>;
}

/// The commit half of the promotion transaction port.
pub trait WorldPromotionCommitPort {
    fn commit_promotion(
        &mut self,
        plan: &WorldPromotionPlan,
        canonical_plan: &CanonicalWorldPromotionRecord,
        reservations: &[CanonicalWorldPromotionRecord],
        facts: &WorldPromotionTransactionFacts,
    ) -> Result<WorldPromotionCommitObservation>;

    fn read_back_promotion(&self, plan: &WorldPromotionPlan) -> Result<WorldPromotionReadBackObservation>;
}

/// The reservation and attempt bookkeeping half of the promotion transaction
/// port.
pub trait WorldPromotionBookkeepingPort {
    fn read_reservation(&self, reservation_ref: &WorldReleaseReservationRef)
    -> Result<Option<WorldReleaseReservation>>;

    fn list_reservations(&self) -> Result<Vec<WorldReleaseReservation>>;

    fn claim_reservation(
        &mut self,
        reservation_ref: &WorldReleaseReservationRef,
    ) -> Result<Option<WorldReleaseReservation>>;

    fn update_reservation(&mut self, reservation: &WorldReleaseReservation) -> Result<()>;

    fn store_attempt(&mut self, attempt: &WorldAttemptRecord) -> Result<()>;

    fn read_attempt(&self, attempt_ref: &WorldReleaseAttemptRef) -> Result<Option<WorldAttemptRecord>>;
}

/// Every transaction capability a promotion adapter must provide.
pub trait WorldPromotionTransactionPort: WorldPromotionCommitPort + WorldPromotionBookkeepingPort {}

impl<T> WorldPromotionTransactionPort for T where T: WorldPromotionCommitPort + WorldPromotionBookkeepingPort {}

pub trait WorldEffectAdmissionPort {
    fn observe_dispatch(
        &mut self,
        plan: &WorldPromotionPlan,
        reservation: &WorldReleaseReservation,
    ) -> Result<WorldDispatchFacts>;
}

pub trait WorldEffectDispatcherPort {
    fn dispatch(&mut self, plan: &WorldDispatchPlan) -> Result<WorldAttemptObservation>;
}

pub trait WorldPromotionReceiptPort {
    fn publish_promotion_receipt(&mut self, receipt: &CanonicalWorldPromotionRecord) -> Result<()>;
}
