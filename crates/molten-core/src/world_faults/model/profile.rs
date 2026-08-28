use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldFaultLimits {
    pub max_cases: usize,
    pub max_schedules: usize,
    pub max_schedule_steps: usize,
    pub max_adapters: usize,
    pub max_observations: usize,
    pub max_unsupported_rows: usize,
    pub max_restarts: u32,
}

impl WorldFaultLimits {
    pub const fn standard() -> Self {
        Self {
            max_cases: MAX_WORLD_FAULT_CASES,
            max_schedules: MAX_WORLD_FAULT_SCHEDULES,
            max_schedule_steps: MAX_WORLD_FAULT_SCHEDULE_STEPS,
            max_adapters: MAX_WORLD_FAULT_ADAPTERS,
            max_observations: MAX_WORLD_FAULT_OBSERVATIONS,
            max_unsupported_rows: MAX_WORLD_FAULT_UNSUPPORTED_ROWS,
            max_restarts: MAX_WORLD_FAULT_RESTARTS,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct WorldFaultAdapterBinding {
    pub adapter_id: String,
    pub owner: WorldMutationOwner,
    pub profile: String,
    pub implementation_ref: String,
    pub semantic_phase_map_ref: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RecoveryClass {
    AlreadyComplete,
    SafeToRetry,
    Superseded,
    Conflict,
    Uncertain,
    Denied,
    Corrupt,
    ManualReview,
}

impl RecoveryClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AlreadyComplete => "already-complete",
            Self::SafeToRetry => "safe-to-retry",
            Self::Superseded => "superseded",
            Self::Conflict => "conflict",
            Self::Uncertain => "uncertain",
            Self::Denied => "denied",
            Self::Corrupt => "corrupt",
            Self::ManualReview => "manual-review",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldFaultCase {
    pub case_id: String,
    pub mutation: WorldMutationKind,
    pub operation_id: String,
    pub phase: FaultPhase,
    pub adapter_id: String,
    pub expected_generation: u64,
    pub pre_state_ref: String,
    pub expected_decision: RecoveryClass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InterleavingPoint {
    Prepare,
    CurrentFactRecheck,
    BeforeLinearization,
    AfterLinearization,
    DurableReadBack,
    Finish,
}

impl InterleavingPoint {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Prepare => "prepare",
            Self::CurrentFactRecheck => "current-fact-recheck",
            Self::BeforeLinearization => "before-linearization",
            Self::AfterLinearization => "after-linearization",
            Self::DurableReadBack => "durable-read-back",
            Self::Finish => "finish",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConcurrentScheduleStep {
    pub position: u32,
    pub operation_id: String,
    pub mutation: WorldMutationKind,
    pub expected_generation: u64,
    pub pre_state_ref: String,
    pub interleaving: InterleavingPoint,
    pub node_id: String,
    pub node_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConcurrentSchedule {
    pub schedule_id: String,
    pub mutation: WorldMutationKind,
    pub steps: Vec<ConcurrentScheduleStep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldFaultProfile {
    pub schema: &'static str,
    pub profile_name: String,
    pub source_revision: String,
    pub inventory_ref: String,
    pub adapters: Vec<WorldFaultAdapterBinding>,
    pub limits: WorldFaultLimits,
    pub cases: Vec<WorldFaultCase>,
    pub schedules: Vec<ConcurrentSchedule>,
}
