use super::*;
use crate::system_extension::SystemExtensionExecutor;

pub const FABRIC_MEMBERSHIP_PORT_ID: &str = "molten.fabric.membership.views";
pub const FABRIC_FAILURE_OBSERVATION_PORT_ID: &str = "molten.fabric.membership.failure-observations";
pub const FABRIC_PLACEMENT_PORT_ID: &str = "molten.fabric.placement.plan";
pub const FABRIC_ASSIGNMENT_PORT_ID: &str = "molten.fabric.membership.assignments";
pub const FABRIC_MEMBERSHIP_PORT_VERSION: &str = "v1";

const MEMBERSHIP_PROFILE_RECORD: &str = "fabric-membership-source-profile-v1";
const MEMBERSHIP_VIEW_RECORD: &str = "fabric-membership-view-v1";
const FAILURE_OBSERVATION_RECORD: &str = "fabric-failure-observation-set-v1";
const PLACEMENT_OUTCOME_RECORD: &str = "fabric-placement-outcome-v1";
const ASSIGNMENT_TRANSITION_RECORD: &str = "fabric-role-assignment-transition-v1";
const MEMBERSHIP_STATUS_RECORD: &str = "fabric-membership-status-v1";
const MEMBERSHIP_PORT_COUNT: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalMembershipProfile {
    pub profile: MembershipSourceProfile,
    pub admission_ref: String,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalMembershipView {
    pub admitted: AdmittedMembershipView,
    pub view_ref: String,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalFailureObservationSet {
    pub observations_ref: String,
    pub observations: std::collections::BTreeMap<String, ReducedFailureObservation>,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalPlacementOutcome {
    pub outcome_ref: String,
    pub outcome: PlacementOutcome,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalAssignmentTransition {
    pub transition_ref: String,
    pub transition: AssignmentTransition,
    pub intent_ref: String,
    pub role_effect_ref: Option<String>,
    pub persistence_ref: String,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipStatusReadback {
    pub status_ref: String,
    pub view_ref: String,
    pub view_id: String,
    pub view_epoch: u64,
    pub provider_kind: MembershipProviderKind,
    pub member_ids: Vec<String>,
    pub active_assignments: u64,
    pub draining_assignments: u64,
    pub uncertain_assignments: u64,
    pub assignment_refs: Vec<String>,
    pub non_claims: Vec<MembershipNonClaim>,
    pub value: preserves::IOValue,
}

// r[impl molten.modularity.fabric_boundary.compatibility]
// r[impl molten.fabric_membership.membership_views]
// r[impl molten.fabric_membership.evidence]
pub fn canonical_membership_profile(
    profile: &MembershipSourceProfile,
) -> crate::error::Result<CanonicalMembershipProfile> {
    let issues = validate_source_profile(profile);
    if !issues.is_empty() {
        return Err(validation_error("membership source profile", &issues));
    }
    let value = crate::preserves_rail::record(MEMBERSHIP_PROFILE_RECORD, vec![
        crate::preserves_rail::string(MEMBERSHIP_SOURCE_PROFILE_SCHEMA),
        field("profile-id", crate::preserves_rail::string(&profile.profile_id)),
        field("declared-profile-ref", crate::preserves_rail::string(&profile.profile_ref)),
        field("provider-kind", crate::preserves_rail::string(profile.provider_kind.as_str())),
        field("authority-strength", crate::preserves_rail::string(profile.authority_strength.as_str())),
        field("authority-scope", crate::preserves_rail::string(&profile.authority_scope)),
        field("max-view-age-ticks", crate::preserves_rail::u64_value(profile.max_view_age_ticks)),
        field("non-claims", strings_value(profile.non_claims.iter().map(|claim| claim.as_str()))),
        checks(&[
            "source-scope-explicit",
            "freshness-bounded",
            "authority-strength-explicit",
            "connectivity-not-membership",
            "placement-not-assignment",
        ]),
    ]);
    let admission_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalMembershipProfile {
        profile: profile.clone(),
        admission_ref,
        value,
    })
}

// r[impl molten.modularity.fabric_boundary.compatibility]
// r[impl molten.fabric_membership.membership_views]
// r[impl molten.fabric_membership.locality]
// r[impl molten.fabric_membership.evidence]
pub fn canonical_membership_view(
    profile: &CanonicalMembershipProfile,
    view: &MembershipView,
    descriptors: &[NodeDescriptor],
    now_ticks: u64,
    required_compatibility_ref: &str,
) -> crate::error::Result<CanonicalMembershipView> {
    let admitted = validate_membership_view(&profile.profile, view, descriptors, now_ticks, required_compatibility_ref)
        .map_err(|issues| validation_error("membership view", &issues))?;
    let members = admitted
        .view
        .members
        .iter()
        .map(|member| {
            let descriptor = &admitted.descriptors[&member.node_id];
            crate::preserves_rail::record("fabric-membership-member-v1", vec![
                field("node-id", crate::preserves_rail::string(&member.node_id)),
                field("descriptor-ref", crate::preserves_rail::string(&member.descriptor_ref)),
                field("eligibility-ref", crate::preserves_rail::string(&member.eligibility_ref)),
                field("compatibility-ref", crate::preserves_rail::string(&descriptor.compatibility_ref)),
                field("capacity", resource_value(descriptor.capacity)),
                field(
                    "labels",
                    crate::preserves_rail::sequence(
                        descriptor
                            .labels
                            .iter()
                            .map(|label| {
                                crate::preserves_rail::record("fabric-membership-label-v1", vec![
                                    crate::preserves_rail::string(&label.key),
                                    crate::preserves_rail::string(&label.value),
                                    crate::preserves_rail::string(label.authority.as_str()),
                                    crate::preserves_rail::string(&label.evidence_ref),
                                ])
                            })
                            .collect(),
                    ),
                ),
                field("runtime-features", strings_value(descriptor.runtime_features.iter().map(String::as_str))),
            ])
        })
        .collect();
    let value = crate::preserves_rail::record(MEMBERSHIP_VIEW_RECORD, vec![
        crate::preserves_rail::string(MEMBERSHIP_VIEW_SCHEMA),
        field("profile-admission-ref", crate::preserves_rail::string(&profile.admission_ref)),
        field("source-profile-ref", crate::preserves_rail::string(&admitted.profile.profile_ref)),
        field("view-id", crate::preserves_rail::string(&admitted.view.view_id)),
        field("epoch", crate::preserves_rail::u64_value(admitted.view.epoch)),
        field("source-evidence-ref", crate::preserves_rail::string(&admitted.view.source_evidence_ref)),
        field("authority-ref", crate::preserves_rail::string(&admitted.view.authority_ref)),
        field("eligibility-policy-ref", crate::preserves_rail::string(&admitted.view.eligibility_policy_ref)),
        field("observed-at-ticks", crate::preserves_rail::u64_value(admitted.view.observed_at_ticks)),
        field("valid-until-ticks", crate::preserves_rail::u64_value(admitted.view.valid_until_ticks)),
        field("members", crate::preserves_rail::sequence(members)),
        field("non-claims", strings_value(admitted.profile.non_claims.iter().map(|claim| claim.as_str()))),
        checks(&[
            "source-scoped-snapshot",
            "members-strictly-ordered",
            "descriptor-identity-exact",
            "labels-authority-typed",
            "freshness-checked",
        ]),
    ]);
    let view_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalMembershipView {
        admitted,
        view_ref,
        value,
    })
}

// r[impl molten.fabric_membership.failure_detector]
// r[impl molten.fabric_membership.evidence]
pub fn canonical_failure_observations(
    view: &CanonicalMembershipView,
    profiles: &[FailureDetectorProfile],
    observations: &[FailureObservation],
    now_ticks: u64,
) -> crate::error::Result<CanonicalFailureObservationSet> {
    let reduced = reduce_failure_observations(&view.admitted, profiles, observations, now_ticks)
        .map_err(|issues| validation_error("failure observations", &issues))?;
    let values = reduced
        .values()
        .map(|observation| {
            crate::preserves_rail::record("fabric-reduced-failure-observation-v1", vec![
                field("subject-node-id", crate::preserves_rail::string(&observation.subject_node_id)),
                field("class", crate::preserves_rail::string(observation.class.as_str())),
                field("observed-at-ticks", crate::preserves_rail::u64_value(observation.observed_at_ticks)),
                field("detector-profile-ref", crate::preserves_rail::string(&observation.detector_profile_ref)),
            ])
        })
        .collect();
    let mut raw_observations = observations.iter().collect::<Vec<_>>();
    raw_observations.sort_by(|left, right| {
        left.subject_node_id
            .cmp(&right.subject_node_id)
            .then_with(|| left.observed_at_ticks.cmp(&right.observed_at_ticks))
            .then_with(|| left.class.cmp(&right.class))
            .then_with(|| left.detector_profile_ref.cmp(&right.detector_profile_ref))
    });
    let raw_values = raw_observations
        .into_iter()
        .map(|observation| {
            crate::preserves_rail::record("fabric-failure-observation-v1", vec![
                field("subject-node-id", crate::preserves_rail::string(&observation.subject_node_id)),
                field("class", crate::preserves_rail::string(observation.class.as_str())),
                field("observed-at-ticks", crate::preserves_rail::u64_value(observation.observed_at_ticks)),
                field("valid-until-ticks", crate::preserves_rail::u64_value(observation.valid_until_ticks)),
                field(
                    "confidence-basis-points",
                    crate::preserves_rail::u64_value(u64::from(observation.confidence_basis_points)),
                ),
                field("detector-profile-ref", crate::preserves_rail::string(&observation.detector_profile_ref)),
                field(
                    "supporting-event-refs",
                    sorted_strings_value(observation.supporting_event_refs.iter().map(String::as_str)),
                ),
            ])
        })
        .collect();
    let value = crate::preserves_rail::record(FAILURE_OBSERVATION_RECORD, vec![
        crate::preserves_rail::string(FAILURE_OBSERVATION_SCHEMA),
        field("view-ref", crate::preserves_rail::string(&view.view_ref)),
        field(
            "detector-profile-refs",
            sorted_strings_value(profiles.iter().map(|profile| profile.profile_ref.as_str())),
        ),
        field("raw-observations", crate::preserves_rail::sequence(raw_values)),
        field("reduced-observations", crate::preserves_rail::sequence(values)),
        field("non-claims", strings_value(REQUIRED_FAILURE_NON_CLAIMS.iter().map(|claim| claim.as_str()))),
        checks(&[
            "observation-only",
            "freshness-bounded",
            "membership-unchanged",
            "authority-unchanged",
        ]),
    ]);
    let observations_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalFailureObservationSet {
        observations_ref,
        observations: reduced,
        value,
    })
}
