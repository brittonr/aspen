use super::super::*;
use super::*;

/// What a recorded observation returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayObservationOutcome {
    /// A result value, identified by content and length.
    Result { result_ref: String, byte_length: u64 },
    /// A typed error class.
    Error { error_class: String },
}

impl ReplayObservationOutcome {
    pub const fn byte_length(&self) -> u64 {
        match self {
            Self::Result { byte_length, .. } => *byte_length,
            Self::Error { .. } => 0,
        }
    }
}

/// The place a transition consumes one recorded observation.
///
/// Slots order by transition step, then by explicit logical position inside
/// the step. Wall-clock arrival never orders a slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayObservationSlot {
    pub step_position: u64,
    pub logical_position: u64,
    pub source_id: String,
    /// The operation or request identifier the transition issued.
    pub operation_id: String,
    /// The canonical identity of the request the source answered.
    pub request_ref: String,
}

impl ReplayObservationSlot {
    /// The replay-stable order key.
    pub const fn order_key(&self) -> (u64, u64) {
        (self.step_position, self.logical_position)
    }

    /// Whether another slot names the same source, operation, and request.
    pub fn binds_same_request(&self, other: &Self) -> bool {
        self.source_id == other.source_id
            && self.operation_id == other.operation_id
            && self.request_ref == other.request_ref
    }
}

/// One sealed recorded observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedReplayObservation {
    pub observation_ref: String,
    pub slot: ReplayObservationSlot,
    pub outcome: ReplayObservationOutcome,
    pub protection: WorldReplayMemberProtection,
    pub protection_profile_ref: String,
    pub adapter_cohort_ref: String,
    /// The wall-clock arrival the recorder saw, kept as an observation only.
    pub arrival_observation: Option<u64>,
}

/// A claim an opaque diagnostic makes about itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OpaqueDiagnosticClaim {
    /// Detached forensic evidence. The only admitted claim.
    DetachedDiagnostic,
    SatisfiesObservation,
    LogicalWorldRoot,
    GrantsAuthority,
    SemanticEquivalence,
}

/// A detached opaque diagnostic, such as a native-process replay descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpaqueReplayDiagnostic {
    pub descriptor_ref: String,
    pub byte_length: u64,
    pub cohort_ref: String,
    pub claims: Vec<OpaqueDiagnosticClaim>,
}

/// The observation ledger of one transition trace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayObservationLedger {
    pub schema: String,
    pub ledger_ref: String,
    pub inventory_ref: String,
    pub trace_ref: String,
    /// Every source the recording reached, deterministic and simulated
    /// sources included, in canonical identifier order.
    pub reached_sources: Vec<String>,
    /// The slots the recorded transitions consumed, in replay order.
    pub slots: Vec<ReplayObservationSlot>,
    /// The recorded observations, in replay order.
    pub observations: Vec<RecordedReplayObservation>,
}

/// The complete observation boundary a replay request carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldReplayObservationBoundary {
    pub inventory: ReplaySourceInventory,
    pub ledger: ReplayObservationLedger,
    pub opaque_diagnostics: Vec<OpaqueReplayDiagnostic>,
}

// r[impl molten.world_replay_boundary.observations]
pub fn identify_recorded_replay_observation(
    observation: &RecordedReplayObservation,
) -> Result<String, WorldReplayIssue> {
    let mut hasher = blake3::Hasher::new_derive_key(REPLAY_OBSERVATION_IDENTITY_DOMAIN);
    update_slot(&mut hasher, &observation.slot)?;
    match &observation.outcome {
        ReplayObservationOutcome::Result {
            result_ref,
            byte_length,
        } => {
            update_text(&mut hasher, "result")?;
            update_text(&mut hasher, result_ref)?;
            update_number(&mut hasher, *byte_length);
        }
        ReplayObservationOutcome::Error { error_class } => {
            update_text(&mut hasher, "error")?;
            update_text(&mut hasher, error_class)?;
        }
    }
    update_text(&mut hasher, observation.protection.as_str())?;
    if let WorldReplayMemberProtection::Ciphertext { descriptor_ref } = &observation.protection {
        update_text(&mut hasher, descriptor_ref)?;
    }
    update_text(&mut hasher, &observation.protection_profile_ref)?;
    update_text(&mut hasher, &observation.adapter_cohort_ref)?;
    update_optional_number(&mut hasher, observation.arrival_observation);
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

// r[impl molten.world_replay_boundary.observations]
pub fn identify_replay_observation_ledger(ledger: &ReplayObservationLedger) -> Result<String, WorldReplayIssue> {
    let mut hasher = blake3::Hasher::new_derive_key(REPLAY_OBSERVATION_LEDGER_IDENTITY_DOMAIN);
    update_text(&mut hasher, &ledger.schema)?;
    update_text(&mut hasher, &ledger.inventory_ref)?;
    update_text(&mut hasher, &ledger.trace_ref)?;
    update_usize(&mut hasher, ledger.reached_sources.len())?;
    for source in &ledger.reached_sources {
        update_text(&mut hasher, source)?;
    }
    update_usize(&mut hasher, ledger.slots.len())?;
    for slot in &ledger.slots {
        update_slot(&mut hasher, slot)?;
    }
    update_usize(&mut hasher, ledger.observations.len())?;
    for observation in &ledger.observations {
        update_text(&mut hasher, &observation.observation_ref)?;
    }
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

fn update_slot(hasher: &mut blake3::Hasher, slot: &ReplayObservationSlot) -> Result<(), WorldReplayIssue> {
    update_number(hasher, slot.step_position);
    update_number(hasher, slot.logical_position);
    update_text(hasher, &slot.source_id)?;
    update_text(hasher, &slot.operation_id)?;
    update_text(hasher, &slot.request_ref)
}

/// One exact reason the observation boundary blocks replay.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReplayBoundaryBlocker {
    InvalidRecord(&'static str),
    InventoryIdentityMismatch,
    LedgerIdentityMismatch,
    DuplicateSource(String),
    NonCanonicalSourceOrder,
    /// An external mutation is declared deterministic or simulated, so replay
    /// would repeat it.
    MutationNotSealed(String),
    /// The inventory belongs to another replay profile.
    ProfileDrift,
    /// The ledger names another inventory or another trace.
    LedgerBindingMismatch(&'static str),
    UnknownSource(String),
    UnsupportedSource(String),
    /// A slot names a source whose handling is not recorded observation.
    UnrecordedHandling(String),
    StepOutsideTrace(u64),
    MissingObservation {
        step_position: u64,
        logical_position: u64,
    },
    ExtraObservation {
        step_position: u64,
        logical_position: u64,
    },
    DuplicateObservation {
        step_position: u64,
        logical_position: u64,
    },
    WrongSource {
        step_position: u64,
        logical_position: u64,
    },
    WrongRequest {
        step_position: u64,
        logical_position: u64,
    },
    OrderingDivergence {
        step_position: u64,
        logical_position: u64,
    },
    ObservationIdentityMismatch(String),
    AdapterDrift(String),
    ProtectionDrift(String),
    PlaintextSecret(String),
    MalformedObservation(String),
    OpaqueCohortMismatch(String),
    OpaqueOverclaim(String),
    /// A receipt claims more than logical replay of its profile.
    Overclaim(&'static str),
    LimitExceeded(&'static str),
}
