use crate::world_commit::RootKind;
use crate::world_commit::SnapshotCohortRef;
use crate::world_commit::SnapshotProfileRef;
use crate::world_commit::WorldCommitRef;
use crate::world_commit::WorldRootRef;

pub const MAX_SNAPSHOT_COMPONENTS: usize = 64;
pub const MAX_COHORT_FACTS: usize = 64;
pub const MAX_CLONE_CHILDREN: usize = 32;
pub const MAX_SNAPSHOT_CANONICAL_BYTES: usize = 1_048_576;
pub const MAX_SNAPSHOT_RECEIPT_ISSUES: usize = 64;
pub const MAX_SNAPSHOT_ISSUE_BYTES: usize = 256;
pub const MAX_OVERLAY_IDENTITY_BYTES: usize = 256;

pub const SNAPSHOT_DESCRIPTOR_SCHEMA: &str = "molten.world-snapshot.descriptor.v1";
pub const SNAPSHOT_INVENTORY_SCHEMA: &str = "molten.world-snapshot.inventory.v1";
pub const SNAPSHOT_COMPATIBILITY_SCHEMA: &str = "molten.world-snapshot.compatibility.v1";
pub const SNAPSHOT_RESTORE_PLAN_SCHEMA: &str = "molten.world-snapshot.restore-plan.v1";
pub const SNAPSHOT_CLONE_PLAN_SCHEMA: &str = "molten.world-snapshot.clone-plan.v1";
pub const SNAPSHOT_RECEIPT_SCHEMA: &str = "molten.world-snapshot.receipt.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotClass {
    Logical,
    Opaque,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SnapshotComponentKind {
    Artifact,
    Schema,
    DurableState,
    Tasks,
    History,
    Effects,
    Scheduler,
    Time,
    Entropy,
    RuntimeProfile,
    Policy,
    MachineDescriptor,
    CpuState,
    Memory,
    DeviceState,
    DiskState,
    BackendState,
}

impl SnapshotComponentKind {
    pub const fn root_kind(self) -> Option<RootKind> {
        match self {
            Self::Artifact => Some(RootKind::Artifact),
            Self::Schema => Some(RootKind::Schema),
            Self::DurableState => Some(RootKind::DurableState),
            Self::Tasks => Some(RootKind::Tasks),
            Self::History => Some(RootKind::History),
            Self::Effects => Some(RootKind::Effects),
            Self::Scheduler => Some(RootKind::Scheduler),
            Self::Time => Some(RootKind::Time),
            Self::Entropy => Some(RootKind::Entropy),
            Self::RuntimeProfile => Some(RootKind::RuntimeProfile),
            Self::Policy => Some(RootKind::Policy),
            Self::MachineDescriptor => Some(RootKind::OpaqueMachineSnapshot),
            Self::CpuState | Self::Memory | Self::DeviceState | Self::DiskState | Self::BackendState => None,
        }
    }
}

pub const LOGICAL_COMPONENTS: &[SnapshotComponentKind] = &[
    SnapshotComponentKind::Artifact,
    SnapshotComponentKind::Schema,
    SnapshotComponentKind::DurableState,
    SnapshotComponentKind::Tasks,
    SnapshotComponentKind::History,
    SnapshotComponentKind::Effects,
    SnapshotComponentKind::Scheduler,
    SnapshotComponentKind::Time,
    SnapshotComponentKind::Entropy,
    SnapshotComponentKind::RuntimeProfile,
    SnapshotComponentKind::Policy,
];

