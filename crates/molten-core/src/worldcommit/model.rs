use super::MAX_WORLD_COMMIT_CLOSURE_OBJECTS;
use super::MAX_WORLD_COMMIT_PARENTS;
use super::MAX_WORLD_COMMIT_REVISION_FENCES;
use super::MAX_WORLD_COMMIT_ROOTS;
use super::RootKind;
use super::SnapshotCohortRef;
use super::SnapshotProfileRef;
use super::WorldCommitRef;
use super::WorldCommitReferenceError;
use super::WorldCommitVersion;
use super::WorldRootRef;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SnapshotProfileKind {
    Logical,
    Opaque,
    Mixed,
}

impl SnapshotProfileKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Logical => "logical",
            Self::Opaque => "opaque",
            Self::Mixed => "mixed",
        }
    }

    pub fn parse(value: &str) -> Result<Self, WorldCommitReferenceError> {
        match value {
            "logical" => Ok(Self::Logical),
            "opaque" => Ok(Self::Opaque),
            "mixed" => Ok(Self::Mixed),
            _ => Err(WorldCommitReferenceError::UnsupportedProfileKind(value.to_string())),
        }
    }

    pub const fn required_roots(self) -> &'static [RootKind] {
        match self {
            Self::Logical => &LOGICAL_REQUIRED_ROOTS,
            Self::Opaque => &OPAQUE_REQUIRED_ROOTS,
            Self::Mixed => &MIXED_REQUIRED_ROOTS,
        }
    }
}

const LOGICAL_REQUIRED_ROOT_COUNT: usize = 11;
const OPAQUE_REQUIRED_ROOT_COUNT: usize = 5;
const MIXED_REQUIRED_ROOT_COUNT: usize = 12;

