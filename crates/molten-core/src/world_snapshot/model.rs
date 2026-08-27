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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotRestoreStep {
    VerifyClosure,
    VerifyCohort,
    MaterializeArtifacts,
    RestoreDurableState,
    RestoreHistory,
    RestoreTasks,
    RestoreScheduler,
    RestoreTime,
    RestoreEntropy,
    RestoreEffects,
    RestoreOpaqueMachine,
    RecreateHostHandles,
    RecheckCurrentAdmission,
    ActivateRuntime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotRestorePlan {
    pub commit_ref: WorldCommitRef,
    pub class: SnapshotClass,
    pub steps: Vec<SnapshotRestoreStep>,
    pub activation_permitted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OverlayIdentity(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloneChild {
    pub parent_ref: WorldCommitRef,
    pub memory_overlay: OverlayIdentity,
    pub device_overlay: OverlayIdentity,
    pub disk_overlay: OverlayIdentity,
    pub endpoint_overlay: OverlayIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClonePlanRequest {
    pub parent_ref: WorldCommitRef,
    pub children: Vec<CloneChild>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotReceiptDecision {
    Planned,
    Restored,
    Cloned,
    Denied,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotReceipt {
    pub decision: SnapshotReceiptDecision,
    pub descriptor_ref: String,
    pub compatibility_ref: String,
    pub restore_plan_ref: Option<String>,
    pub clone_plan_ref: Option<String>,
    pub current_admission_ref: Option<String>,
    pub issues: Vec<String>,
    pub non_claims: Vec<String>,
}

pub const SNAPSHOT_NON_CLAIMS: &[&str] = &[
    "snapshot-completeness-is-not-guest-correctness",
    "snapshot-compatibility-is-not-cross-host-portability",
    "restore-planning-is-not-current-authority",
    "snapshot-bytes-do-not-transfer-host-handles",
    "opaque-identity-is-not-logical-semantic-equivalence",
    "clone-isolation-does-not-prove-workload-correctness",
    "snapshot-receipts-do-not-prove-release-eligibility",
];

impl SnapshotClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Logical => "logical",
            Self::Opaque => "opaque",
        }
    }

    pub fn parse(value: &str) -> Result<Self, SnapshotIssue> {
        match value {
            "logical" => Ok(Self::Logical),
            "opaque" => Ok(Self::Opaque),
            _ => Err(SnapshotIssue::UnsupportedProfile),
        }
    }
}

impl SnapshotComponentKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Artifact => "artifact",
            Self::Schema => "schema",
            Self::DurableState => "durable-state",
            Self::Tasks => "tasks",
            Self::History => "history",
            Self::Effects => "effects",
            Self::Scheduler => "scheduler",
            Self::Time => "time",
            Self::Entropy => "entropy",
            Self::RuntimeProfile => "runtime-profile",
            Self::Policy => "policy",
            Self::MachineDescriptor => "machine-descriptor",
            Self::CpuState => "cpu-state",
            Self::Memory => "memory",
            Self::DeviceState => "device-state",
            Self::DiskState => "disk-state",
            Self::BackendState => "backend-state",
        }
    }
}

impl CohortFactKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Architecture => "architecture",
            Self::RuntimeBuild => "runtime-build",
            Self::RuntimeAbi => "runtime-abi",
            Self::SchemaSet => "schema-set",
            Self::HandlerSet => "handler-set",
            Self::TaskModel => "task-model",
            Self::SchedulerProfile => "scheduler-profile",
            Self::TimeProfile => "time-profile",
            Self::EntropyProfile => "entropy-profile",
            Self::EffectProfile => "effect-profile",
            Self::KvmStateProfile => "kvm-state-profile",
            Self::CpuFeatureInventory => "cpu-feature-inventory",
            Self::VcpuTopology => "vcpu-topology",
            Self::DeviceInventory => "device-inventory",
            Self::MemoryFormat => "memory-format",
            Self::DiskFormat => "disk-format",
            Self::BackendProfile => "backend-profile",
        }
    }
}

impl ComponentOwner {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Molten => "molten",
            Self::ChaosControl => "chaoscontrol",
        }
    }
}

impl CompatibilityVerdict {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Compatible => "compatible",
            Self::Incomplete => "incomplete",
            Self::Incompatible => "incompatible",
            Self::Unsafe => "unsafe",
        }
    }
}

impl SnapshotRestoreStep {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::VerifyClosure => "verify-closure",
            Self::VerifyCohort => "verify-cohort",
            Self::MaterializeArtifacts => "materialize-artifacts",
            Self::RestoreDurableState => "restore-durable-state",
            Self::RestoreHistory => "restore-history",
            Self::RestoreTasks => "restore-tasks",
            Self::RestoreScheduler => "restore-scheduler",
            Self::RestoreTime => "restore-time",
            Self::RestoreEntropy => "restore-entropy",
            Self::RestoreEffects => "restore-effects",
            Self::RestoreOpaqueMachine => "restore-opaque-machine",
            Self::RecreateHostHandles => "recreate-host-handles",
            Self::RecheckCurrentAdmission => "recheck-current-admission",
            Self::ActivateRuntime => "activate-runtime",
        }
    }
}

impl SnapshotReceiptDecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Restored => "restored",
            Self::Cloned => "cloned",
            Self::Denied => "denied",
        }
    }
}
