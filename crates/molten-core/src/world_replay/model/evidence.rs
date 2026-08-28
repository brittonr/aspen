use crate::world_commit::RootKind;
use crate::world_commit::WorldCommitRef;
use crate::world_commit::WorldRootRef;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldReplayFieldDifference {
    pub root_kind: RootKind,
    pub field_path: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldReplayObservedCommit {
    pub commit_ref: WorldCommitRef,
    pub roots: Vec<WorldRootRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldReplayTransitionObservation {
    pub position: u64,
    pub observed_parent: WorldCommitRef,
    pub actual: WorldReplayObservedCommit,
    pub field_differences: Vec<WorldReplayFieldDifference>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldReplayDivergenceKind {
    MissingObservation,
    ParentMismatch,
    CommitMismatch,
    RootMismatch,
    UnexpectedObservation,
}

impl WorldReplayDivergenceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MissingObservation => "missing-observation",
            Self::ParentMismatch => "parent-mismatch",
            Self::CommitMismatch => "commit-mismatch",
            Self::RootMismatch => "root-mismatch",
            Self::UnexpectedObservation => "unexpected-observation",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldReplayDivergence {
    pub schema: String,
    pub divergence_ref: String,
    pub kind: WorldReplayDivergenceKind,
    pub position: u64,
    pub expected_parent: WorldCommitRef,
    pub observed_parent: Option<WorldCommitRef>,
    pub expected_commit: WorldCommitRef,
    pub actual_commit: Option<WorldCommitRef>,
    pub root_kind: Option<RootKind>,
    pub field_path: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldReplayComparison {
    pub complete: bool,
    pub matched_steps: usize,
    pub divergence: Option<WorldReplayDivergence>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldReplayReceiptDecision {
    Replayed,
    Diverged,
    Denied,
}

impl WorldReplayReceiptDecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Replayed => "replayed",
            Self::Diverged => "diverged",
            Self::Denied => "denied",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldReplayReceipt {
    pub schema: String,
    pub receipt_ref: String,
    pub decision: WorldReplayReceiptDecision,
    pub trace_ref: String,
    pub capsule_ref: String,
    pub profile_ref: String,
    pub horizon: usize,
    pub actual_transition_refs: Vec<String>,
    pub divergence_ref: Option<String>,
    pub current_admission_ref: Option<String>,
    pub dependency_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub non_claims: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldReplayImportDecision {
    Available,
    Denied,
}

impl WorldReplayImportDecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Denied => "denied",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldReplayImportReceipt {
    pub schema: String,
    pub receipt_ref: String,
    pub decision: WorldReplayImportDecision,
    pub capsule_ref: String,
    pub verified_members: usize,
    pub availability_ref: Option<String>,
    pub diagnostics: Vec<String>,
    pub branch_moved: bool,
    pub runtime_activated: bool,
    pub authority_granted: bool,
    pub non_claims: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldReplayIssue {
    InvalidBounds(&'static str),
    InvalidSchema(&'static str),
    InvalidReference(&'static str),
    InvalidText(&'static str),
    TraceIdentityMismatch,
    CapsuleIdentityMismatch,
    PlanIdentityMismatch,
    EmptyTrace,
    StepLimitExceeded,
    MemberLimitExceeded,
    MemberByteLimitExceeded(String),
    TotalByteLimitExceeded,
    DiagnosticLimitExceeded,
    DependencyLimitExceeded,
    NonContiguousStep { expected: u64, actual: u64 },
    StepParentMismatch { position: u64 },
    StepProfileMismatch { position: u64 },
    UnsupportedProfile,
    LogicalCohortUnexpected,
    LogicalSnapshotDescriptorUnexpected,
    OpaqueCohortMissing,
    OpaqueSnapshotDescriptorMissing,
    DuplicateCommit(String),
    MissingCommit(String),
    CommitIdentityUnverified(String),
    CommitParentMismatch(String),
    DuplicateCommitRoot { commit_ref: String, root_kind: RootKind },
    DuplicateMember(String),
    NonCanonicalMemberOrder,
    EmptyMemberRoles(String),
    TooManyMemberRoles(String),
    NonCanonicalMemberRoleOrder(String),
    InvalidMemberProtection(String),
    MissingClosureRole { object_ref: String, role: String },
    UndeclaredClosureRole { object_ref: String, role: String },
    FieldPathLimitExceeded,
    InvalidFieldPath,
    ObservationLimitExceeded,
    ObservationPositionMismatch { expected: u64, actual: u64 },
}
