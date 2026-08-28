use super::*;
use crate::fabric_simulation::EligibleChoice;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmissionObservation {
    NotSubmitted,
    PossiblySubmitted,
    DurablySubmitted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseObservation {
    NotExpected,
    Received,
    Lost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurableReadBackStatus {
    Prior,
    Applied,
    Missing,
    Corrupt,
    Contradictory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableReadBack {
    pub status: DurableReadBackStatus,
    pub state_ref: Option<String>,
    pub record_ref: Option<String>,
    pub observed_generation: Option<u64>,
    pub independent_witness: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldOperationObservation {
    pub case_id: String,
    pub operation_id: String,
    pub phase: FaultPhase,
    pub submission: SubmissionObservation,
    pub response: ResponseObservation,
    pub read_back: DurableReadBack,
    pub owner_decision: RecoveryClass,
    pub whole_store_rollback: bool,
    pub cleanup_authorized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConcurrentOutcome {
    Applied,
    AlreadyComplete,
    Stale,
    Superseded,
    Conflict,
    Uncertain,
    Denied,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConcurrentOperationObservation {
    pub operation_id: String,
    pub mutation: WorldMutationKind,
    pub expected_generation: u64,
    pub pre_state_ref: String,
    pub outcome: ConcurrentOutcome,
    pub effect_release_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConformanceDisposition {
    Passed,
    Failed,
    Unsupported,
}

impl ConformanceDisposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Unsupported => "unsupported",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldFaultConformanceResult {
    pub case_id: String,
    pub mutation: WorldMutationKind,
    pub phase: FaultPhase,
    pub expected_decision: RecoveryClass,
    pub observed_decision: RecoveryClass,
    pub observation: WorldOperationObservation,
    pub disposition: ConformanceDisposition,
    pub diagnostics: Vec<WorldFaultIssue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnsupportedReason {
    IndependentWitnessOwnerUnavailable,
    PhysicalFailureProfileNotExercised,
}

impl UnsupportedReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::IndependentWitnessOwnerUnavailable => "independent-witness-owner-unavailable",
            Self::PhysicalFailureProfileNotExercised => "physical-failure-profile-not-exercised",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedConformanceRow {
    pub mutation: WorldMutationKind,
    pub case_id: String,
    pub reason: UnsupportedReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldFaultNonClaim {
    UniversalCrashSafety,
    PhysicalPowerLossCoverage,
    StorageCorrectness,
    ReleaseEligibility,
}

impl WorldFaultNonClaim {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UniversalCrashSafety => "does-not-prove-universal-crash-safety",
            Self::PhysicalPowerLossCoverage => "does-not-prove-physical-power-loss-coverage",
            Self::StorageCorrectness => "does-not-prove-storage-correctness",
            Self::ReleaseEligibility => "does-not-establish-release-eligibility",
        }
    }
}

pub const REQUIRED_WORLD_FAULT_NON_CLAIMS: [WorldFaultNonClaim; REQUIRED_FAULT_NON_CLAIM_COUNT] = [
    WorldFaultNonClaim::UniversalCrashSafety,
    WorldFaultNonClaim::PhysicalPowerLossCoverage,
    WorldFaultNonClaim::StorageCorrectness,
    WorldFaultNonClaim::ReleaseEligibility,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConcurrentScheduleResult {
    pub schedule_id: String,
    pub observations: Vec<ConcurrentOperationObservation>,
    pub scheduler_choices: Vec<EligibleChoice>,
    pub disposition: ConformanceDisposition,
    pub diagnostics: Vec<WorldFaultIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldFaultConformanceReceipt {
    pub schema: &'static str,
    pub source_revision: String,
    pub inventory_ref: String,
    pub profile_ref: String,
    pub adapter_refs: Vec<String>,
    pub schedule_refs: Vec<String>,
    pub limits: WorldFaultLimits,
    pub results: Vec<WorldFaultConformanceResult>,
    pub schedules: Vec<ConcurrentScheduleResult>,
    pub unsupported_rows: Vec<UnsupportedConformanceRow>,
    pub decision: ConformanceDisposition,
    pub mutation_authorized_by_evidence: bool,
    pub cleanup_authorized_by_evidence: bool,
    pub non_claims: Vec<WorldFaultNonClaim>,
}
