use artifact_binding_core::RetirementClassification;
use artifact_binding_core::RetirementDecision;

use super::super::MAX_WORLD_RETENTION_CLASSES;
use super::super::WorldObjectRef;
use super::closure::WorldDagProjection;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldRetentionClass {
    CurrentHead,
    CompetingHead,
    ActiveExecution,
    TaskCheckpoint,
    ReplayPin,
    SimulationPin,
    ComparisonPin,
    MergeConflict,
    PromotionState,
    ReconciliationState,
    RollbackHold,
    LegalHold,
    EvidenceHold,
    OperatorHold,
    RemoteLease,
    IncompleteTransfer,
}

impl WorldRetentionClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CurrentHead => "current-head",
            Self::CompetingHead => "competing-head",
            Self::ActiveExecution => "active-execution",
            Self::TaskCheckpoint => "task-checkpoint",
            Self::ReplayPin => "replay-pin",
            Self::SimulationPin => "simulation-pin",
            Self::ComparisonPin => "comparison-pin",
            Self::MergeConflict => "merge-conflict",
            Self::PromotionState => "promotion-state",
            Self::ReconciliationState => "reconciliation-state",
            Self::RollbackHold => "rollback-hold",
            Self::LegalHold => "legal-hold",
            Self::EvidenceHold => "evidence-hold",
            Self::OperatorHold => "operator-hold",
            Self::RemoteLease => "remote-lease",
            Self::IncompleteTransfer => "incomplete-transfer",
        }
    }

    pub const fn all() -> [Self; MAX_WORLD_RETENTION_CLASSES] {
        [
            Self::CurrentHead,
            Self::CompetingHead,
            Self::ActiveExecution,
            Self::TaskCheckpoint,
            Self::ReplayPin,
            Self::SimulationPin,
            Self::ComparisonPin,
            Self::MergeConflict,
            Self::PromotionState,
            Self::ReconciliationState,
            Self::RollbackHold,
            Self::LegalHold,
            Self::EvidenceHold,
            Self::OperatorHold,
            Self::RemoteLease,
            Self::IncompleteTransfer,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldRetentionClassObservation {
    pub class: WorldRetentionClass,
    pub owner_ref: String,
    pub roots: Vec<WorldObjectRef>,
    pub observed: bool,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RemoteLeaseState {
    Active,
    Cleared,
    Uncertain,
    Contradictory,
    Unavailable,
}

impl RemoteLeaseState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Cleared => "cleared",
            Self::Uncertain => "uncertain",
            Self::Contradictory => "contradictory",
            Self::Unavailable => "unavailable",
        }
    }

    pub const fn retains_roots(self) -> bool {
        !matches!(self, Self::Cleared)
    }

    pub const fn unresolved(self) -> bool {
        matches!(self, Self::Uncertain | Self::Contradictory | Self::Unavailable)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldRemoteLeaseObservation {
    pub lease_ref: String,
    pub peer_ref: String,
    pub generation: u64,
    pub validity_basis_ref: String,
    pub roots: Vec<WorldObjectRef>,
    pub state: RemoteLeaseState,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldRetentionProjectionRequest {
    pub snapshot_ref: String,
    pub generation_ref: String,
    pub projection: WorldDagProjection,
    pub classes: Vec<WorldRetentionClassObservation>,
    pub remote_leases: Vec<WorldRemoteLeaseObservation>,
    pub edge_inventory_complete: bool,
    pub attribution_inventory_complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBindingReachabilityReport {
    pub decision: RetirementDecision,
    pub observation_only: bool,
    pub retention_authorized: bool,
    pub deletion_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldRetentionReport {
    pub snapshot_ref: String,
    pub generation_ref: String,
    pub retained_refs: Vec<String>,
    pub remote_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub missing_classes: Vec<WorldRetentionClass>,
    pub unresolved_remote: Vec<String>,
    pub reference_index_complete: bool,
    pub shared_classification: RetirementClassification,
    pub binding_report: WorldBindingReachabilityReport,
    pub observation_only: bool,
    pub retention_authorized: bool,
    pub deletion_authorized: bool,
    pub non_claims: Vec<String>,
}
