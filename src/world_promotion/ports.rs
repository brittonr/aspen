use molten_core::world_head::WorldBranchId;
use molten_core::world_head::WorldHeadState;
use molten_core::world_promotion::*;

use super::CanonicalWorldPromotionRecord;
use crate::error::Result;

/// One validated operator mutation. The store applies it in a single write transaction only while
/// the branch head is still `validated_head` and the grant nonce is still unconsumed, consuming the
/// nonce together with the reservation and attempt writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldOperatorMutation {
    pub grant: WorldPromotionGrant,
    pub validated_head: WorldHeadState,
    pub reservation: Option<WorldReleaseReservation>,
    pub attempt: Option<WorldAttemptRecord>,
}

/// A stored promotion operation identity. Records written before the identity
/// table existed expose only their logical operation and plan identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoredPromotionIdentity {
    Current(WorldPromotionIdentity),
    Legacy {
        operation_ref: WorldPromotionOperationRef,
        plan_ref: WorldPromotionPlanRef,
    },
}

pub trait WorldPromotionCurrentPort {
    fn observe_transaction(&mut self, plan: &WorldPromotionPlan) -> Result<WorldPromotionTransactionFacts>;
}

/// The commit half of the promotion transaction port.
pub trait WorldPromotionCommitPort {
    /// Validates `facts` against the live head and grant consumption read inside the write
    /// transaction, then publishes the head, plan, reservations, identity and consumed grant
    /// together.
    fn commit_promotion(
        &mut self,
        plan: &WorldPromotionPlan,
        canonical_plan: &CanonicalWorldPromotionRecord,
        reservations: &[CanonicalWorldPromotionRecord],
        facts: &WorldPromotionTransactionFacts,
    ) -> Result<WorldPromotionCommitObservation>;

    fn read_back_promotion(&self, plan: &WorldPromotionPlan) -> Result<WorldPromotionReadBackObservation>;
    fn read_promotion_identity(
        &self,
        operation_ref: &WorldPromotionOperationRef,
    ) -> Result<Option<StoredPromotionIdentity>>;
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

    /// Writes one reservation record after checking that its identity fields are unchanged and
    /// its state transition is allowed.
    fn update_reservation(&mut self, reservation: &WorldReleaseReservation) -> Result<()>;

    fn store_attempt(&mut self, attempt: &WorldAttemptRecord) -> Result<()>;

    fn read_attempt(&self, attempt_ref: &WorldReleaseAttemptRef) -> Result<Option<WorldAttemptRecord>>;

    /// Every stored attempt, in attempt-reference order.
    fn list_attempts(&self) -> Result<Vec<WorldAttemptRecord>>;
}

/// The atomic multi-record transitions of the promotion transaction port and the live facts they
/// recheck: each mutating method commits all of its writes in one write transaction or none.
pub trait WorldPromotionTransitionPort {
    /// Writes a reservation transition and its attempt record in one write transaction.
    fn record_attempt_transition(
        &mut self,
        reservation: &WorldReleaseReservation,
        attempt: &WorldAttemptRecord,
    ) -> Result<()>;

    /// The live head of `branch_id` and whether `nonce` is already consumed.
    fn observe_live(
        &self,
        branch_id: &WorldBranchId,
        nonce: &WorldPromotionGrantNonce,
    ) -> Result<WorldPromotionLiveFacts>;

    fn apply_operator_mutation(&mut self, mutation: &WorldOperatorMutation) -> Result<()>;
}

/// Every transaction capability a promotion adapter must provide.
pub trait WorldPromotionTransactionPort:
    WorldPromotionCommitPort + WorldPromotionBookkeepingPort + WorldPromotionTransitionPort
{
}

impl<T> WorldPromotionTransactionPort for T where T: WorldPromotionCommitPort + WorldPromotionBookkeepingPort + WorldPromotionTransitionPort
{}

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
