use super::super::*;
use super::*;

/// The non-claims every replay boundary summary carries.
pub const REPLAY_BOUNDARY_NON_CLAIMS: &[&str] = &[
    "logical replay holds only for the named profile, trace, horizon, and cohorts",
    "replay does not prove arbitrary process determinism",
    "replay does not prove kernel or native-process replay",
    "a recorded effect observation does not prove external effect completion",
    "replay does not prove host security",
    "replay grants no authority",
    "replay does not establish release eligibility",
    "opaque diagnostics are detached and establish no semantic equivalence",
];

/// A claim a replay receipt makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReplayBoundaryClaim {
    /// The one admitted claim: the selected adapters reproduced the recorded
    /// observations and expected transitions for this profile and horizon.
    LogicalReplayForProfile,
    ArbitraryProcessDeterminism,
    KernelReplay,
    ExternalEffectCompletion,
    HostSecurity,
    Authority,
    ReleaseEligibility,
    OpaqueSemanticEquivalence,
}

impl ReplayBoundaryClaim {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LogicalReplayForProfile => "logical-replay-for-profile",
            Self::ArbitraryProcessDeterminism => "arbitrary-process-determinism",
            Self::KernelReplay => "kernel-replay",
            Self::ExternalEffectCompletion => "external-effect-completion",
            Self::HostSecurity => "host-security",
            Self::Authority => "authority",
            Self::ReleaseEligibility => "release-eligibility",
            Self::OpaqueSemanticEquivalence => "opaque-semantic-equivalence",
        }
    }
}

/// How recorded values of sensitive sources were stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReplayRedaction {
    /// No inventory source is sensitive.
    NoSensitiveSources,
    /// Every sensitive observation is protected; none is public plaintext.
    SensitiveProtected,
}

/// The boundary facts a replay receipt states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayBoundarySummary {
    pub inventory_ref: String,
    pub ledger_ref: String,
    pub deterministic_sources: u32,
    pub simulated_sources: u32,
    pub recorded_sources: u32,
    pub recorded_observations: u32,
    /// The last replayed slot, as (step, logical position).
    pub horizon: Option<(u64, u64)>,
    pub adapter_cohort_ref: String,
    pub runtime_cohort_ref: Option<String>,
    /// Effect observations returned sealed instead of executed.
    pub sealed_effects: u32,
    /// Attempts to execute an original effect during replay. Success requires zero.
    pub repeated_effect_attempts: u32,
    pub opaque_diagnostic_refs: Vec<String>,
    pub redaction: ReplayRedaction,
    pub claims: Vec<ReplayBoundaryClaim>,
    pub non_claims: Vec<String>,
}

/// Summarize an admitted boundary for a receipt that claims only logical
/// replay of this profile.
// r[impl molten.world_replay_boundary.claims]
pub fn summarize_replay_boundary(
    admission: &ReplayObservationAdmission,
    boundary: &WorldReplayObservationBoundary,
    profile: &WorldReplayProfile,
    repeated_effect_attempts: u32,
) -> Result<ReplayBoundarySummary, ReplayBoundaryBlocker> {
    let has_sensitive = boundary.inventory.sources.iter().any(|source| source.is_sensitive);
    let sealed_effects = u32::try_from(admission.sealed_effect_refs.len())
        .map_err(|_| ReplayBoundaryBlocker::LimitExceeded("sealed"))?;
    Ok(ReplayBoundarySummary {
        inventory_ref: admission.inventory_ref.clone(),
        ledger_ref: admission.ledger_ref.clone(),
        deterministic_sources: admission.deterministic_sources,
        simulated_sources: admission.simulated_sources,
        recorded_sources: admission.recorded_sources,
        recorded_observations: admission.recorded_observations,
        horizon: admission.horizon,
        adapter_cohort_ref: boundary.inventory.adapter_cohort_ref.clone(),
        runtime_cohort_ref: profile.cohort_ref.as_ref().map(|cohort| cohort.as_str().to_string()),
        sealed_effects,
        repeated_effect_attempts,
        opaque_diagnostic_refs: admission.opaque_diagnostic_refs.clone(),
        redaction: if has_sensitive {
            ReplayRedaction::SensitiveProtected
        } else {
            ReplayRedaction::NoSensitiveSources
        },
        claims: vec![ReplayBoundaryClaim::LogicalReplayForProfile],
        non_claims: REPLAY_BOUNDARY_NON_CLAIMS.iter().map(ToString::to_string).collect(),
    })
}

/// Reject a summary that claims more than logical replay of its profile,
/// drops a non-claim, or claims success after an effect was attempted.
// r[impl molten.world_replay_boundary.claims]
pub fn validate_replay_boundary_claims(summary: &ReplayBoundarySummary) -> Vec<ReplayBoundaryBlocker> {
    let mut blockers = summary
        .claims
        .iter()
        .filter(|claim| **claim != ReplayBoundaryClaim::LogicalReplayForProfile)
        .map(|claim| ReplayBoundaryBlocker::Overclaim(claim.as_str()))
        .collect::<Vec<_>>();
    let is_complete = REPLAY_BOUNDARY_NON_CLAIMS
        .iter()
        .all(|non_claim| summary.non_claims.iter().any(|stated| stated == non_claim));
    if !is_complete {
        blockers.push(ReplayBoundaryBlocker::InvalidRecord("non-claims"));
    }
    let is_claiming_replay = summary.claims.contains(&ReplayBoundaryClaim::LogicalReplayForProfile);
    if is_claiming_replay && summary.repeated_effect_attempts > 0 {
        blockers.push(ReplayBoundaryBlocker::Overclaim("replay-after-repeated-effect"));
    }
    blockers
}
