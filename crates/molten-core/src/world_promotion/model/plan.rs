use transactional_reconciliation_core::AttemptReservation;
use transactional_reconciliation_core::ImmutablePlan;
use transactional_reconciliation_core::PersistenceBinding;
use transactional_reconciliation_core::PublicationIntent;

use super::super::MAX_WORLD_PROMOTION_INTENTS;
use super::dispatch::WorldReleaseState;
use super::reference::*;
use crate::world_commit::WorldCommitRef;
use crate::world_head::WorldBranchClass;
use crate::world_head::WorldBranchId;
use crate::world_head::WorldHeadPolicyRef;
use crate::world_head::WorldHeadState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldIntentReleaseClass {
    Release,
    Deny,
    Simulate,
    Retain,
}

impl WorldIntentReleaseClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Release => "release",
            Self::Deny => "deny",
            Self::Simulate => "simulate",
            Self::Retain => "retain",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldEffectIntent {
    pub intent_ref: WorldEffectIntentRef,
    pub semantic_ref: WorldSemanticIntentRef,
    pub handler_ref: WorldPromotionHandlerRef,
    pub adapter_ref: WorldPromotionAdapterRef,
    pub release_class: Option<WorldIntentReleaseClass>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldPromotionAuthorityObservation {
    pub authority_ref: WorldPromotionAuthorityRef,
    pub policy_ref: WorldHeadPolicyRef,
    pub observed_generation: u64,
    pub admitted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldPromotionBounds {
    pub max_intents: usize,
    pub max_reservations: usize,
}

impl WorldPromotionBounds {
    pub const fn standard() -> Self {
        Self {
            max_intents: MAX_WORLD_PROMOTION_INTENTS,
            max_reservations: MAX_WORLD_PROMOTION_INTENTS,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldPromotionRequest {
    pub operation_ref: WorldPromotionOperationRef,
    pub branch_id: WorldBranchId,
    pub branch_class: WorldBranchClass,
    pub expected_head: WorldCommitRef,
    pub candidate_head: WorldCommitRef,
    pub expected_generation: u64,
    pub policy_ref: WorldHeadPolicyRef,
    pub authority: WorldPromotionAuthorityObservation,
    pub intent_closure_complete: bool,
    pub simulation_only: bool,
    pub intents: Vec<WorldEffectIntent>,
    pub bounds: WorldPromotionBounds,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldReleaseReservation {
    pub reservation_ref: WorldReleaseReservationRef,
    pub promotion_ref: WorldPromotionPlanRef,
    pub operation_ref: WorldPromotionOperationRef,
    pub candidate_head: WorldCommitRef,
    pub intent_ref: WorldEffectIntentRef,
    pub semantic_ref: WorldSemanticIntentRef,
    pub handler_ref: WorldPromotionHandlerRef,
    pub adapter_ref: WorldPromotionAdapterRef,
    pub generation: u64,
    pub state: WorldReleaseState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionalReleaseOperation {
    pub reservation_ref: WorldReleaseReservationRef,
    pub shared_operation_identity: transactional_reconciliation_core::Identity,
    pub initial_attempt: AttemptReservation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionalPromotionPlan {
    pub shared_plan: ImmutablePlan,
    pub publication: PublicationIntent,
    pub publication_reservation: AttemptReservation,
    pub persistence_binding: PersistenceBinding,
    pub release_operations: Vec<TransactionalReleaseOperation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorldPromotionPlan {
    pub plan_ref: WorldPromotionPlanRef,
    pub operation_ref: WorldPromotionOperationRef,
    pub authority_ref: WorldPromotionAuthorityRef,
    pub before: WorldHeadState,
    pub after: WorldHeadState,
    pub intents: Vec<WorldEffectIntent>,
    pub reservations: Vec<WorldReleaseReservation>,
    pub transaction: TransactionalPromotionPlan,
    pub external_effects_completed: bool,
    pub non_claims: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldPromotionIssue {
    InvalidBounds,
    IntentLimitExceeded,
    ReservationLimitExceeded,
    DuplicateIntent(String),
    DuplicateSemanticIntent(String),
    IntentClosureIncomplete,
    IntentUnclassified(String),
    SimulationBranchDenied,
    BranchClassDenied,
    CandidateEqualsActive,
    GenerationOverflow,
    AuthorityDenied,
    AuthorityPolicyMismatch,
    AuthorityGenerationMismatch,
    ReservationSetMismatch,
    ReservationNotCommitted,
    DispatchGenerationMismatch,
    DispatchAuthorityDenied,
    DispatchPolicyDenied,
    DispatchCapabilityDenied,
    DispatchHandlerMismatch,
    DispatchAdapterMismatch,
    AttemptIdentityReused,
    RetryAcknowledgementRequired,
    TransactionalMapping(String),
}
