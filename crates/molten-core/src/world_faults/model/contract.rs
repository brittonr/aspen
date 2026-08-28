use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FaultPhase {
    Uninterrupted,
    BeforeSubmit,
    AfterPossibleSubmit,
    AfterDurableWrite,
    BeforeResponse,
    LostResponse,
    ProcessRestart,
    RecoveryReadBack,
}

impl FaultPhase {
    pub const ALL: [Self; REQUIRED_FAULT_PHASE_COUNT] = [
        Self::Uninterrupted,
        Self::BeforeSubmit,
        Self::AfterPossibleSubmit,
        Self::AfterDurableWrite,
        Self::BeforeResponse,
        Self::LostResponse,
        Self::ProcessRestart,
        Self::RecoveryReadBack,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Uninterrupted => "uninterrupted",
            Self::BeforeSubmit => "before-submit",
            Self::AfterPossibleSubmit => "after-possible-submit",
            Self::AfterDurableWrite => "after-durable-write",
            Self::BeforeResponse => "before-response",
            Self::LostResponse => "lost-response",
            Self::ProcessRestart => "process-restart",
            Self::RecoveryReadBack => "recovery-read-back",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RequiredFailureCase {
    TornRecord,
    LostResponse,
    DuplicateSubmission,
    StalePlan,
    MissingObject,
    CorruptRecord,
    GenerationRace,
    EffectUncertainty,
    RollbackWithoutWitness,
    UnsafeCleanup,
    ContradictoryObservation,
    FaultCoverageOverclaim,
}

impl RequiredFailureCase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TornRecord => "torn-record",
            Self::LostResponse => "lost-response",
            Self::DuplicateSubmission => "duplicate-submission",
            Self::StalePlan => "stale-plan",
            Self::MissingObject => "missing-object",
            Self::CorruptRecord => "corrupt-record",
            Self::GenerationRace => "generation-race",
            Self::EffectUncertainty => "effect-uncertainty",
            Self::RollbackWithoutWitness => "rollback-without-witness",
            Self::UnsafeCleanup => "unsafe-cleanup",
            Self::ContradictoryObservation => "contradictory-observation",
            Self::FaultCoverageOverclaim => "fault-coverage-overclaim",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldMutationContract {
    pub mutation: WorldMutationKind,
    pub owner: WorldMutationOwner,
    pub operation_domain: OperationIdentityDomain,
    pub expected_pre_state: &'static str,
    pub effects: Vec<WorldMutationEffect>,
    pub linearization_point: LinearizationPoint,
    pub durable_record: DurableRecordKind,
    pub uncertain_window: UncertainWindow,
    pub reconciliation_entry: ReconciliationEntry,
    pub required_phases: Vec<FaultPhase>,
    pub required_cases: Vec<RequiredFailureCase>,
    pub support: MutationSupport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldMutationInventory {
    pub schema: &'static str,
    pub version: u32,
    pub rows: Vec<WorldMutationContract>,
}
