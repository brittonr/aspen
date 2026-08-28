use super::super::MAX_WORLD_OPERATOR_CANONICAL_BYTES;
use super::super::MAX_WORLD_OPERATOR_DEPENDENCIES;
use super::super::MAX_WORLD_OPERATOR_OPERATIONS;
use super::super::MAX_WORLD_OPERATOR_RECEIPT_LINKS;
use crate::world_commit::WorldCommitRef;
use crate::world_head::WorldBranchId;
use crate::world_head::WorldHeadPolicyRef;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldComponentOwner {
    WorldCommit,
    WorldHead,
    FabricSimulation,
    WorldMerge,
    WorldReplay,
    WorldPromotion,
    WorldDistribution,
}

impl WorldComponentOwner {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WorldCommit => "world-commit",
            Self::WorldHead => "world-head",
            Self::FabricSimulation => "fabric-simulation",
            Self::WorldMerge => "world-merge",
            Self::WorldReplay => "world-replay",
            Self::WorldPromotion => "world-promotion",
            Self::WorldDistribution => "world-distribution",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldOperationKind {
    Inspect,
    Checkpoint,
    Branch,
    Run,
    Diff,
    Conflicts,
    Replay,
    Simulate,
    Verify,
    Promote,
    Export,
    Import,
    GarbageCollectionPlan,
}

impl WorldOperationKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Inspect => "inspect",
            Self::Checkpoint => "checkpoint",
            Self::Branch => "branch",
            Self::Run => "run",
            Self::Diff => "diff",
            Self::Conflicts => "conflicts",
            Self::Replay => "replay",
            Self::Simulate => "simulate",
            Self::Verify => "verify",
            Self::Promote => "promote",
            Self::Export => "export",
            Self::Import => "import",
            Self::GarbageCollectionPlan => "gc-plan",
        }
    }

    pub const fn is_mutating(self) -> bool {
        matches!(self, Self::Checkpoint | Self::Branch | Self::Run | Self::Promote | Self::Import)
    }

    pub const fn is_semantic_comparison(self) -> bool {
        matches!(self, Self::Diff | Self::Conflicts)
    }

    pub const fn owner(self) -> WorldComponentOwner {
        match self {
            Self::Inspect | Self::Checkpoint => WorldComponentOwner::WorldCommit,
            Self::Branch => WorldComponentOwner::WorldHead,
            Self::Run | Self::Simulate => WorldComponentOwner::FabricSimulation,
            Self::Diff | Self::Conflicts => WorldComponentOwner::WorldMerge,
            Self::Replay | Self::Verify | Self::Export | Self::Import => WorldComponentOwner::WorldReplay,
            Self::Promote => WorldComponentOwner::WorldPromotion,
            Self::GarbageCollectionPlan => WorldComponentOwner::WorldDistribution,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldProfileKind {
    Logical,
    Opaque,
    WitnessedHead,
    ExecutableExtent,
}

impl WorldProfileKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Logical => "logical",
            Self::Opaque => "opaque",
            Self::WitnessedHead => "witnessed-head",
            Self::ExecutableExtent => "executable-extent",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldProfileStatus {
    Admitted,
    Blocked,
    Unsupported,
    Unavailable,
}

impl WorldProfileStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Admitted => "admitted",
            Self::Blocked => "blocked",
            Self::Unsupported => "unsupported",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldProfileCapability {
    pub profile_ref: String,
    pub kind: WorldProfileKind,
    pub status: WorldProfileStatus,
    pub status_ref: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldExpectedObservationKind {
    Head,
    Policy,
    Authority,
    Profile,
    Conflict,
    Effect,
    CapsuleClosure,
    Retention,
    Witness,
    ExecutableExtent,
}

impl WorldExpectedObservationKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Head => "head",
            Self::Policy => "policy",
            Self::Authority => "authority",
            Self::Profile => "profile",
            Self::Conflict => "conflict",
            Self::Effect => "effect",
            Self::CapsuleClosure => "capsule-closure",
            Self::Retention => "retention",
            Self::Witness => "witness",
            Self::ExecutableExtent => "executable-extent",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldExpectedObservation {
    pub kind: WorldExpectedObservationKind,
    pub observation_ref: String,
    pub subject_ref: String,
    pub admitted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldWorkflowLimits {
    pub limits_ref: String,
    pub max_operations: usize,
    pub max_dependencies_per_operation: usize,
    pub max_receipt_links: usize,
    pub max_canonical_bytes: usize,
}

impl WorldWorkflowLimits {
    pub fn standard(limits_ref: String) -> Self {
        Self {
            limits_ref,
            max_operations: MAX_WORLD_OPERATOR_OPERATIONS,
            max_dependencies_per_operation: MAX_WORLD_OPERATOR_DEPENDENCIES,
            max_receipt_links: MAX_WORLD_OPERATOR_RECEIPT_LINKS,
            max_canonical_bytes: MAX_WORLD_OPERATOR_CANONICAL_BYTES,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldOperationRequest {
    pub operation_id: String,
    pub kind: WorldOperationKind,
    pub subject_ref: String,
    pub profile_ref: String,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldWorkflowRequest {
    pub schema: String,
    pub request_ref: String,
    pub world_ref: WorldCommitRef,
    pub branch_id: WorldBranchId,
    pub expected_head: WorldCommitRef,
    pub expected_generation: u64,
    pub policy_ref: WorldHeadPolicyRef,
    pub authority_observation_ref: String,
    pub limits: WorldWorkflowLimits,
    pub profiles: Vec<WorldProfileCapability>,
    pub observations: Vec<WorldExpectedObservation>,
    pub operations: Vec<WorldOperationRequest>,
}
