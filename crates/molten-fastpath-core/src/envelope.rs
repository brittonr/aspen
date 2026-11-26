//! The model profile: source cohort, bounds, base-ordering prerequisites, claims, and quorums.

pub const JETPACK_ARTIFACT_REVISION: &str = "c03e318ec355b11edd42aac56c68d0765f88d1d2";
pub const JETPACK_ARTIFACT_SOURCE: &str = "https://github.com/stonysystems/jetpack";
pub const JETPACK_PAPER: &str = "Jetpack: Consensus Made Generally Fast, OSDI 2026";
pub const CRASH_FAULT_MODEL: &str = "crash-fault";
pub const MODEL_ONLY_CLAIM: &str = "pure-model-only";
pub const THREE_REPLICA_PROFILE: &str = "three-replica-fastpath-hazards-v1";
pub const FIVE_REPLICA_PROFILE: &str = "five-replica-fastpath-hazards-v1";

const MIN_NODE_COUNT: u32 = 3;
const THREE_NODE_COUNT: usize = 3;
const FIVE_NODE_COUNT: usize = 5;
const QUORUM_DIVISOR: u64 = 2;
const SUPERQUORUM_NUMERATOR: u64 = 3;
const SUPERQUORUM_DENOMINATOR: u64 = 4;
const QUORUM_ROUND_UP: u64 = 1;
const MAX_MODEL_COMMANDS: usize = 16;
const MAX_MODEL_KEYS: usize = 8;
const MAX_MODEL_VIEWS: usize = 8;
const MAX_MODEL_STEPS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode {
    PureModel,
    DeterministicSimulation,
    Live,
    Production,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceCohort {
    pub paper: alloc::string::String,
    pub artifact_source: alloc::string::String,
    pub artifact_revision: alloc::string::String,
    pub artifact_license: alloc::string::String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseOrderingPrerequisites {
    pub receive_order_preserved: bool,
    pub proposal_order_preserved: bool,
    pub execution_order_preserved: bool,
    pub acknowledgement_waits_for_proposal_evidence: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FastPathModelProfile {
    pub profile_id: alloc::string::String,
    pub source: SourceCohort,
    pub base_model_ref: alloc::string::String,
    pub conflict_contract_ref: alloc::string::String,
    pub fault_model: alloc::string::String,
    pub node_count: usize,
    pub active_proposers: alloc::collections::BTreeSet<alloc::string::String>,
    pub max_commands: usize,
    pub max_keys: usize,
    pub max_views: usize,
    pub max_steps: usize,
    pub base_ordering: BaseOrderingPrerequisites,
    pub selection: SelectionMode,
    pub claim_profile: alloc::string::String,
    pub invariant_names: alloc::collections::BTreeSet<alloc::string::String>,
    pub non_claims: alloc::collections::BTreeSet<alloc::string::String>,
}

/// Quorum sizes are compared with collection lengths, so they keep the host index type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DerivedQuorums {
    pub majority: usize,
    pub superquorum: usize,
    pub tolerated_failures: usize,
    pub fast_path_requires_every_replica: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProfileIssue {
    BaseExecutionReorders,
    BaseProposalReorders,
    BaseReceiveReordersWithoutEvidence,
    BoundExceeded(&'static str),
    EmptyField(&'static str),
    ImpossibleQuorum,
    LiveSelectionDenied,
    MissingInvariant,
    MissingNonClaim,
    ProductionSelectionDenied,
    UnknownReference,
    UnsupportedFaultModel,
    UnsupportedNodeCount,
    UnsupportedClaimProfile,
}

impl core::fmt::Display for ProfileIssue {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let code = match self {
            Self::BoundExceeded(field) => return write!(formatter, "bound-exceeded:{field}"),
            Self::EmptyField(field) => return write!(formatter, "empty-field:{field}"),
            Self::BaseExecutionReorders => "base-execution-reorders",
            Self::BaseProposalReorders => "base-proposal-reorders",
            Self::BaseReceiveReordersWithoutEvidence => "base-receive-reorders-without-evidence",
            Self::ImpossibleQuorum => "impossible-quorum",
            Self::LiveSelectionDenied => "live-selection-denied",
            Self::MissingInvariant => "missing-invariant",
            Self::MissingNonClaim => "missing-non-claim",
            Self::ProductionSelectionDenied => "production-selection-denied",
            Self::UnknownReference => "unknown-reference",
            Self::UnsupportedFaultModel => "unsupported-fault-model",
            Self::UnsupportedNodeCount => "unsupported-node-count",
            Self::UnsupportedClaimProfile => "unsupported-claim-profile",
        };
        formatter.write_str(code)
    }
}

impl core::error::Error for ProfileIssue {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileAdmission {
    pub quorums: Option<DerivedQuorums>,
    pub issues: alloc::vec::Vec<ProfileIssue>,
}

impl ProfileAdmission {
    pub fn is_admitted(&self) -> bool {
        self.issues.is_empty() && self.quorums.is_some()
    }
}

// r[impl molten.consensus.fast_path_model.stable_view]
/// Quorums are computed in `u64`, so `3 * node_count` cannot overflow for any `u32` input; only the
/// final sizes are narrowed to the host index type.
pub fn derive_quorums(node_count: u32) -> Option<DerivedQuorums> {
    if node_count < MIN_NODE_COUNT {
        return None;
    }
    let nodes = u64::from(node_count);
    let majority = nodes.checked_div(QUORUM_DIVISOR)?.checked_add(QUORUM_ROUND_UP)?;
    let scaled = nodes.checked_mul(SUPERQUORUM_NUMERATOR)?;
    let superquorum = scaled.checked_div(SUPERQUORUM_DENOMINATOR)?.checked_add(QUORUM_ROUND_UP)?;
    let tolerated_failures = nodes.checked_sub(QUORUM_ROUND_UP)?.checked_div(QUORUM_DIVISOR)?;
    if majority > nodes || superquorum > nodes || superquorum < majority {
        return None;
    }
    Some(DerivedQuorums {
        majority: usize::try_from(majority).ok()?,
        superquorum: usize::try_from(superquorum).ok()?,
        tolerated_failures: usize::try_from(tolerated_failures).ok()?,
        fast_path_requires_every_replica: superquorum == nodes,
    })
}

// r[impl molten.consensus.fast_path_model.profile]
// r[impl molten.consensus.fast_path_model.base_prerequisites]
// r[impl molten.consensus.fast_path_model.nonclaims]
pub fn validate_profile(profile: &FastPathModelProfile) -> ProfileAdmission {
    let quorums = u32::try_from(profile.node_count).ok().and_then(derive_quorums);
    let scalar_issues = [
        text_issue(&profile.profile_id, "profile id"),
        text_issue(&profile.base_model_ref, "base model ref"),
        text_issue(&profile.conflict_contract_ref, "conflict contract ref"),
        source_issue(&profile.source),
        (profile.fault_model != CRASH_FAULT_MODEL).then_some(ProfileIssue::UnsupportedFaultModel),
        (!matches!(profile.node_count, THREE_NODE_COUNT | FIVE_NODE_COUNT))
            .then_some(ProfileIssue::UnsupportedNodeCount),
        bound_issue(profile.max_commands, MAX_MODEL_COMMANDS, "commands"),
        bound_issue(profile.max_keys, MAX_MODEL_KEYS, "keys"),
        bound_issue(profile.max_views, MAX_MODEL_VIEWS, "views"),
        bound_issue(profile.max_steps, MAX_MODEL_STEPS, "steps"),
        selection_issue(profile.selection),
        quorums.is_none().then_some(ProfileIssue::ImpossibleQuorum),
    ];
    let mut issues: alloc::vec::Vec<ProfileIssue> = scalar_issues
        .into_iter()
        .chain(ordering_issues(&profile.base_ordering))
        .chain(claim_issues(profile))
        .flatten()
        .collect();
    issues.sort();
    issues.dedup();
    ProfileAdmission { quorums, issues }
}

fn text_issue(value: &str, field: &'static str) -> Option<ProfileIssue> {
    value.trim().is_empty().then_some(ProfileIssue::EmptyField(field))
}

fn bound_issue(value: usize, maximum: usize, field: &'static str) -> Option<ProfileIssue> {
    (value == 0 || value > maximum).then_some(ProfileIssue::BoundExceeded(field))
}

fn source_issue(source: &SourceCohort) -> Option<ProfileIssue> {
    let is_known = source.paper == JETPACK_PAPER
        && source.artifact_source == JETPACK_ARTIFACT_SOURCE
        && source.artifact_revision == JETPACK_ARTIFACT_REVISION
        && source.artifact_license == "MIT";
    (!is_known).then_some(ProfileIssue::UnknownReference)
}

fn ordering_issues(ordering: &BaseOrderingPrerequisites) -> [Option<ProfileIssue>; 3] {
    let is_receive_unordered =
        !ordering.receive_order_preserved && !ordering.acknowledgement_waits_for_proposal_evidence;
    [
        (!ordering.execution_order_preserved).then_some(ProfileIssue::BaseExecutionReorders),
        (!ordering.proposal_order_preserved).then_some(ProfileIssue::BaseProposalReorders),
        is_receive_unordered.then_some(ProfileIssue::BaseReceiveReordersWithoutEvidence),
    ]
}

fn selection_issue(selection: SelectionMode) -> Option<ProfileIssue> {
    match selection {
        SelectionMode::Live => Some(ProfileIssue::LiveSelectionDenied),
        SelectionMode::Production => Some(ProfileIssue::ProductionSelectionDenied),
        SelectionMode::PureModel | SelectionMode::DeterministicSimulation => None,
    }
}

fn claim_issues(profile: &FastPathModelProfile) -> [Option<ProfileIssue>; 3] {
    let is_missing_invariant =
        required_invariants().iter().any(|required| !profile.invariant_names.contains(*required));
    let is_missing_non_claim = required_non_claims().iter().any(|required| !profile.non_claims.contains(*required));
    [
        (profile.claim_profile != MODEL_ONLY_CLAIM).then_some(ProfileIssue::UnsupportedClaimProfile),
        is_missing_invariant.then_some(ProfileIssue::MissingInvariant),
        is_missing_non_claim.then_some(ProfileIssue::MissingNonClaim),
    ]
}

pub fn required_invariants() -> &'static [&'static str] {
    &[
        "recoverability",
        "no-conflicting-predecessor",
        "committed-order-agreement",
        "execution-order-agreement",
        "linearizable-conflict-order",
        "at-most-once-application",
    ]
}

pub fn required_non_claims() -> &'static [&'static str] {
    &[
        "no-live-engine-proof",
        "no-transport-proof",
        "no-durability-proof",
        "no-performance-proof",
        "no-production-linearizability-proof",
        "no-release-readiness",
    ]
}
