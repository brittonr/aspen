use super::capsule::WorldReplayCapsule;
use super::capsule::WorldReplayClosureRequirement;
use super::capsule::WorldReplayCommitClosure;
use super::trace::WorldReplayBounds;
use super::trace::WorldReplayProfile;
use super::trace::WorldTransitionTrace;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldReplayPlanRequest {
    pub trace: WorldTransitionTrace,
    pub capsule: WorldReplayCapsule,
    pub commits: Vec<WorldReplayCommitClosure>,
    pub additional_requirements: Vec<WorldReplayClosureRequirement>,
    pub supported_profile_refs: Vec<String>,
    pub bounds: WorldReplayBounds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldReplayOperationKind {
    MaterializeMember,
    RestoreLogicalProfile,
    RestoreOpaqueProfile,
    RecheckCurrentAdmission,
    ExecuteTransition,
    CaptureSuccessor,
    CompareSuccessor,
    PublishReceipt,
}

impl WorldReplayOperationKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MaterializeMember => "materialize-member",
            Self::RestoreLogicalProfile => "restore-logical-profile",
            Self::RestoreOpaqueProfile => "restore-opaque-profile",
            Self::RecheckCurrentAdmission => "recheck-current-admission",
            Self::ExecuteTransition => "execute-transition",
            Self::CaptureSuccessor => "capture-successor",
            Self::CompareSuccessor => "compare-successor",
            Self::PublishReceipt => "publish-receipt",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldReplayOperation {
    pub kind: WorldReplayOperationKind,
    pub position: Option<u64>,
    pub subject_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldReplayPlan {
    pub schema: String,
    pub plan_ref: String,
    pub trace_ref: String,
    pub capsule_ref: String,
    pub profile: WorldReplayProfile,
    pub operations: Vec<WorldReplayOperation>,
    pub current_admission_required: bool,
    pub non_claims: Vec<String>,
}
