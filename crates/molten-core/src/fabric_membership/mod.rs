//! Pure membership, failure-observation, placement, and fencing contracts.
//!
//! Membership snapshots are explicit inputs rather than ambient truth. This
//! module performs no clock, network, persistence, policy, or role effects.

mod transition;

use std::collections::BTreeMap;
use std::collections::BTreeSet;

pub use transition::*;

use crate::fabric::valid_blake3_ref;
use crate::fabric::valid_fabric_token;

pub const MEMBERSHIP_SOURCE_PROFILE_SCHEMA: &str = "molten.fabric.membership.source-profile.v1";
pub const NODE_DESCRIPTOR_SCHEMA: &str = "molten.fabric.membership.node-descriptor.v1";
pub const MEMBERSHIP_VIEW_SCHEMA: &str = "molten.fabric.membership.view.v1";
pub const FAILURE_OBSERVATION_SCHEMA: &str = "molten.fabric.membership.failure-observation.v1";
pub const PLACEMENT_PLAN_SCHEMA: &str = "molten.fabric.placement.plan.v1";
pub const ROLE_ASSIGNMENT_SCHEMA: &str = "molten.fabric.membership.role-assignment.v1";
pub const FENCING_PROFILE_SCHEMA: &str = "molten.fabric.membership.fencing-profile.v1";
pub const MEMBERSHIP_EVIDENCE_SCHEMA: &str = "molten.fabric.membership.evidence.v1";

pub const MAX_MEMBERSHIP_ITEMS: usize = 4_096;
pub const MAX_MEMBERSHIP_TEXT_BYTES: usize = 256;
pub const MAX_PLACEMENT_REPLICAS: u32 = 64;
pub const MAX_PLACEMENT_SEARCH_STEPS: u64 = 100_000;
pub const MAX_CONFIDENCE_BASIS_POINTS: u16 = 10_000;
const REQUIRED_MEMBERSHIP_NON_CLAIM_COUNT: usize = 7;
const REQUIRED_FAILURE_NON_CLAIM_COUNT: usize = 4;
const ADJACENT_PAIR_WIDTH: usize = 2;

pub const REQUIRED_MEMBERSHIP_NON_CLAIMS: [MembershipNonClaim; REQUIRED_MEMBERSHIP_NON_CLAIM_COUNT] = [
    MembershipNonClaim::NoConsensus,
    MembershipNonClaim::NoGlobalTruth,
    MembershipNonClaim::NoFailureProof,
    MembershipNonClaim::NoCapabilityAuthority,
    MembershipNonClaim::NoPlacementCommit,
    MembershipNonClaim::NoConnectivityEligibility,
    MembershipNonClaim::NoServiceCorrectness,
];

