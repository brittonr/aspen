use std::collections::BTreeSet;

pub const JETPACK_ARTIFACT_REVISION: &str = "c03e318ec355b11edd42aac56c68d0765f88d1d2";
pub const JETPACK_ARTIFACT_SOURCE: &str = "https://github.com/stonysystems/jetpack";
pub const JETPACK_PAPER: &str = "Jetpack: Consensus Made Generally Fast, OSDI 2026";
pub const CRASH_FAULT_MODEL: &str = "crash-fault";
pub const MODEL_ONLY_CLAIM: &str = "pure-model-only";
pub const THREE_REPLICA_PROFILE: &str = "three-replica-fastpath-hazards-v1";
pub const FIVE_REPLICA_PROFILE: &str = "five-replica-fastpath-hazards-v1";

const MIN_NODE_COUNT: usize = 3;
const THREE_NODE_COUNT: usize = 3;
const FIVE_NODE_COUNT: usize = 5;
const QUORUM_DIVISOR: usize = 2;
const SUPERQUORUM_NUMERATOR: usize = 3;
const SUPERQUORUM_DENOMINATOR: usize = 4;
const QUORUM_ROUND_UP: usize = 1;
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
    pub paper: String,
    pub artifact_source: String,
    pub artifact_revision: String,
    pub artifact_license: String,
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
    pub profile_id: String,
    pub source: SourceCohort,
    pub base_model_ref: String,
    pub conflict_contract_ref: String,
    pub fault_model: String,
    pub node_count: usize,
    pub active_proposers: BTreeSet<String>,
    pub max_commands: usize,
    pub max_keys: usize,
    pub max_views: usize,
    pub max_steps: usize,
    pub base_ordering: BaseOrderingPrerequisites,
    pub selection: SelectionMode,
    pub claim_profile: String,
    pub invariant_names: BTreeSet<String>,
    pub non_claims: BTreeSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DerivedQuorums {
    pub majority: usize,
    pub superquorum: usize,
    pub tolerated_failures: usize,
    pub fast_path_requires_every_replica: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileAdmission {
    pub quorums: Option<DerivedQuorums>,
    pub issues: Vec<ProfileIssue>,
}

impl ProfileAdmission {
    pub fn is_admitted(&self) -> bool {
        self.issues.is_empty() && self.quorums.is_some()
    }
}

// r[impl molten.consensus.fast_path_model.stable_view]
pub fn derive_quorums(node_count: usize) -> Option<DerivedQuorums> {
    if node_count < MIN_NODE_COUNT {
        return None;
    }
    let majority = node_count.checked_div(QUORUM_DIVISOR)?.checked_add(QUORUM_ROUND_UP)?;
    let scaled = node_count.checked_mul(SUPERQUORUM_NUMERATOR)?;
    let superquorum = scaled.checked_div(SUPERQUORUM_DENOMINATOR)?.checked_add(QUORUM_ROUND_UP)?;
    let tolerated_failures = node_count.checked_sub(QUORUM_ROUND_UP)?.checked_div(QUORUM_DIVISOR)?;
    if majority > node_count || superquorum > node_count || superquorum < majority {
        return None;
    }
    Some(DerivedQuorums {
        majority,
        superquorum,
        tolerated_failures,
        fast_path_requires_every_replica: superquorum == node_count,
    })
}

// r[impl molten.consensus.fast_path_model.profile]
// r[impl molten.consensus.fast_path_model.base_prerequisites]
// r[impl molten.consensus.fast_path_model.nonclaims]
pub fn validate_profile(profile: &FastPathModelProfile) -> ProfileAdmission {
    let mut issues = Vec::new();
    validate_text(&profile.profile_id, "profile id", &mut issues);
    validate_text(&profile.base_model_ref, "base model ref", &mut issues);
    validate_text(&profile.conflict_contract_ref, "conflict contract ref", &mut issues);
    validate_source(&profile.source, &mut issues);
    if profile.fault_model != CRASH_FAULT_MODEL {
        issues.push(ProfileIssue::UnsupportedFaultModel);
    }
    if !matches!(profile.node_count, THREE_NODE_COUNT | FIVE_NODE_COUNT) {
        issues.push(ProfileIssue::UnsupportedNodeCount);
    }
    validate_bound(profile.max_commands, MAX_MODEL_COMMANDS, "commands", &mut issues);
    validate_bound(profile.max_keys, MAX_MODEL_KEYS, "keys", &mut issues);
    validate_bound(profile.max_views, MAX_MODEL_VIEWS, "views", &mut issues);
    validate_bound(profile.max_steps, MAX_MODEL_STEPS, "steps", &mut issues);
    validate_ordering(&profile.base_ordering, &mut issues);
    validate_claims(profile, &mut issues);
    let quorums = derive_quorums(profile.node_count);
    if quorums.is_none() {
        issues.push(ProfileIssue::ImpossibleQuorum);
    }
    issues.sort();
    issues.dedup();
    ProfileAdmission { quorums, issues }
}

fn validate_text(value: &str, field: &'static str, issues: &mut Vec<ProfileIssue>) {
    if value.trim().is_empty() {
        issues.push(ProfileIssue::EmptyField(field));
    }
}

fn validate_bound(value: usize, maximum: usize, field: &'static str, issues: &mut Vec<ProfileIssue>) {
    if value == 0 || value > maximum {
        issues.push(ProfileIssue::BoundExceeded(field));
    }
}

fn validate_source(source: &SourceCohort, issues: &mut Vec<ProfileIssue>) {
    if source.paper != JETPACK_PAPER
        || source.artifact_source != JETPACK_ARTIFACT_SOURCE
        || source.artifact_revision != JETPACK_ARTIFACT_REVISION
        || source.artifact_license != "MIT"
    {
        issues.push(ProfileIssue::UnknownReference);
    }
}

fn validate_ordering(ordering: &BaseOrderingPrerequisites, issues: &mut Vec<ProfileIssue>) {
    if !ordering.execution_order_preserved {
        issues.push(ProfileIssue::BaseExecutionReorders);
    }
    if !ordering.proposal_order_preserved {
        issues.push(ProfileIssue::BaseProposalReorders);
    }
    if !ordering.receive_order_preserved && !ordering.acknowledgement_waits_for_proposal_evidence {
        issues.push(ProfileIssue::BaseReceiveReordersWithoutEvidence);
    }
}

fn validate_claims(profile: &FastPathModelProfile, issues: &mut Vec<ProfileIssue>) {
    match profile.selection {
        SelectionMode::Live => issues.push(ProfileIssue::LiveSelectionDenied),
        SelectionMode::Production => issues.push(ProfileIssue::ProductionSelectionDenied),
        SelectionMode::PureModel | SelectionMode::DeterministicSimulation => {}
    }
    if profile.claim_profile != MODEL_ONLY_CLAIM {
        issues.push(ProfileIssue::UnsupportedClaimProfile);
    }
    for &required in required_invariants() {
        if !profile.invariant_names.contains(required) {
            issues.push(ProfileIssue::MissingInvariant);
        }
    }
    for &required in required_non_claims() {
        if !profile.non_claims.contains(required) {
            issues.push(ProfileIssue::MissingNonClaim);
        }
    }
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
