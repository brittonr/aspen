use transactional_reconciliation_core::AttemptReservation;
use transactional_reconciliation_core::PersistenceDecision;

use super::reference::*;
use crate::world_commit::WorldCommitRef;
use crate::world_head::WorldHeadPolicyRef;
use crate::world_head::WorldHeadState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldReleaseState {
    Planned,
    Committed,
    Claimed,
    Attempting,
    Blocked,
    Observed,
    Acknowledged,
    Uncertain,
    Conflict,
    Denied,
    Reconciled,
    Abandoned,
}

impl WorldReleaseState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Committed => "committed",
            Self::Claimed => "claimed",
            Self::Attempting => "attempting",
            Self::Blocked => "blocked",
            Self::Observed => "observed",
            Self::Acknowledged => "acknowledged",
            Self::Uncertain => "uncertain",
            Self::Conflict => "conflict",
            Self::Denied => "denied",
            Self::Reconciled => "reconciled",
            Self::Abandoned => "abandoned",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "planned" => Some(Self::Planned),
            "committed" => Some(Self::Committed),
            "claimed" => Some(Self::Claimed),
            "attempting" => Some(Self::Attempting),
            "blocked" => Some(Self::Blocked),
            "observed" => Some(Self::Observed),
            "acknowledged" => Some(Self::Acknowledged),
            "uncertain" => Some(Self::Uncertain),
            "conflict" => Some(Self::Conflict),
            "denied" => Some(Self::Denied),
            "reconciled" => Some(Self::Reconciled),
            "abandoned" => Some(Self::Abandoned),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldPromotionTransactionFacts {
    pub observed_head: Option<WorldHeadState>,
    pub authority_ref: WorldPromotionAuthorityRef,
    pub authority_admitted: bool,
    pub authority_generation: u64,
    pub policy_ref: WorldHeadPolicyRef,
    pub intent_closure_complete: bool,
    pub reservation_refs: Vec<WorldReleaseReservationRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldDispatchFacts {
    pub observed_generation: u64,
    pub authority_admitted: bool,
    pub policy_admitted: bool,
    pub capability_admitted: bool,
    pub handler_matches: bool,
    pub adapter_matches: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldDispatchPlan {
    pub reservation_ref: WorldReleaseReservationRef,
    pub attempt_ref: WorldReleaseAttemptRef,
    pub operation_ref: WorldPromotionOperationRef,
    pub intent_ref: WorldEffectIntentRef,
    pub idempotency_ref: WorldReleaseReservationRef,
    pub dispatch_authorized: bool,
    pub non_claims: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldAttemptObservation {
    Succeeded(WorldReleaseObservationRef),
    Failed(WorldReleaseObservationRef),
    Unknown,
    Conflict(WorldReleaseObservationRef),
    Duplicate(WorldReleaseObservationRef),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldAttemptRecord {
    pub reservation_ref: WorldReleaseReservationRef,
    pub attempt_ref: WorldReleaseAttemptRef,
    pub state: WorldReleaseState,
    pub observation_ref: Option<WorldReleaseObservationRef>,
    pub external_completion_proven: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorldRetryPlan {
    pub reservation_ref: WorldReleaseReservationRef,
    pub previous_attempt_ref: WorldReleaseAttemptRef,
    pub next_attempt_ref: WorldReleaseAttemptRef,
    pub shared_attempt: AttemptReservation,
    pub duplicate_risk_acknowledged: bool,
    pub same_logical_release: bool,
    pub external_completion_proven: bool,
    pub non_claims: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldPromotionCommitObservation {
    Applied,
    NotApplied {
        current_head: WorldCommitRef,
        current_generation: u64,
    },
    OutcomeUnknown,
    RepairReported,
    Corrupt,
    Inconsistent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldPromotionReadBackObservation {
    Prior { head: WorldCommitRef, generation: u64 },
    Reservation,
    Missing,
    Corrupt,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorldPromotionPersistence {
    pub shared: PersistenceDecision,
    pub dispatch_eligible: bool,
    pub mutation_authorized_by_evidence: bool,
    pub non_claims: Vec<String>,
}
