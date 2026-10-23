use std::collections::BTreeSet;

use super::*;

const MAX_RECORDS: u32 = 65_536;
const MAX_STATE_BYTES: u32 = 1_048_576;
const MAX_TOTAL_BYTES: u64 = 268_435_456;
const MAX_PARTICIPANTS: u32 = 64;
const MAX_COUNTERS: u32 = 64;
const MAX_PROGRESS_HORIZON: u64 = 1_000_000;

/// The safety level a result belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SafetyLevel {
    /// One participant's transition from its previous projection.
    Local,
    /// Two participants at one shared logical position.
    Pairwise,
    /// Every participant of a service at one logical position.
    Cohort,
    /// A recovered participant against its own pre-recovery projections.
    Durability,
}

impl SafetyLevel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Pairwise => "pairwise",
            Self::Cohort => "cohort",
            Self::Durability => "durability",
        }
    }
}

/// Which logical position an extension uses for a service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalPositionKind {
    Version,
    NextOffset,
    TransitionCount,
}

/// Whether physical bytes take part in agreement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalLayout {
    /// Only semantic projections are compared.
    SemanticOnly,
    /// The adapter promises a canonical physical layout; physical refs must also match.
    CanonicalPhysical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletenessMode {
    /// Every source sequence of every observed participant must be present.
    Lossless,
    /// Sources may be sampled; gaps do not mark the cohort incomplete.
    Sampled,
}

/// The projection fields a novelty identity is computed over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NoveltyField {
    Service,
    Participant,
    TransitionClass,
    LogicalPosition,
    State,
    Durability,
    Progress,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OracleIdentity {
    pub oracle_id: String,
    pub version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceProfile {
    pub service: String,
    pub logical_position: LogicalPositionKind,
    pub safety: Vec<SafetyLevel>,
    pub physical_layout: PhysicalLayout,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticipantProfile {
    pub participant: String,
    pub service: String,
    pub is_required: bool,
}

/// The stabilization facts a liveness evaluation requires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LivenessPreconditions {
    pub requires_readiness: bool,
    pub requires_membership: bool,
    pub disruptive_faults_block: bool,
    pub requires_durability: bool,
    pub virtual_progress_horizon: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CounterSchema {
    pub name: String,
    pub resets_on_generation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectionBounds {
    pub max_records: u32,
    pub max_state_bytes: u32,
    pub max_total_bytes: u64,
    pub max_participants: u32,
    pub max_counters: u32,
}

/// The typed oracle profile exported from `config/protocol-oracle/profile.ncl`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolOracleProfile {
    pub profile_id: String,
    pub oracle: OracleIdentity,
    pub oracle_ref: String,
    pub services: Vec<ServiceProfile>,
    pub participants: Vec<ParticipantProfile>,
    pub completeness: CompletenessMode,
    pub liveness: LivenessPreconditions,
    pub novelty_fields: Vec<NoveltyField>,
    pub counters: Vec<CounterSchema>,
    pub bounds: ProjectionBounds,
    pub non_claims: Vec<String>,
}

impl ProtocolOracleProfile {
    pub fn service(&self, service: &str) -> Option<&ServiceProfile> {
        self.services.iter().find(|candidate| candidate.service == service)
    }

    pub fn participant(&self, participant: &str) -> Option<&ParticipantProfile> {
        self.participants.iter().find(|candidate| candidate.participant == participant)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProfileIssue {
    OracleRefMismatch,
    BoundOutOfRange(&'static str),
    DuplicateService(String),
    DuplicateParticipant(String),
    UndeclaredService(String),
    DuplicateCounter(String),
    EmptyField(&'static str),
    MissingNonClaim(&'static str),
}

/// The identity of an oracle: BLAKE3 over `<oracle_id>.v<version>`.
pub fn oracle_ref_of(identity: &OracleIdentity) -> String {
    format!(
        "blake3:{}",
        blake3::hash(format!("{}.v{}", identity.oracle_id, identity.version).as_bytes()).to_hex()
    )
}

/// Validate a profile before any projection is admitted under it.
// r[impl molten.fabric_simulation.protocol_projection]
pub fn validate_oracle_profile(profile: &ProtocolOracleProfile) -> Vec<ProfileIssue> {
    let mut issues = BTreeSet::new();
    if profile.oracle_ref != oracle_ref_of(&profile.oracle) {
        issues.insert(ProfileIssue::OracleRefMismatch);
    }
    validate_bounds(profile, &mut issues);
    for (field, is_empty) in [
        ("services", profile.services.is_empty()),
        ("participants", profile.participants.is_empty()),
        ("novelty-fields", profile.novelty_fields.is_empty()),
        ("counters", profile.counters.is_empty()),
    ] {
        if is_empty {
            issues.insert(ProfileIssue::EmptyField(field));
        }
    }
    let mut services = BTreeSet::new();
    for service in &profile.services {
        if !services.insert(service.service.as_str()) {
            issues.insert(ProfileIssue::DuplicateService(service.service.clone()));
        }
        if service.safety.is_empty() {
            issues.insert(ProfileIssue::EmptyField("safety"));
        }
    }
    let mut participants = BTreeSet::new();
    for participant in &profile.participants {
        if !participants.insert(participant.participant.as_str()) {
            issues.insert(ProfileIssue::DuplicateParticipant(participant.participant.clone()));
        }
        if !services.contains(participant.service.as_str()) {
            issues.insert(ProfileIssue::UndeclaredService(participant.service.clone()));
        }
    }
    let mut counters = BTreeSet::new();
    for counter in &profile.counters {
        if !counters.insert(counter.name.as_str()) {
            issues.insert(ProfileIssue::DuplicateCounter(counter.name.clone()));
        }
    }
    for non_claim in PROTOCOL_ORACLE_NON_CLAIMS {
        if !profile.non_claims.iter().any(|stated| stated == non_claim) {
            issues.insert(ProfileIssue::MissingNonClaim(non_claim));
        }
    }
    issues.into_iter().collect()
}

fn validate_bounds(profile: &ProtocolOracleProfile, issues: &mut BTreeSet<ProfileIssue>) {
    let bounds = &profile.bounds;
    for (name, is_valid) in [
        ("max-records", (1..=MAX_RECORDS).contains(&bounds.max_records)),
        ("max-state-bytes", (1..=MAX_STATE_BYTES).contains(&bounds.max_state_bytes)),
        ("max-total-bytes", (1..=MAX_TOTAL_BYTES).contains(&bounds.max_total_bytes)),
        ("state-within-total", u64::from(bounds.max_state_bytes) <= bounds.max_total_bytes),
        ("max-participants", (1..=MAX_PARTICIPANTS).contains(&bounds.max_participants)),
        ("max-counters", (1..=MAX_COUNTERS).contains(&bounds.max_counters)),
        ("participant-count", fits(profile.participants.len(), bounds.max_participants)),
        ("counter-count", fits(profile.counters.len(), bounds.max_counters)),
        ("progress-horizon", (1..=MAX_PROGRESS_HORIZON).contains(&profile.liveness.virtual_progress_horizon)),
    ] {
        if !is_valid {
            issues.insert(ProfileIssue::BoundOutOfRange(name));
        }
    }
}

fn fits(count: usize, maximum: u32) -> bool {
    u32::try_from(count).is_ok_and(|count| count <= maximum)
}
