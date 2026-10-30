use std::collections;

use crate::world_head;

pub const MAX_WITNESSES: usize = 16;
pub const MAX_WITNESS_LABEL_BYTES: usize = 128;
pub const MAX_WITNESS_CONTRACT_BYTES: usize = 512;

/// Versioned assurance is independent of authentication and branch authority.
// r[impl molten.world_witness.profiles]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AssuranceProfile {
    LocalGenerationV1,
    IndependentWitnessV1,
}

impl AssuranceProfile {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LocalGenerationV1 => "local-generation-v1",
            Self::IndependentWitnessV1 => "independent-witness-v1",
        }
    }
}

/// Exact reviewed consumer contract; this does not select global trust roots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderContract {
    pub provider_id: String,
    pub contract_id: String,
    pub source_revision: String,
    pub license: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessPolicy {
    pub required_profile: AssuranceProfile,
    pub provider: ProviderContract,
    pub allowed_witnesses: collections::BTreeSet<String>,
    pub minimum_quorum: usize,
    pub maximum_age: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessSubject {
    pub branch_id: world_head::WorldBranchId,
    pub claim_ref: world_head::WorldHeadClaimRef,
    pub generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedCheckpoint {
    pub provider_id: String,
    pub checkpoint_id: String,
    pub revision: u64,
    pub signature_verified: bool,
    pub observed_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighestWitnessState {
    pub subject: WitnessSubject,
    pub checkpoint: SignedCheckpoint,
    pub consistency_predecessor: Option<String>,
    pub quorum: collections::BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetainedWitnessState {
    /// Explicit initial enrollment, not permission to recover missing state.
    Empty,
    Present(HighestWitnessState),
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppendObservation {
    pub operation_id: String,
    pub subject: WitnessSubject,
    pub accepted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InclusionObservation {
    pub subject: WitnessSubject,
    pub checkpoint_id: String,
    pub verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsistencyObservation {
    pub predecessor: Option<String>,
    pub checkpoint_id: String,
    pub verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuorumObservation {
    pub checkpoint_id: String,
    pub witnesses: collections::BTreeSet<String>,
    pub verified: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentHeadPhase {
    Reserved,
    Committed,
}

/// An authenticated independent current-head read, never mere log inclusion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentHeadObservation {
    pub subject: WitnessSubject,
    pub checkpoint_id: String,
    pub verified: bool,
    pub phase: CurrentHeadPhase,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableWitnessObservation {
    pub contract: ProviderContract,
    pub append: AppendObservation,
    pub inclusion: InclusionObservation,
    pub consistency: ConsistencyObservation,
    pub checkpoint: SignedCheckpoint,
    pub quorum: QuorumObservation,
    pub current_head: CurrentHeadObservation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WitnessObservation {
    Available(Box<AvailableWitnessObservation>),
    Unavailable {
        append_may_have_occurred: bool,
    },
    Fork {
        provider_id: String,
        checkpoint_ids: collections::BTreeSet<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentnessInput {
    pub selected_profile: AssuranceProfile,
    pub policy: WitnessPolicy,
    pub operation_id: String,
    pub subject: WitnessSubject,
    pub local_generation: u64,
    pub retained: RetainedWitnessState,
    pub observation: Option<WitnessObservation>,
    pub now: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentnessAdmission {
    pub profile: AssuranceProfile,
    pub highest: Option<HighestWitnessState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WitnessIssue {
    BoundsExceeded,
    ProfileDowngrade,
    StaleGeneration,
    MissingHighestState,
    InvalidContract,
    ProviderSubstitution,
    DomainSubstitution,
    OperationSubstitution,
    InvalidCheckpoint,
    StaleCheckpoint,
    InvalidInclusion,
    FailedConsistency,
    InsufficientQuorum,
    Fork,
    Unavailable,
    UncertainAppend,
    MissingCurrentness,
    InvalidStage,
    AuthorityDenied,
    LocalConflict,
}
