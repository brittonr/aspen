use super::model::CohortFactKind;
use super::model::CompatibilityVerdict;
use super::model::ComponentOwner;
use super::model::SnapshotClass;
use super::model::SnapshotComponentKind;
use super::model::SnapshotIssue;
use crate::world_commit::WorldCommitRef;

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

    pub fn parse(value: &str) -> Result<Self, SnapshotIssue> {
        match value {
            "artifact" => Ok(Self::Artifact),
            "schema" => Ok(Self::Schema),
            "durable-state" => Ok(Self::DurableState),
            "tasks" => Ok(Self::Tasks),
            "history" => Ok(Self::History),
            "effects" => Ok(Self::Effects),
            "scheduler" => Ok(Self::Scheduler),
            "time" => Ok(Self::Time),
            "entropy" => Ok(Self::Entropy),
            "runtime-profile" => Ok(Self::RuntimeProfile),
            "policy" => Ok(Self::Policy),
            "machine-descriptor" => Ok(Self::MachineDescriptor),
            "cpu-state" => Ok(Self::CpuState),
            "memory" => Ok(Self::Memory),
            "device-state" => Ok(Self::DeviceState),
            "disk-state" => Ok(Self::DiskState),
            "backend-state" => Ok(Self::BackendState),
            _ => Err(SnapshotIssue::UnsupportedComponentKind),
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

    pub fn parse(value: &str) -> Result<Self, SnapshotIssue> {
        match value {
            "architecture" => Ok(Self::Architecture),
            "runtime-build" => Ok(Self::RuntimeBuild),
            "runtime-abi" => Ok(Self::RuntimeAbi),
            "schema-set" => Ok(Self::SchemaSet),
            "handler-set" => Ok(Self::HandlerSet),
            "task-model" => Ok(Self::TaskModel),
            "scheduler-profile" => Ok(Self::SchedulerProfile),
            "time-profile" => Ok(Self::TimeProfile),
            "entropy-profile" => Ok(Self::EntropyProfile),
            "effect-profile" => Ok(Self::EffectProfile),
            "kvm-state-profile" => Ok(Self::KvmStateProfile),
            "cpu-feature-inventory" => Ok(Self::CpuFeatureInventory),
            "vcpu-topology" => Ok(Self::VcpuTopology),
            "device-inventory" => Ok(Self::DeviceInventory),
            "memory-format" => Ok(Self::MemoryFormat),
            "disk-format" => Ok(Self::DiskFormat),
            "backend-profile" => Ok(Self::BackendProfile),
            _ => Err(SnapshotIssue::UnsupportedCohortFact),
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

    pub fn parse(value: &str) -> Result<Self, SnapshotIssue> {
        match value {
            "molten" => Ok(Self::Molten),
            "chaoscontrol" => Ok(Self::ChaosControl),
            _ => Err(SnapshotIssue::UnsupportedOwner),
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