const LOGICAL_REQUIRED_ROOTS: [RootKind; LOGICAL_REQUIRED_ROOT_COUNT] = [
    RootKind::Artifact,
    RootKind::Schema,
    RootKind::DurableState,
    RootKind::Tasks,
    RootKind::History,
    RootKind::Effects,
    RootKind::Scheduler,
    RootKind::Time,
    RootKind::Entropy,
    RootKind::RuntimeProfile,
    RootKind::Policy,
];
const OPAQUE_REQUIRED_ROOTS: [RootKind; OPAQUE_REQUIRED_ROOT_COUNT] = [
    RootKind::Artifact,
    RootKind::Schema,
    RootKind::RuntimeProfile,
    RootKind::Policy,
    RootKind::OpaqueMachineSnapshot,
];
const MIXED_REQUIRED_ROOTS: [RootKind; MIXED_REQUIRED_ROOT_COUNT] = [
    RootKind::Artifact,
    RootKind::Schema,
    RootKind::DurableState,
    RootKind::Tasks,
    RootKind::History,
    RootKind::Effects,
    RootKind::Scheduler,
    RootKind::Time,
    RootKind::Entropy,
    RootKind::RuntimeProfile,
    RootKind::Policy,
    RootKind::OpaqueMachineSnapshot,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotProfile {
    pub kind: SnapshotProfileKind,
    pub profile_ref: SnapshotProfileRef,
    pub cohort_ref: Option<SnapshotCohortRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletenessClaim {
    pub required_roots: Vec<RootKind>,
}

impl CompletenessClaim {
    pub fn for_profile(profile: SnapshotProfileKind) -> Self {
        Self {
            required_roots: profile.required_roots().to_vec(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldCommitCore {
    pub version: WorldCommitVersion,
    pub profile: SnapshotProfile,
    pub parents: Vec<WorldCommitRef>,
    pub roots: Vec<WorldRootRef>,
    pub completeness: CompletenessClaim,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldCommitBounds {
    pub max_parents: usize,
    pub max_roots: usize,
    pub max_revision_fences: usize,
    pub max_closure_objects: usize,
}

impl Default for WorldCommitBounds {
    fn default() -> Self {
        Self {
            max_parents: MAX_WORLD_COMMIT_PARENTS,
            max_roots: MAX_WORLD_COMMIT_ROOTS,
            max_revision_fences: MAX_WORLD_COMMIT_REVISION_FENCES,
            max_closure_objects: MAX_WORLD_COMMIT_CLOSURE_OBJECTS,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootClosureObservation {
    pub root: WorldRootRef,
    pub object_present: bool,
    pub identity_matches: bool,
    pub schema_matches: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParentClosureObservation {
    pub commit_ref: WorldCommitRef,
    pub parents: Vec<WorldCommitRef>,
    pub object_present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosureRequest {
    pub commit_ref: WorldCommitRef,
    pub core: WorldCommitCore,
    pub roots: Vec<RootClosureObservation>,
    pub parent_graph: Vec<ParentClosureObservation>,
    pub bounds: WorldCommitBounds,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosureReport {
    pub commit_ref: WorldCommitRef,
    pub complete: bool,
    pub first_missing_root: Option<RootKind>,
    pub issues: Vec<ClosureIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClosureIssue {
    InvalidCore(String),
    BoundExceeded {
        field: &'static str,
        actual: usize,
        maximum: usize,
    },
    DuplicateRootObservation(RootKind),
    MissingRootObject(RootKind),
    RootIdentityMismatch(RootKind),
    RootSchemaMismatch(RootKind),
    UnexpectedRootObservation(RootKind),
    DuplicateParentObservation(String),
    ParentEdgeBoundExceeded {
        commit_ref: String,
        actual: usize,
        maximum: usize,
    },
    DuplicateParentEdge {
        commit_ref: String,
        parent_ref: String,
    },
    MissingParentObject(String),
    MissingParentObservation(String),
    ParentCycle(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RootReplayClass {
    VerifyOnly,
    ReplayLogicalState,
    HistoricalEvidenceOnly,
    RestoreOpaqueState,
}

impl RootReplayClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::VerifyOnly => "verify-only",
            Self::ReplayLogicalState => "replay-logical-state",
            Self::HistoricalEvidenceOnly => "historical-evidence-only",
            Self::RestoreOpaqueState => "restore-opaque-state",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RestoreStepKind {
    VerifySchema,
    MaterializeArtifacts,
    AdmitPolicy,
    AdmitRuntimeProfile,
    RestoreDurableState,
    RestoreHistory,
    RestoreTasks,
    RestoreScheduler,
    RestoreTime,
    RestoreEntropy,
    RestoreEffects,
    RecordAuthorityObservation,
    RestoreOpaqueMachineSnapshot,
    RecheckCurrentAdmission,
    ActivateRuntime,
}

impl RestoreStepKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::VerifySchema => "verify-schema",
            Self::MaterializeArtifacts => "materialize-artifacts",
            Self::AdmitPolicy => "admit-policy",
            Self::AdmitRuntimeProfile => "admit-runtime-profile",
            Self::RestoreDurableState => "restore-durable-state",
            Self::RestoreHistory => "restore-history",
            Self::RestoreTasks => "restore-tasks",
            Self::RestoreScheduler => "restore-scheduler",
            Self::RestoreTime => "restore-time",
            Self::RestoreEntropy => "restore-entropy",
            Self::RestoreEffects => "restore-effects",
            Self::RecordAuthorityObservation => "record-authority-observation",
            Self::RestoreOpaqueMachineSnapshot => "restore-opaque-machine-snapshot",
            Self::RecheckCurrentAdmission => "recheck-current-admission",
            Self::ActivateRuntime => "activate-runtime",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreStep {
    pub kind: RestoreStepKind,
    pub root: Option<WorldRootRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootReplayClassification {
    pub root_kind: RootKind,
    pub class: RootReplayClass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestorePlan {
    pub commit_ref: WorldCommitRef,
    pub steps: Vec<RestoreStep>,
    pub replay: Vec<RootReplayClassification>,
    pub current_admission_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestoreIssue {
    IncompleteClosure(Vec<ClosureIssue>),
    ClosureCommitMismatch,
    InvalidCore,
    RootUnavailable(RootKind),
}
