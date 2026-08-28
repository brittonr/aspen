use super::WorldOperationKind;
use super::WorldProfileStatus;
use crate::world_commit::WorldCommitRef;
use crate::world_head::WorldBranchId;
use crate::world_head::WorldHeadPolicyRef;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldWorkflowBlockerCode {
    ProfileBlocked,
    ProfileUnsupported,
    ProfileUnavailable,
    OpaqueSemanticOperation,
    HeadObservationDenied,
    PolicyObservationDenied,
    AuthorityObservationDenied,
    ProfileObservationDenied,
    ConflictUnresolved,
    EffectObservationDenied,
    CapsuleIncomplete,
    RetentionObservationDenied,
    WitnessUnavailable,
    ExecutableExtentUnavailable,
    DependencyBlocked,
    HandlerUnavailable,
    StalePlan,
    MutableObservationDrift,
    ComponentDenied,
    ComponentOutcomeUnknown,
}

impl WorldWorkflowBlockerCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProfileBlocked => "profile-blocked",
            Self::ProfileUnsupported => "profile-unsupported",
            Self::ProfileUnavailable => "profile-unavailable",
            Self::OpaqueSemanticOperation => "opaque-semantic-operation",
            Self::HeadObservationDenied => "head-observation-denied",
            Self::PolicyObservationDenied => "policy-observation-denied",
            Self::AuthorityObservationDenied => "authority-observation-denied",
            Self::ProfileObservationDenied => "profile-observation-denied",
            Self::ConflictUnresolved => "conflict-unresolved",
            Self::EffectObservationDenied => "effect-observation-denied",
            Self::CapsuleIncomplete => "capsule-incomplete",
            Self::RetentionObservationDenied => "retention-observation-denied",
            Self::WitnessUnavailable => "witness-unavailable",
            Self::ExecutableExtentUnavailable => "executable-extent-unavailable",
            Self::DependencyBlocked => "dependency-blocked",
            Self::HandlerUnavailable => "handler-unavailable",
            Self::StalePlan => "stale-plan",
            Self::MutableObservationDrift => "mutable-observation-drift",
            Self::ComponentDenied => "component-denied",
            Self::ComponentOutcomeUnknown => "component-outcome-unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldWorkflowBlocker {
    pub operation_id: String,
    pub code: WorldWorkflowBlockerCode,
    pub evidence_ref: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldOperationPlanState {
    Ready,
    Blocked,
}

impl WorldOperationPlanState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Blocked => "blocked",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldOperationPlanNode {
    pub operation_id: String,
    pub kind: WorldOperationKind,
    pub subject_ref: String,
    pub profile_ref: String,
    pub dependencies: Vec<String>,
    pub state: WorldOperationPlanState,
    pub blocker: Option<WorldWorkflowBlocker>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldWorkflowPlan {
    pub schema: &'static str,
    pub plan_ref: String,
    pub request_ref: String,
    pub world_ref: WorldCommitRef,
    pub branch_id: WorldBranchId,
    pub expected_head: WorldCommitRef,
    pub expected_generation: u64,
    pub policy_ref: WorldHeadPolicyRef,
    pub authority_observation_ref: String,
    pub limits_ref: String,
    pub operations: Vec<WorldOperationPlanNode>,
    pub first_blocker: Option<WorldWorkflowBlocker>,
    pub non_claims: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldWorkflowIssue {
    InvalidSchema,
    InvalidLimits,
    OperationLimitExceeded,
    ProfileLimitExceeded,
    ObservationLimitExceeded,
    EmptyOperations,
    InvalidReference(&'static str),
    DuplicateOperation(String),
    DuplicateDependency(String),
    DuplicateProfile(String),
    DuplicateObservation(String),
    MissingDependency(String),
    DependencyCycle,
    ApplyOperationMissing,
    ApplyReadOnlyOperation,
    ApplyPlanMismatch,
    ApplyHeadMismatch,
    ApplyGenerationMismatch,
    ApplyPolicyMismatch,
    ApplyAuthorityMismatch,
    ApplyProfileMismatch,
    ApplyProfileDenied,
    ReceiptLimitExceeded,
    ReceiptOperationMissing(String),
    ReceiptOwnerMismatch,
    ReceiptOrderMismatch,
    ReceiptAfterBlocker,
    ReceiptOverclaimsAuthority,
    ReceiptOverclaimsDeletionAuthority,
    ReceiptContainsSensitiveMaterial,
    ReceiptMissingCompletion(String),
    IdentityLengthExceeded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldOperationCurrentFacts {
    pub plan_ref: String,
    pub operation_id: String,
    pub observed_head: WorldCommitRef,
    pub observed_generation: u64,
    pub policy_ref: WorldHeadPolicyRef,
    pub authority_observation_ref: String,
    pub profile_ref: String,
    pub profile_status: WorldProfileStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldOperationApplyAdmission {
    pub plan_ref: String,
    pub operation_id: String,
    pub admitted: bool,
}