pub const OPAQUE_COMPONENTS: &[SnapshotComponentKind] = &[
    SnapshotComponentKind::Artifact,
    SnapshotComponentKind::Schema,
    SnapshotComponentKind::RuntimeProfile,
    SnapshotComponentKind::Policy,
    SnapshotComponentKind::MachineDescriptor,
    SnapshotComponentKind::CpuState,
    SnapshotComponentKind::Memory,
    SnapshotComponentKind::DeviceState,
    SnapshotComponentKind::DiskState,
    SnapshotComponentKind::BackendState,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CohortFactKind {
    Architecture,
    RuntimeBuild,
    RuntimeAbi,
    SchemaSet,
    HandlerSet,
    TaskModel,
    SchedulerProfile,
    TimeProfile,
    EntropyProfile,
    EffectProfile,
    KvmStateProfile,
    CpuFeatureInventory,
    VcpuTopology,
    DeviceInventory,
    MemoryFormat,
    DiskFormat,
    BackendProfile,
}

pub const LOGICAL_COHORT_FACTS: &[CohortFactKind] = &[
    CohortFactKind::RuntimeBuild,
    CohortFactKind::RuntimeAbi,
    CohortFactKind::SchemaSet,
    CohortFactKind::HandlerSet,
    CohortFactKind::TaskModel,
    CohortFactKind::SchedulerProfile,
    CohortFactKind::TimeProfile,
    CohortFactKind::EntropyProfile,
    CohortFactKind::EffectProfile,
];

pub const OPAQUE_COHORT_FACTS: &[CohortFactKind] = &[
    CohortFactKind::Architecture,
    CohortFactKind::RuntimeBuild,
    CohortFactKind::RuntimeAbi,
    CohortFactKind::KvmStateProfile,
    CohortFactKind::CpuFeatureInventory,
    CohortFactKind::VcpuTopology,
    CohortFactKind::DeviceInventory,
    CohortFactKind::MemoryFormat,
    CohortFactKind::DiskFormat,
    CohortFactKind::BackendProfile,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CohortFact {
    pub kind: CohortFactKind,
    pub identity: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotCohort {
    pub cohort_ref: SnapshotCohortRef,
    pub facts: Vec<CohortFact>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentOwner {
    Molten,
    ChaosControl,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotComponent {
    pub kind: SnapshotComponentKind,
    pub identity: String,
    pub root: Option<WorldRootRef>,
    pub owner: ComponentOwner,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotDescriptor {
    pub class: SnapshotClass,
    pub commit_ref: WorldCommitRef,
    pub profile_ref: SnapshotProfileRef,
    pub cohort: SnapshotCohort,
    pub components: Vec<SnapshotComponent>,
    pub contains_live_handle: bool,
    pub synchronization: Option<SnapshotSynchronization>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotSynchronization {
    pub logical_commit_ref: WorldCommitRef,
    pub opaque_snapshot_ref: WorldRootRef,
    pub observation_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotInventory {
    pub class: SnapshotClass,
    pub required: Vec<SnapshotComponentKind>,
    pub observed: Vec<SnapshotComponent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotOwnership {
    pub component: SnapshotComponentKind,
    pub owner: ComponentOwner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityVerdict {
    Compatible,
    Incomplete,
    Incompatible,
    Unsafe,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SnapshotIssue {
    UnsupportedProfile,
    UnsupportedComponentKind,
    UnsupportedCohortFact,
    UnsupportedOwner,
    TooManyComponents,
    TooManyCohortFacts,
    DuplicateComponent(SnapshotComponentKind),
    MissingComponent(SnapshotComponentKind),
    UnexpectedComponent(SnapshotComponentKind),
    DuplicateCohortFact(CohortFactKind),
    MissingCohortFact(CohortFactKind),
    UnexpectedCohortFact(CohortFactKind),
    EmptyIdentity,
    WrongOwner(SnapshotComponentKind),
    MissingRoot(SnapshotComponentKind),
    WrongRootKind(SnapshotComponentKind),
    UnexpectedRoot(SnapshotComponentKind),
    LiveHandleCaptured,
    UnexpectedSynchronization,
    InvalidContentIdentity,
    InvalidOverlayIdentity,
    ReceiptBoundExceeded,
    ReceiptNonClaimsIncomplete,
    CohortIdentityMismatch,
    CohortMismatch(CohortFactKind),
    OpaqueMergeDenied,
    CurrentAdmissionDenied,
    EmptyClonePlan,
    ChildBoundExceeded,
    ParentMismatch,
    OverlayCollision,
    PartialOverlaySet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityReport {
    pub verdict: CompatibilityVerdict,
    pub issues: Vec<SnapshotIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotDiagnostic {
    pub issue: SnapshotIssue,
    pub component: Option<SnapshotComponentKind>,
    pub cohort_fact: Option<CohortFactKind>,
}