pub const REQUIRED_FAILURE_NON_CLAIMS: [FailureNonClaim; REQUIRED_FAILURE_NON_CLAIM_COUNT] = [
    FailureNonClaim::NoProcessDeathProof,
    FailureNonClaim::NoMembershipMutation,
    FailureNonClaim::NoAuthorityRevocation,
    FailureNonClaim::NoOwnershipTransfer,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MembershipNonClaim {
    NoConsensus,
    NoGlobalTruth,
    NoFailureProof,
    NoCapabilityAuthority,
    NoPlacementCommit,
    NoConnectivityEligibility,
    NoServiceCorrectness,
}

impl MembershipNonClaim {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoConsensus => "does-not-prove-consensus",
            Self::NoGlobalTruth => "does-not-provide-global-membership-truth",
            Self::NoFailureProof => "does-not-prove-process-failure",
            Self::NoCapabilityAuthority => "does-not-grant-capability-authority",
            Self::NoPlacementCommit => "does-not-commit-placement",
            Self::NoConnectivityEligibility => "connectivity-does-not-imply-eligibility",
            Self::NoServiceCorrectness => "does-not-prove-service-correctness",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FailureNonClaim {
    NoProcessDeathProof,
    NoMembershipMutation,
    NoAuthorityRevocation,
    NoOwnershipTransfer,
}

impl FailureNonClaim {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoProcessDeathProof => "does-not-prove-process-death",
            Self::NoMembershipMutation => "does-not-mutate-membership",
            Self::NoAuthorityRevocation => "does-not-revoke-authority",
            Self::NoOwnershipTransfer => "does-not-transfer-ownership",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MembershipProviderKind {
    Static,
    PolicyManaged,
    ConsistencyBacked,
    DeterministicSimulation,
}

impl MembershipProviderKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Static => "static",
            Self::PolicyManaged => "policy-managed",
            Self::ConsistencyBacked => "consistency-backed",
            Self::DeterministicSimulation => "deterministic-simulation",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MembershipAuthorityStrength {
    ObservationOnly,
    OperatorDeclared,
    ConsistencyOrdered,
    ExternallyEnforced,
}

impl MembershipAuthorityStrength {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ObservationOnly => "observation-only",
            Self::OperatorDeclared => "operator-declared",
            Self::ConsistencyOrdered => "consistency-ordered",
            Self::ExternallyEnforced => "externally-enforced",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipSourceProfile {
    pub schema: String,
    pub profile_id: String,
    pub profile_ref: String,
    pub provider_kind: MembershipProviderKind,
    pub authority_strength: MembershipAuthorityStrength,
    pub authority_scope: String,
    pub max_view_age_ticks: u64,
    pub non_claims: Vec<MembershipNonClaim>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LabelAuthority {
    Advisory,
    Measured,
    OperatorDeclared,
    Authoritative,
}

impl LabelAuthority {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Advisory => "advisory",
            Self::Measured => "measured",
            Self::OperatorDeclared => "operator-declared",
            Self::Authoritative => "authoritative",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeLabel {
    pub key: String,
    pub value: String,
    pub authority: LabelAuthority,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResourceAmount {
    pub cpu_millis: u64,
    pub memory_bytes: u64,
    pub storage_bytes: u64,
}

impl ResourceAmount {
    pub const fn fits(self, requested: Self) -> bool {
        self.cpu_millis >= requested.cpu_millis
            && self.memory_bytes >= requested.memory_bytes
            && self.storage_bytes >= requested.storage_bytes
    }

    pub fn checked_sub(self, requested: Self) -> Option<Self> {
        Some(Self {
            cpu_millis: self.cpu_millis.checked_sub(requested.cpu_millis)?,
            memory_bytes: self.memory_bytes.checked_sub(requested.memory_bytes)?,
            storage_bytes: self.storage_bytes.checked_sub(requested.storage_bytes)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeDescriptor {
    pub schema: String,
    pub node_id: String,
    pub descriptor_ref: String,
    pub compatibility_ref: String,
    pub labels: Vec<NodeLabel>,
    pub runtime_features: Vec<String>,
    pub capacity: ResourceAmount,
}

impl NodeDescriptor {
    pub fn label(&self, key: &str) -> Option<&NodeLabel> {
        self.labels.iter().find(|label| label.key == key)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipMember {
    pub node_id: String,
    pub descriptor_ref: String,
    pub eligibility_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipView {
    pub schema: String,
    pub view_id: String,
    pub epoch: u64,
    pub source_profile_ref: String,
    pub source_evidence_ref: String,
    pub authority_ref: String,
    pub eligibility_policy_ref: String,
    pub observed_at_ticks: u64,
    pub valid_until_ticks: u64,
    pub members: Vec<MembershipMember>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedMembershipView {
    pub profile: MembershipSourceProfile,
    pub view: MembershipView,
    pub descriptors: BTreeMap<String, NodeDescriptor>,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FailureObservationClass {
    Unknown,
    Reachable,
    Recovered,
    Suspected,
    Unavailable,
}

impl FailureObservationClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Reachable => "reachable",
            Self::Suspected => "suspected",
            Self::Unavailable => "unavailable",
            Self::Recovered => "recovered",
            Self::Unknown => "unknown",
        }
    }

    const fn precedence(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailureDetectorProfile {
    pub profile_id: String,
    pub profile_ref: String,
    pub time_basis_ref: String,
    pub max_observation_age_ticks: u64,
    pub non_claims: Vec<FailureNonClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailureObservation {
    pub schema: String,
    pub subject_node_id: String,
    pub detector_profile_ref: String,
    pub class: FailureObservationClass,
    pub observed_at_ticks: u64,
    pub valid_until_ticks: u64,
    pub confidence_basis_points: u16,
    pub supporting_event_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReducedFailureObservation {
    pub subject_node_id: String,
    pub class: FailureObservationClass,
    pub observed_at_ticks: u64,
    pub detector_profile_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardLabelConstraint {
    pub key: String,
    pub value: Option<String>,
    pub minimum_authority: LabelAuthority,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreferredLabel {
    pub key: String,
    pub value: String,
    pub minimum_authority: LabelAuthority,
    pub weight: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleRequirements {
    pub extension_id: String,
    pub service_id: String,
    pub role_kind: String,
    pub replica_count: u32,
    pub per_replica: ResourceAmount,
    pub required_features: Vec<String>,
    pub required_labels: Vec<HardLabelConstraint>,
    pub preferred_labels: Vec<PreferredLabel>,
    pub anti_affinity_label_keys: Vec<String>,
    pub distinct_nodes: bool,
    pub avoid_suspected: bool,
    pub allow_degraded: bool,
    pub policy_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapacityReservation {
    pub reservation_ref: String,
    pub node_id: String,
    pub resources: ResourceAmount,
    pub assignment_epoch: u64,
    pub released: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentRoleAssignment {
    pub assignment_ref: String,
    pub node_id: String,
    pub service_id: String,
    pub role_kind: String,
    pub assignment_epoch: u64,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementRequest {
    pub requirements: RoleRequirements,
    pub current_assignments: Vec<CurrentRoleAssignment>,
    pub current_reservations: Vec<CapacityReservation>,
    pub failure_observations: Vec<FailureObservation>,
    pub detector_profiles: Vec<FailureDetectorProfile>,
    pub tie_break_order: Vec<String>,
    pub conflicting_view_refs: Vec<String>,
    pub now_ticks: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedRole {
    pub role_ordinal: u32,
    pub node_id: String,
    pub descriptor_ref: String,
    pub resources: ResourceAmount,
    pub preference_score: u64,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementPlan {
    pub schema: String,
    pub view_id: String,
    pub view_epoch: u64,
    pub source_profile_ref: String,
    pub policy_ref: String,
    pub roles: Vec<PlannedRole>,
    pub residual_capacity: BTreeMap<String, ResourceAmount>,
    pub degraded: bool,
    pub advisory_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnsatisfiedConstraintKind {
    ConflictingViews,
    InsufficientEligibleNodes,
    InsufficientCapacity,
    RequiredLabel,
    RequiredFeature,
    AntiAffinity,
    FailurePolicy,
    SearchLimit,
}

impl UnsatisfiedConstraintKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::ConflictingViews => "conflicting-views",
            Self::InsufficientEligibleNodes => "insufficient-eligible-nodes",
            Self::InsufficientCapacity => "insufficient-capacity",
            Self::RequiredLabel => "required-label",
            Self::RequiredFeature => "required-feature",
            Self::AntiAffinity => "anti-affinity",
            Self::FailurePolicy => "failure-policy",
            Self::SearchLimit => "search-limit",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsatisfiedConstraint {
    pub kind: UnsatisfiedConstraintKind,
    pub subject: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsatisfiedPlacement {
    pub view_id: String,
    pub policy_ref: String,
    pub constraints: Vec<UnsatisfiedConstraint>,
    pub partial_selection: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlacementOutcome {
    Planned(PlacementPlan),
    Unsatisfied(UnsatisfiedPlacement),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MembershipIssue {
    SchemaMismatch(&'static str),
    EmptyField(&'static str),
    MalformedToken(&'static str),
    MalformedRef(&'static str),
    CollectionLimitExceeded(&'static str),
    MissingNonClaim(&'static str),
    ZeroEpoch,
    ZeroFreshnessWindow,
    ObservationFromFuture,
    FreshnessWindowInvalid,
    StaleView,
    SourceProfileMismatch,
    SourceAuthorityOverclaim,
    MembersNotStrictlyOrdered,
    LabelsNotStrictlyOrdered(String),
    FeaturesNotStrictlyOrdered(String),
    DuplicateValue(&'static str),
    DescriptorMissing(String),
    DescriptorIdentityMismatch(String),
    DescriptorRefMismatch(String),
    CompatibilityMismatch(String),
    UnknownObservationSubject(String),
    DetectorProfileMissing(String),
    DetectorProfileMismatch(String),
    ObservationStale(String),
    ConfidenceOutOfRange,
    ReservationUnknownNode(String),
    ReservationExceedsCapacity(String),
    CurrentAssignmentUnknownNode(String),
    ReplicaLimitExceeded,
    SearchLimitExceeded,
    ArithmeticOverflow(&'static str),
    ProviderParityMismatch(&'static str),
}

// r[impl molten.fabric_membership.membership_views]
// r[impl molten.fabric_membership.locality]
pub fn validate_membership_view(
    profile: &MembershipSourceProfile,
    view: &MembershipView,
    descriptors: &[NodeDescriptor],
    now_ticks: u64,
    required_compatibility_ref: &str,
) -> Result<AdmittedMembershipView, Vec<MembershipIssue>> {
    let mut issues = validate_source_profile(profile);
    validate_view_shape(profile, view, now_ticks, &mut issues);
    if !valid_blake3_ref(required_compatibility_ref) {
        issues.push(MembershipIssue::MalformedRef("required-compatibility-ref"));
    }
    if descriptors.len() > MAX_MEMBERSHIP_ITEMS {
        issues.push(MembershipIssue::CollectionLimitExceeded("node-descriptors"));
    }

    let mut descriptor_map = BTreeMap::new();
    for descriptor in descriptors {
        validate_descriptor(descriptor, required_compatibility_ref, &mut issues);
        if descriptor_map.insert(descriptor.node_id.clone(), descriptor.clone()).is_some() {
            issues.push(MembershipIssue::DuplicateValue("descriptor-node-id"));
        }
    }
    for member in &view.members {
        validate_member(member, &mut issues);
        match descriptor_map.get(&member.node_id) {
            None => issues.push(MembershipIssue::DescriptorMissing(member.node_id.clone())),
            Some(descriptor) => {
                if descriptor.node_id != member.node_id {
                    issues.push(MembershipIssue::DescriptorIdentityMismatch(member.node_id.clone()));
                }
                if descriptor.descriptor_ref != member.descriptor_ref {
                    issues.push(MembershipIssue::DescriptorRefMismatch(member.node_id.clone()));
                }
            }
        }
    }
    if descriptor_map.len() != view.members.len() {
        issues.push(MembershipIssue::DescriptorIdentityMismatch("view-descriptor-set".to_string()));
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    Ok(AdmittedMembershipView {
        profile: profile.clone(),
        view: view.clone(),
        descriptors: descriptor_map,
    })
}

pub fn validate_source_profile(profile: &MembershipSourceProfile) -> Vec<MembershipIssue> {
    let mut issues = Vec::new();
    if profile.schema != MEMBERSHIP_SOURCE_PROFILE_SCHEMA {
        issues.push(MembershipIssue::SchemaMismatch("membership-source-profile"));
    }
    validate_token("profile-id", &profile.profile_id, &mut issues);
    validate_ref("profile-ref", &profile.profile_ref, &mut issues);
    validate_token("authority-scope", &profile.authority_scope, &mut issues);
    if profile.max_view_age_ticks == 0 {
        issues.push(MembershipIssue::ZeroFreshnessWindow);
    }
    validate_membership_non_claims(&profile.non_claims, &mut issues);
    if matches!(profile.provider_kind, MembershipProviderKind::DeterministicSimulation)
        && profile.authority_strength > MembershipAuthorityStrength::OperatorDeclared
    {
        issues.push(MembershipIssue::SourceAuthorityOverclaim);
    }
    issues
}

fn validate_view_shape(
    profile: &MembershipSourceProfile,
    view: &MembershipView,
    now_ticks: u64,
    issues: &mut Vec<MembershipIssue>,
) {
    if view.schema != MEMBERSHIP_VIEW_SCHEMA {
        issues.push(MembershipIssue::SchemaMismatch("membership-view"));
    }
    validate_token("view-id", &view.view_id, issues);
    if view.epoch == 0 {
        issues.push(MembershipIssue::ZeroEpoch);
    }
    if view.source_profile_ref != profile.profile_ref {
        issues.push(MembershipIssue::SourceProfileMismatch);
    }
    for (field, value) in [
        ("source-evidence-ref", view.source_evidence_ref.as_str()),
        ("authority-ref", view.authority_ref.as_str()),
        ("eligibility-policy-ref", view.eligibility_policy_ref.as_str()),
    ] {
        validate_ref(field, value, issues);
    }
    if view.observed_at_ticks > now_ticks {
        issues.push(MembershipIssue::ObservationFromFuture);
    }
    if view.valid_until_ticks < view.observed_at_ticks {
        issues.push(MembershipIssue::FreshnessWindowInvalid);
    }
    let age = now_ticks.saturating_sub(view.observed_at_ticks);
    if now_ticks > view.valid_until_ticks || age > profile.max_view_age_ticks {
        issues.push(MembershipIssue::StaleView);
    }
    if view.members.is_empty() || view.members.len() > MAX_MEMBERSHIP_ITEMS {
        issues.push(MembershipIssue::CollectionLimitExceeded("membership-members"));
    }
    if !strictly_ordered_by(&view.members, |member| member.node_id.as_str()) {
        issues.push(MembershipIssue::MembersNotStrictlyOrdered);
    }
}

fn validate_descriptor(
    descriptor: &NodeDescriptor,
    required_compatibility_ref: &str,
    issues: &mut Vec<MembershipIssue>,
) {
    if descriptor.schema != NODE_DESCRIPTOR_SCHEMA {
        issues.push(MembershipIssue::SchemaMismatch("node-descriptor"));
    }
    validate_token("node-id", &descriptor.node_id, issues);
    validate_ref("descriptor-ref", &descriptor.descriptor_ref, issues);
    validate_ref("compatibility-ref", &descriptor.compatibility_ref, issues);
    if descriptor.compatibility_ref != required_compatibility_ref {
        issues.push(MembershipIssue::CompatibilityMismatch(descriptor.node_id.clone()));
    }
    if descriptor.labels.len() > MAX_MEMBERSHIP_ITEMS {
        issues.push(MembershipIssue::CollectionLimitExceeded("node-labels"));
    }
    if !strictly_ordered_by(&descriptor.labels, |label| label.key.as_str()) {
        issues.push(MembershipIssue::LabelsNotStrictlyOrdered(descriptor.node_id.clone()));
    }
    for label in &descriptor.labels {
        validate_token("label-key", &label.key, issues);
        validate_token("label-value", &label.value, issues);
        validate_ref("label-evidence-ref", &label.evidence_ref, issues);
    }
    if descriptor.runtime_features.len() > MAX_MEMBERSHIP_ITEMS {
        issues.push(MembershipIssue::CollectionLimitExceeded("runtime-features"));
    }
    if !strictly_ordered_by(&descriptor.runtime_features, String::as_str) {
        issues.push(MembershipIssue::FeaturesNotStrictlyOrdered(descriptor.node_id.clone()));
    }
    for feature in &descriptor.runtime_features {
        validate_token("runtime-feature", feature, issues);
    }
}

fn validate_member(member: &MembershipMember, issues: &mut Vec<MembershipIssue>) {
    validate_token("member-node-id", &member.node_id, issues);
    validate_ref("member-descriptor-ref", &member.descriptor_ref, issues);
    validate_ref("member-eligibility-ref", &member.eligibility_ref, issues);
}

// r[impl molten.fabric_membership.failure_detector]
pub fn reduce_failure_observations(
    view: &AdmittedMembershipView,
    profiles: &[FailureDetectorProfile],
    observations: &[FailureObservation],
    now_ticks: u64,
) -> Result<BTreeMap<String, ReducedFailureObservation>, Vec<MembershipIssue>> {
    let mut issues = Vec::new();
    let profile_map = validate_detector_profiles(profiles, &mut issues);
    let member_ids = view.view.members.iter().map(|member| member.node_id.as_str()).collect::<BTreeSet<_>>();
    let mut reduced: BTreeMap<String, ReducedFailureObservation> = BTreeMap::new();
    for observation in observations {
        validate_observation(observation, &profile_map, &member_ids, now_ticks, &mut issues);
        let candidate = ReducedFailureObservation {
            subject_node_id: observation.subject_node_id.clone(),
            class: observation.class,
            observed_at_ticks: observation.observed_at_ticks,
            detector_profile_ref: observation.detector_profile_ref.clone(),
        };
        match reduced.get(&observation.subject_node_id) {
            Some(current)
                if current.observed_at_ticks > candidate.observed_at_ticks
                    || (current.observed_at_ticks == candidate.observed_at_ticks
                        && current.class.precedence() >= candidate.class.precedence()) => {}
            _ => {
                reduced.insert(observation.subject_node_id.clone(), candidate);
            }
        }
    }
    if issues.is_empty() { Ok(reduced) } else { Err(issues) }
}

fn validate_detector_profiles<'a>(
    profiles: &'a [FailureDetectorProfile],
    issues: &mut Vec<MembershipIssue>,
) -> BTreeMap<&'a str, &'a FailureDetectorProfile> {
    let mut profile_map = BTreeMap::new();
    if profiles.len() > MAX_MEMBERSHIP_ITEMS {
        issues.push(MembershipIssue::CollectionLimitExceeded("detector-profiles"));
    }
    for profile in profiles {
        validate_token("detector-profile-id", &profile.profile_id, issues);
        validate_ref("detector-profile-ref", &profile.profile_ref, issues);
        validate_ref("detector-time-basis-ref", &profile.time_basis_ref, issues);
        if profile.max_observation_age_ticks == 0 {
            issues.push(MembershipIssue::ZeroFreshnessWindow);
        }
        validate_failure_non_claims(&profile.non_claims, issues);
        if profile_map.insert(profile.profile_ref.as_str(), profile).is_some() {
            issues.push(MembershipIssue::DuplicateValue("detector-profile-ref"));
        }
    }
    profile_map
}

fn validate_observation(
    observation: &FailureObservation,
    profiles: &BTreeMap<&str, &FailureDetectorProfile>,
    member_ids: &BTreeSet<&str>,
    now_ticks: u64,
    issues: &mut Vec<MembershipIssue>,
) {
    if observation.schema != FAILURE_OBSERVATION_SCHEMA {
        issues.push(MembershipIssue::SchemaMismatch("failure-observation"));
    }
    validate_token("observation-subject", &observation.subject_node_id, issues);
    if !member_ids.contains(observation.subject_node_id.as_str()) {
        issues.push(MembershipIssue::UnknownObservationSubject(observation.subject_node_id.clone()));
    }
    let Some(profile) = profiles.get(observation.detector_profile_ref.as_str()) else {
        issues.push(MembershipIssue::DetectorProfileMissing(observation.detector_profile_ref.clone()));
        return;
    };
    if profile.profile_ref != observation.detector_profile_ref {
        issues.push(MembershipIssue::DetectorProfileMismatch(observation.detector_profile_ref.clone()));
    }
    if observation.observed_at_ticks > now_ticks {
        issues.push(MembershipIssue::ObservationFromFuture);
    }
    let age = now_ticks.saturating_sub(observation.observed_at_ticks);
    if observation.valid_until_ticks < observation.observed_at_ticks
        || now_ticks > observation.valid_until_ticks
        || age > profile.max_observation_age_ticks
    {
        issues.push(MembershipIssue::ObservationStale(observation.subject_node_id.clone()));
    }
    if observation.confidence_basis_points > MAX_CONFIDENCE_BASIS_POINTS {
        issues.push(MembershipIssue::ConfidenceOutOfRange);
    }
    if observation.supporting_event_refs.is_empty() || observation.supporting_event_refs.len() > MAX_MEMBERSHIP_ITEMS {
        issues.push(MembershipIssue::CollectionLimitExceeded("failure-supporting-events"));
    }
    for event_ref in &observation.supporting_event_refs {
        validate_ref("failure-supporting-event-ref", event_ref, issues);
    }
}

// r[impl molten.fabric_membership.placement]
pub fn plan_placement(
    admitted: &AdmittedMembershipView,
    request: &PlacementRequest,
) -> Result<PlacementOutcome, Vec<MembershipIssue>> {
    let mut issues = validate_placement_request(admitted, request);
    let observations = match reduce_failure_observations(
        admitted,
        &request.detector_profiles,
        &request.failure_observations,
        request.now_ticks,
    ) {
        Ok(observations) => observations,
        Err(mut observation_issues) => {
            issues.append(&mut observation_issues);
            BTreeMap::new()
        }
    };
    let mut residual = initial_residual_capacity(admitted, &request.current_reservations, &mut issues);
    if !issues.is_empty() {
        return Err(issues);
    }
    if !request.conflicting_view_refs.is_empty() {
        return Ok(PlacementOutcome::Unsatisfied(UnsatisfiedPlacement {
            view_id: admitted.view.view_id.clone(),
            policy_ref: request.requirements.policy_ref.clone(),
            constraints: vec![UnsatisfiedConstraint {
                kind: UnsatisfiedConstraintKind::ConflictingViews,
                subject: admitted.profile.authority_scope.clone(),
                detail: "unreconciled membership views cannot be promoted to one authoritative view".to_string(),
            }],
            partial_selection: Vec::new(),
        }));
    }

    let candidates = eligible_candidates(admitted, request, &observations, &residual);
    let mut search = PlacementSearch::new(admitted, request, &candidates, &mut residual);
    let found = search.select(0)?;
    if !found {
        let partial_selection = search.selected.iter().map(|candidate| candidate.node_id.clone()).collect();
        drop(search);
        let constraints = explain_unsatisfied(admitted, request, &observations, &residual, &candidates);
        return Ok(PlacementOutcome::Unsatisfied(UnsatisfiedPlacement {
            view_id: admitted.view.view_id.clone(),
            policy_ref: request.requirements.policy_ref.clone(),
            constraints,
            partial_selection,
        }));
    }

    let roles = search
        .selected
        .iter()
        .enumerate()
        .map(|(ordinal, candidate)| PlannedRole {
            role_ordinal: u32::try_from(ordinal).unwrap_or(u32::MAX),
            node_id: candidate.node_id.clone(),
            descriptor_ref: candidate.descriptor_ref.clone(),
            resources: request.requirements.per_replica,
            preference_score: candidate.preference_score,
            reasons: candidate.reasons.clone(),
        })
        .collect();
    let degraded = search.degraded;
    drop(search);
    Ok(PlacementOutcome::Planned(PlacementPlan {
        schema: PLACEMENT_PLAN_SCHEMA.to_string(),
        view_id: admitted.view.view_id.clone(),
        view_epoch: admitted.view.epoch,
        source_profile_ref: admitted.profile.profile_ref.clone(),
        policy_ref: request.requirements.policy_ref.clone(),
        roles,
        residual_capacity: residual,
        degraded,
        advisory_only: true,
    }))
}

#[derive(Debug, Clone)]
struct PlacementCandidate {
    node_id: String,
    descriptor_ref: String,
    preference_score: u64,
    reasons: Vec<String>,
}

fn validate_placement_request(admitted: &AdmittedMembershipView, request: &PlacementRequest) -> Vec<MembershipIssue> {
    let mut issues = Vec::new();
    let requirements = &request.requirements;
    for (field, value) in [
        ("extension-id", requirements.extension_id.as_str()),
        ("service-id", requirements.service_id.as_str()),
        ("role-kind", requirements.role_kind.as_str()),
    ] {
        validate_token(field, value, &mut issues);
    }
    validate_ref("placement-policy-ref", &requirements.policy_ref, &mut issues);
    if requirements.replica_count == 0 || requirements.replica_count > MAX_PLACEMENT_REPLICAS {
        issues.push(MembershipIssue::ReplicaLimitExceeded);
    }
    validate_sorted_tokens("required-features", &requirements.required_features, &mut issues);
    validate_sorted_tokens("anti-affinity-label-keys", &requirements.anti_affinity_label_keys, &mut issues);
    if !strictly_ordered_by(&requirements.required_labels, |constraint| constraint.key.as_str()) {
        issues.push(MembershipIssue::DuplicateValue("required-label-key"));
    }
    for constraint in &requirements.required_labels {
        validate_token("required-label-key", &constraint.key, &mut issues);
        if let Some(value) = &constraint.value {
            validate_token("required-label-value", value, &mut issues);
        }
    }
    if !requirements
        .preferred_labels
        .windows(ADJACENT_PAIR_WIDTH)
        .all(|pair| (&pair[0].key, &pair[0].value) < (&pair[1].key, &pair[1].value))
    {
        issues.push(MembershipIssue::DuplicateValue("preferred-label-key-value"));
    }
    for preference in &requirements.preferred_labels {
        validate_token("preferred-label-key", &preference.key, &mut issues);
        validate_token("preferred-label-value", &preference.value, &mut issues);
    }
    if request.current_assignments.len() > MAX_MEMBERSHIP_ITEMS {
        issues.push(MembershipIssue::CollectionLimitExceeded("current-assignments"));
    }
    let mut assignment_refs = BTreeSet::new();
    for assignment in &request.current_assignments {
        validate_ref("current-assignment-ref", &assignment.assignment_ref, &mut issues);
        validate_token("current-assignment-service-id", &assignment.service_id, &mut issues);
        validate_token("current-assignment-role-kind", &assignment.role_kind, &mut issues);
        if !assignment_refs.insert(assignment.assignment_ref.as_str()) {
            issues.push(MembershipIssue::DuplicateValue("current-assignment-ref"));
        }
        if assignment.assignment_epoch == 0 {
            issues.push(MembershipIssue::ZeroEpoch);
        }
        if !admitted.descriptors.contains_key(&assignment.node_id) {
            issues.push(MembershipIssue::CurrentAssignmentUnknownNode(assignment.node_id.clone()));
        }
    }
    if request.tie_break_order.len() != admitted.view.members.len()
        || !same_unique_values(
            request.tie_break_order.iter().map(String::as_str),
            admitted.view.members.iter().map(|member| member.node_id.as_str()),
        )
    {
        issues.push(MembershipIssue::DuplicateValue("tie-break-order-or-membership-mismatch"));
    }
    if request.conflicting_view_refs.len() > MAX_MEMBERSHIP_ITEMS {
        issues.push(MembershipIssue::CollectionLimitExceeded("conflicting-view-refs"));
    }
    for view_ref in &request.conflicting_view_refs {
        validate_ref("conflicting-view-ref", view_ref, &mut issues);
    }
    issues
}

fn initial_residual_capacity(
    admitted: &AdmittedMembershipView,
    reservations: &[CapacityReservation],
    issues: &mut Vec<MembershipIssue>,
) -> BTreeMap<String, ResourceAmount> {
    let mut residual = admitted
        .descriptors
        .iter()
        .map(|(node_id, descriptor)| (node_id.clone(), descriptor.capacity))
        .collect::<BTreeMap<_, _>>();
    if reservations.len() > MAX_MEMBERSHIP_ITEMS {
        issues.push(MembershipIssue::CollectionLimitExceeded("capacity-reservations"));
    }
    let mut refs = BTreeSet::new();
    for reservation in reservations.iter().filter(|reservation| !reservation.released) {
        validate_ref("reservation-ref", &reservation.reservation_ref, issues);
        if !refs.insert(reservation.reservation_ref.as_str()) {
            issues.push(MembershipIssue::DuplicateValue("reservation-ref"));
        }
        let Some(available) = residual.get_mut(&reservation.node_id) else {
            issues.push(MembershipIssue::ReservationUnknownNode(reservation.node_id.clone()));
            continue;
        };
        match available.checked_sub(reservation.resources) {
            Some(next) => *available = next,
            None => issues.push(MembershipIssue::ReservationExceedsCapacity(reservation.node_id.clone())),
        }
    }
    residual
}

fn eligible_candidates(
    admitted: &AdmittedMembershipView,
    request: &PlacementRequest,
    observations: &BTreeMap<String, ReducedFailureObservation>,
    residual: &BTreeMap<String, ResourceAmount>,
) -> Vec<PlacementCandidate> {
    let tie_rank = request
        .tie_break_order
        .iter()
        .enumerate()
        .map(|(rank, node_id)| (node_id.as_str(), rank))
        .collect::<BTreeMap<_, _>>();
    let mut candidates = Vec::new();
    for member in &admitted.view.members {
        let Some(descriptor) = admitted.descriptors.get(&member.node_id) else {
            continue;
        };
        let Some(available) = residual.get(&member.node_id) else {
            continue;
        };
        if !available.fits(request.requirements.per_replica)
            || !required_features_match(descriptor, &request.requirements.required_features)
            || !required_labels_match(descriptor, &request.requirements.required_labels)
        {
            continue;
        }
        let failure_class = observations.get(&member.node_id).map(|observation| observation.class);
        let failure_avoided =
            matches!(failure_class, Some(FailureObservationClass::Suspected | FailureObservationClass::Unavailable));
        if request.requirements.avoid_suspected && failure_avoided && !request.requirements.allow_degraded {
            continue;
        }
        let preference_score = request
            .requirements
            .preferred_labels
            .iter()
            .filter(|preference| {
                descriptor.label(&preference.key).is_some_and(|label| {
                    label.value == preference.value && label.authority >= preference.minimum_authority
                })
            })
            .map(|preference| u64::from(preference.weight))
            .sum();
        let mut reasons = vec![
            "membership-and-descriptor-admitted".to_string(),
            "capacity-available".to_string(),
        ];
        if preference_score > 0 {
            reasons.push("preferred-locality-matched".to_string());
        }
        if failure_avoided {
            reasons.push("degraded-failure-observation-accepted".to_string());
        }
        candidates.push(PlacementCandidate {
            node_id: member.node_id.clone(),
            descriptor_ref: member.descriptor_ref.clone(),
            preference_score,
            reasons,
        });
    }
    candidates.sort_by(|left, right| {
        right
            .preference_score
            .cmp(&left.preference_score)
            .then_with(|| tie_rank.get(left.node_id.as_str()).cmp(&tie_rank.get(right.node_id.as_str())))
            .then_with(|| left.node_id.cmp(&right.node_id))
    });
    candidates
}

struct PlacementSearch<'a> {
    admitted: &'a AdmittedMembershipView,
    request: &'a PlacementRequest,
    candidates: &'a [PlacementCandidate],
    residual: &'a mut BTreeMap<String, ResourceAmount>,
    selected: Vec<PlacementCandidate>,
    occupied_nodes: BTreeSet<String>,
    anti_values: BTreeMap<String, BTreeSet<String>>,
    search_steps: u64,
    degraded: bool,
}

impl<'a> PlacementSearch<'a> {
    fn new(
        admitted: &'a AdmittedMembershipView,
        request: &'a PlacementRequest,
        candidates: &'a [PlacementCandidate],
        residual: &'a mut BTreeMap<String, ResourceAmount>,
    ) -> Self {
        let mut occupied_nodes = BTreeSet::new();
        let mut anti_values: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for assignment in request.current_assignments.iter().filter(|assignment| {
            assignment.active
                && assignment.service_id == request.requirements.service_id
                && assignment.role_kind == request.requirements.role_kind
        }) {
            occupied_nodes.insert(assignment.node_id.clone());
            if let Some(descriptor) = admitted.descriptors.get(&assignment.node_id) {
                for key in &request.requirements.anti_affinity_label_keys {
                    if let Some(label) = descriptor.label(key) {
                        anti_values.entry(key.clone()).or_default().insert(label.value.clone());
                    }
                }
            }
        }
        Self {
            admitted,
            request,
            candidates,
            residual,
            selected: Vec::new(),
            occupied_nodes,
            anti_values,
            search_steps: 0,
            degraded: false,
        }
    }

    fn select(&mut self, ordinal: u32) -> Result<bool, Vec<MembershipIssue>> {
        if ordinal == self.request.requirements.replica_count {
            return Ok(true);
        }
        for candidate in self.candidates {
            self.search_steps = self
                .search_steps
                .checked_add(1)
                .ok_or_else(|| vec![MembershipIssue::ArithmeticOverflow("placement-search-steps")])?;
            if self.search_steps > MAX_PLACEMENT_SEARCH_STEPS {
                return Err(vec![MembershipIssue::SearchLimitExceeded]);
            }
            if !self.can_select(candidate) {
                continue;
            }
            let prior_capacity = self.residual[&candidate.node_id];
            let next_capacity = prior_capacity
                .checked_sub(self.request.requirements.per_replica)
                .expect("candidate capacity checked before selection");
            let inserted_anti = self.insert_anti_values(candidate);
            *self.residual.get_mut(&candidate.node_id).expect("candidate node exists") = next_capacity;
            let candidate_degraded =
                candidate.reasons.iter().any(|reason| reason == "degraded-failure-observation-accepted");
            let prior_degraded = self.degraded;
            self.degraded |= candidate_degraded;
            self.selected.push(candidate.clone());
            if self.select(ordinal.saturating_add(1))? {
                return Ok(true);
            }
            self.selected.pop();
            self.degraded = prior_degraded;
            *self.residual.get_mut(&candidate.node_id).expect("candidate node exists") = prior_capacity;
            self.remove_anti_values(inserted_anti);
        }
        Ok(false)
    }

    fn can_select(&self, candidate: &PlacementCandidate) -> bool {
        if self.request.requirements.distinct_nodes
            && (self.occupied_nodes.contains(&candidate.node_id)
                || self.selected.iter().any(|selected| selected.node_id == candidate.node_id))
        {
            return false;
        }
        if !self.residual[&candidate.node_id].fits(self.request.requirements.per_replica) {
            return false;
        }
        let descriptor = &self.admitted.descriptors[&candidate.node_id];
        self.request.requirements.anti_affinity_label_keys.iter().all(|key| {
            descriptor
                .label(key)
                .is_some_and(|label| !self.anti_values.get(key).is_some_and(|values| values.contains(&label.value)))
        })
    }

    fn insert_anti_values(&mut self, candidate: &PlacementCandidate) -> Vec<(String, String)> {
        let descriptor = &self.admitted.descriptors[&candidate.node_id];
        let mut inserted = Vec::new();
        for key in &self.request.requirements.anti_affinity_label_keys {
            if let Some(label) = descriptor.label(key) {
                self.anti_values.entry(key.clone()).or_default().insert(label.value.clone());
                inserted.push((key.clone(), label.value.clone()));
            }
        }
        inserted
    }

    fn remove_anti_values(&mut self, inserted: Vec<(String, String)>) {
        for (key, value) in inserted {
            if let Some(values) = self.anti_values.get_mut(&key) {
                values.remove(&value);
            }
        }
    }
}

fn explain_unsatisfied(
    admitted: &AdmittedMembershipView,
    request: &PlacementRequest,
    observations: &BTreeMap<String, ReducedFailureObservation>,
    residual: &BTreeMap<String, ResourceAmount>,
    candidates: &[PlacementCandidate],
) -> Vec<UnsatisfiedConstraint> {
    let mut constraints = Vec::new();
    if candidates.len() < usize::try_from(request.requirements.replica_count).unwrap_or(usize::MAX) {
        constraints.push(UnsatisfiedConstraint {
            kind: UnsatisfiedConstraintKind::InsufficientEligibleNodes,
            subject: request.requirements.role_kind.clone(),
            detail: "eligible candidate count is below requested replicas".to_string(),
        });
    }
    if !admitted.descriptors.values().any(|descriptor| {
        residual
            .get(&descriptor.node_id)
            .is_some_and(|capacity| capacity.fits(request.requirements.per_replica))
    }) {
        constraints.push(UnsatisfiedConstraint {
            kind: UnsatisfiedConstraintKind::InsufficientCapacity,
            subject: request.requirements.role_kind.clone(),
            detail: "no member has residual per-replica capacity".to_string(),
        });
    }
    for constraint in &request.requirements.required_labels {
        if !admitted.descriptors.values().any(|descriptor| label_constraint_matches(descriptor, constraint)) {
            constraints.push(UnsatisfiedConstraint {
                kind: UnsatisfiedConstraintKind::RequiredLabel,
                subject: constraint.key.clone(),
                detail: "no admitted descriptor satisfies the required label authority and value".to_string(),
            });
        }
    }
    for feature in &request.requirements.required_features {
        if !admitted
            .descriptors
            .values()
            .any(|descriptor| descriptor.runtime_features.binary_search(feature).is_ok())
        {
            constraints.push(UnsatisfiedConstraint {
                kind: UnsatisfiedConstraintKind::RequiredFeature,
                subject: feature.clone(),
                detail: "no admitted descriptor declares the required runtime feature".to_string(),
            });
        }
    }
    if request.requirements.avoid_suspected
        && observations.values().any(|observation| {
            matches!(observation.class, FailureObservationClass::Suspected | FailureObservationClass::Unavailable)
        })
    {
        constraints.push(UnsatisfiedConstraint {
            kind: UnsatisfiedConstraintKind::FailurePolicy,
            subject: request.requirements.role_kind.clone(),
            detail: "failure policy excludes one or more otherwise eligible nodes".to_string(),
        });
    }
    if !request.requirements.anti_affinity_label_keys.is_empty() {
        constraints.push(UnsatisfiedConstraint {
            kind: UnsatisfiedConstraintKind::AntiAffinity,
            subject: request.requirements.anti_affinity_label_keys.join(","),
            detail: "no deterministic selection satisfies every anti-affinity key".to_string(),
        });
    }
    if constraints.is_empty() {
        constraints.push(UnsatisfiedConstraint {
            kind: UnsatisfiedConstraintKind::InsufficientEligibleNodes,
            subject: request.requirements.role_kind.clone(),
            detail: "combined hard constraints are unsatisfied".to_string(),
        });
    }
    constraints
}

fn required_features_match(descriptor: &NodeDescriptor, required: &[String]) -> bool {
    required.iter().all(|feature| descriptor.runtime_features.binary_search(feature).is_ok())
}

fn required_labels_match(descriptor: &NodeDescriptor, required: &[HardLabelConstraint]) -> bool {
    required.iter().all(|constraint| label_constraint_matches(descriptor, constraint))
}

fn label_constraint_matches(descriptor: &NodeDescriptor, constraint: &HardLabelConstraint) -> bool {
    descriptor.label(&constraint.key).is_some_and(|label| {
        label.authority >= constraint.minimum_authority
            && constraint.value.as_ref().is_none_or(|value| label.value == *value)
    })
}

// r[impl molten.fabric_membership.live_sim_parity]
pub fn validate_provider_parity(
    live: &AdmittedMembershipView,
    simulated: &AdmittedMembershipView,
    live_observations: &BTreeMap<String, ReducedFailureObservation>,
    simulated_observations: &BTreeMap<String, ReducedFailureObservation>,
    live_reservations: &[CapacityReservation],
    simulated_reservations: &[CapacityReservation],
) -> Result<(), Vec<MembershipIssue>> {
    let mut issues = Vec::new();
    if !provider_view_contract_equal(live, simulated) {
        issues.push(MembershipIssue::ProviderParityMismatch("membership-view"));
    }
    if live.descriptors != simulated.descriptors {
        issues.push(MembershipIssue::ProviderParityMismatch("node-descriptors"));
    }
    if !provider_observations_equal(live_observations, simulated_observations) {
        issues.push(MembershipIssue::ProviderParityMismatch("failure-observations"));
    }
    if !provider_reservations_equal(live_reservations, simulated_reservations) {
        issues.push(MembershipIssue::ProviderParityMismatch("resource-reservations"));
    }
    if issues.is_empty() { Ok(()) } else { Err(issues) }
}

fn provider_view_contract_equal(left: &AdmittedMembershipView, right: &AdmittedMembershipView) -> bool {
    left.view.schema == right.view.schema
        && left.view.view_id == right.view.view_id
        && left.view.epoch == right.view.epoch
        && left.view.eligibility_policy_ref == right.view.eligibility_policy_ref
        && left.view.observed_at_ticks == right.view.observed_at_ticks
        && left.view.valid_until_ticks == right.view.valid_until_ticks
        && left.view.members == right.view.members
        && left.profile.max_view_age_ticks == right.profile.max_view_age_ticks
        && left.profile.non_claims == right.profile.non_claims
}

fn provider_observations_equal(
    left: &BTreeMap<String, ReducedFailureObservation>,
    right: &BTreeMap<String, ReducedFailureObservation>,
) -> bool {
    left.len() == right.len()
        && left.iter().all(|(subject, observation)| {
            right.get(subject).is_some_and(|other| {
                observation.subject_node_id == other.subject_node_id
                    && observation.class == other.class
                    && observation.observed_at_ticks == other.observed_at_ticks
            })
        })
}

fn provider_reservations_equal(left: &[CapacityReservation], right: &[CapacityReservation]) -> bool {
    let mut left = left
        .iter()
        .map(|reservation| {
            (
                reservation.node_id.as_str(),
                reservation.assignment_epoch,
                reservation.released,
                reservation.resources,
            )
        })
        .collect::<Vec<_>>();
    let mut right = right
        .iter()
        .map(|reservation| {
            (
                reservation.node_id.as_str(),
                reservation.assignment_epoch,
                reservation.released,
                reservation.resources,
            )
        })
        .collect::<Vec<_>>();
    left.sort_unstable();
    right.sort_unstable();
    left == right
}

fn validate_membership_non_claims(claims: &[MembershipNonClaim], issues: &mut Vec<MembershipIssue>) {
    if claims.len() > MAX_MEMBERSHIP_ITEMS {
        issues.push(MembershipIssue::CollectionLimitExceeded("membership-non-claims"));
    }
    let supplied = claims.iter().copied().collect::<BTreeSet<_>>();
    if supplied.len() != claims.len() {
        issues.push(MembershipIssue::DuplicateValue("membership-non-claim"));
    }
    for required in REQUIRED_MEMBERSHIP_NON_CLAIMS {
        if !supplied.contains(&required) {
            issues.push(MembershipIssue::MissingNonClaim(required.as_str()));
        }
    }
}

fn validate_failure_non_claims(claims: &[FailureNonClaim], issues: &mut Vec<MembershipIssue>) {
    if claims.len() > MAX_MEMBERSHIP_ITEMS {
        issues.push(MembershipIssue::CollectionLimitExceeded("failure-non-claims"));
    }
    let supplied = claims.iter().copied().collect::<BTreeSet<_>>();
    if supplied.len() != claims.len() {
        issues.push(MembershipIssue::DuplicateValue("failure-non-claim"));
    }
    for required in REQUIRED_FAILURE_NON_CLAIMS {
        if !supplied.contains(&required) {
            issues.push(MembershipIssue::MissingNonClaim(required.as_str()));
        }
    }
}

fn validate_sorted_tokens(field: &'static str, values: &[String], issues: &mut Vec<MembershipIssue>) {
    if values.len() > MAX_MEMBERSHIP_ITEMS {
        issues.push(MembershipIssue::CollectionLimitExceeded(field));
    }
    if !strictly_ordered_by(values, String::as_str) {
        issues.push(MembershipIssue::DuplicateValue(field));
    }
    for value in values {
        validate_token(field, value, issues);
    }
}

fn validate_token(field: &'static str, value: &str, issues: &mut Vec<MembershipIssue>) {
    if value.is_empty() {
        issues.push(MembershipIssue::EmptyField(field));
    } else if value.len() > MAX_MEMBERSHIP_TEXT_BYTES || !valid_fabric_token(value) {
        issues.push(MembershipIssue::MalformedToken(field));
    }
}

fn validate_ref(field: &'static str, value: &str, issues: &mut Vec<MembershipIssue>) {
    if !valid_blake3_ref(value) {
        issues.push(MembershipIssue::MalformedRef(field));
    }
}

fn strictly_ordered_by<T, F>(values: &[T], key: F) -> bool
where F: Fn(&T) -> &str {
    values.windows(ADJACENT_PAIR_WIDTH).all(|pair| key(&pair[0]) < key(&pair[1]))
}

fn same_unique_values<'a>(left: impl Iterator<Item = &'a str>, right: impl Iterator<Item = &'a str>) -> bool {
    let left = left.collect::<BTreeSet<_>>();
    let right = right.collect::<BTreeSet<_>>();
    left == right
}

#[cfg(test)]
mod tests;
