use std::collections::BTreeMap;

use preserves::IOValue;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;
use crate::fabric::DeterminismClass;
use crate::fabric::FABRIC_PORT_DESCRIPTOR_SCHEMA;
use crate::fabric::FabricAuthority;
use crate::fabric::FabricPortClass;
use crate::fabric::FabricPortDescriptor;
use crate::fabric::FabricPortKey;
use crate::fabric::FabricResource;
use crate::fabric::REQUIRED_FABRIC_NON_CLAIMS;
use crate::fabric::ReplayClass;
use crate::preserves_rail::bool_value;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::u64_value;
use crate::system_extension::SystemExtensionExecutor;
use crate::system_extension::SystemExtensionHost;

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
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalMembershipView {
    pub admitted: AdmittedMembershipView,
    pub view_ref: String,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalFailureObservationSet {
    pub observations_ref: String,
    pub observations: BTreeMap<String, ReducedFailureObservation>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalPlacementOutcome {
    pub outcome_ref: String,
    pub outcome: PlacementOutcome,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalAssignmentTransition {
    pub transition_ref: String,
    pub transition: AssignmentTransition,
    pub intent_ref: String,
    pub role_effect_ref: Option<String>,
    pub persistence_ref: String,
    pub value: IOValue,
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
    pub value: IOValue,
}

// r[impl molten.fabric_membership.membership_views]
// r[impl molten.fabric_membership.evidence]
pub fn canonical_membership_profile(profile: &MembershipSourceProfile) -> Result<CanonicalMembershipProfile> {
    let issues = validate_source_profile(profile);
    if !issues.is_empty() {
        return Err(validation_error("membership source profile", &issues));
    }
    let value = record(MEMBERSHIP_PROFILE_RECORD, vec![
        string(MEMBERSHIP_SOURCE_PROFILE_SCHEMA),
        field("profile-id", string(&profile.profile_id)),
        field("declared-profile-ref", string(&profile.profile_ref)),
        field("provider-kind", string(profile.provider_kind.as_str())),
        field("authority-strength", string(profile.authority_strength.as_str())),
        field("authority-scope", string(&profile.authority_scope)),
        field("max-view-age-ticks", u64_value(profile.max_view_age_ticks)),
        field("non-claims", strings_value(profile.non_claims.iter().map(|claim| claim.as_str()))),
        checks(&[
            "source-scope-explicit",
            "freshness-bounded",
            "authority-strength-explicit",
            "connectivity-not-membership",
            "placement-not-assignment",
        ]),
    ]);
    let admission_ref = canonical_hash(&value)?;
    Ok(CanonicalMembershipProfile {
        profile: profile.clone(),
        admission_ref,
        value,
    })
}

// r[impl molten.fabric_membership.membership_views]
// r[impl molten.fabric_membership.locality]
// r[impl molten.fabric_membership.evidence]
pub fn canonical_membership_view(
    profile: &CanonicalMembershipProfile,
    view: &MembershipView,
    descriptors: &[NodeDescriptor],
    now_ticks: u64,
    required_compatibility_ref: &str,
) -> Result<CanonicalMembershipView> {
    let admitted = validate_membership_view(&profile.profile, view, descriptors, now_ticks, required_compatibility_ref)
        .map_err(|issues| validation_error("membership view", &issues))?;
    let members = admitted
        .view
        .members
        .iter()
        .map(|member| {
            let descriptor = &admitted.descriptors[&member.node_id];
            record("fabric-membership-member-v1", vec![
                field("node-id", string(&member.node_id)),
                field("descriptor-ref", string(&member.descriptor_ref)),
                field("eligibility-ref", string(&member.eligibility_ref)),
                field("compatibility-ref", string(&descriptor.compatibility_ref)),
                field("capacity", resource_value(descriptor.capacity)),
                field(
                    "labels",
                    sequence(
                        descriptor
                            .labels
                            .iter()
                            .map(|label| {
                                record("fabric-membership-label-v1", vec![
                                    string(&label.key),
                                    string(&label.value),
                                    string(label.authority.as_str()),
                                    string(&label.evidence_ref),
                                ])
                            })
                            .collect(),
                    ),
                ),
                field("runtime-features", strings_value(descriptor.runtime_features.iter().map(String::as_str))),
            ])
        })
        .collect();
    let value = record(MEMBERSHIP_VIEW_RECORD, vec![
        string(MEMBERSHIP_VIEW_SCHEMA),
        field("profile-admission-ref", string(&profile.admission_ref)),
        field("source-profile-ref", string(&admitted.profile.profile_ref)),
        field("view-id", string(&admitted.view.view_id)),
        field("epoch", u64_value(admitted.view.epoch)),
        field("source-evidence-ref", string(&admitted.view.source_evidence_ref)),
        field("authority-ref", string(&admitted.view.authority_ref)),
        field("eligibility-policy-ref", string(&admitted.view.eligibility_policy_ref)),
        field("observed-at-ticks", u64_value(admitted.view.observed_at_ticks)),
        field("valid-until-ticks", u64_value(admitted.view.valid_until_ticks)),
        field("members", sequence(members)),
        field("non-claims", strings_value(admitted.profile.non_claims.iter().map(|claim| claim.as_str()))),
        checks(&[
            "source-scoped-snapshot",
            "members-strictly-ordered",
            "descriptor-identity-exact",
            "labels-authority-typed",
            "freshness-checked",
        ]),
    ]);
    let view_ref = canonical_hash(&value)?;
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
) -> Result<CanonicalFailureObservationSet> {
    let reduced = reduce_failure_observations(&view.admitted, profiles, observations, now_ticks)
        .map_err(|issues| validation_error("failure observations", &issues))?;
    let values = reduced
        .values()
        .map(|observation| {
            record("fabric-reduced-failure-observation-v1", vec![
                field("subject-node-id", string(&observation.subject_node_id)),
                field("class", string(observation.class.as_str())),
                field("observed-at-ticks", u64_value(observation.observed_at_ticks)),
                field("detector-profile-ref", string(&observation.detector_profile_ref)),
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
            record("fabric-failure-observation-v1", vec![
                field("subject-node-id", string(&observation.subject_node_id)),
                field("class", string(observation.class.as_str())),
                field("observed-at-ticks", u64_value(observation.observed_at_ticks)),
                field("valid-until-ticks", u64_value(observation.valid_until_ticks)),
                field("confidence-basis-points", u64_value(u64::from(observation.confidence_basis_points))),
                field("detector-profile-ref", string(&observation.detector_profile_ref)),
                field(
                    "supporting-event-refs",
                    sorted_strings_value(observation.supporting_event_refs.iter().map(String::as_str)),
                ),
            ])
        })
        .collect();
    let value = record(FAILURE_OBSERVATION_RECORD, vec![
        string(FAILURE_OBSERVATION_SCHEMA),
        field("view-ref", string(&view.view_ref)),
        field(
            "detector-profile-refs",
            sorted_strings_value(profiles.iter().map(|profile| profile.profile_ref.as_str())),
        ),
        field("raw-observations", sequence(raw_values)),
        field("reduced-observations", sequence(values)),
        field("non-claims", strings_value(REQUIRED_FAILURE_NON_CLAIMS.iter().map(|claim| claim.as_str()))),
        checks(&[
            "observation-only",
            "freshness-bounded",
            "membership-unchanged",
            "authority-unchanged",
        ]),
    ]);
    let observations_ref = canonical_hash(&value)?;
    Ok(CanonicalFailureObservationSet {
        observations_ref,
        observations: reduced,
        value,
    })
}

// r[impl molten.fabric_membership.placement]
// r[impl molten.fabric_membership.authority_separation]
// r[impl molten.fabric_membership.evidence]
pub fn canonical_placement_outcome(
    view: &CanonicalMembershipView,
    request: &PlacementRequest,
) -> Result<CanonicalPlacementOutcome> {
    let outcome =
        plan_placement(&view.admitted, request).map_err(|issues| validation_error("placement request", &issues))?;
    let outcome_value = match &outcome {
        PlacementOutcome::Planned(plan) => {
            let roles = plan
                .roles
                .iter()
                .map(|role| {
                    record("fabric-planned-role-v1", vec![
                        field("role-ordinal", u64_value(u64::from(role.role_ordinal))),
                        field("node-id", string(&role.node_id)),
                        field("descriptor-ref", string(&role.descriptor_ref)),
                        field("resources", resource_value(role.resources)),
                        field("preference-score", u64_value(role.preference_score)),
                        field("reasons", strings_value(role.reasons.iter().map(String::as_str))),
                    ])
                })
                .collect();
            let residual = plan
                .residual_capacity
                .iter()
                .map(|(node_id, resources)| {
                    record("fabric-residual-capacity-v1", vec![string(node_id), resource_value(*resources)])
                })
                .collect();
            record("fabric-placement-plan-v1", vec![
                field("schema", string(&plan.schema)),
                field("roles", sequence(roles)),
                field("residual-capacity", sequence(residual)),
                field("degraded", bool_value(plan.degraded)),
                field("advisory-only", bool_value(plan.advisory_only)),
            ])
        }
        PlacementOutcome::Unsatisfied(unsatisfied) => {
            let constraints = unsatisfied
                .constraints
                .iter()
                .map(|constraint| {
                    record("fabric-unsatisfied-constraint-v1", vec![
                        string(constraint.kind.as_str()),
                        string(&constraint.subject),
                        string(&constraint.detail),
                    ])
                })
                .collect();
            record("fabric-unsatisfied-placement-v1", vec![
                field("constraints", sequence(constraints)),
                field("partial-selection", strings_value(unsatisfied.partial_selection.iter().map(String::as_str))),
            ])
        }
    };
    let value = record(PLACEMENT_OUTCOME_RECORD, vec![
        string(PLACEMENT_PLAN_SCHEMA),
        field("view-ref", string(&view.view_ref)),
        field("view-id", string(&view.admitted.view.view_id)),
        field("view-epoch", u64_value(view.admitted.view.epoch)),
        field("policy-ref", string(&request.requirements.policy_ref)),
        field("role-requirements", role_requirements_value(&request.requirements)),
        field(
            "current-assignment-refs",
            sorted_strings_value(
                request.current_assignments.iter().map(|assignment| assignment.assignment_ref.as_str()),
            ),
        ),
        field(
            "reservation-refs",
            sorted_strings_value(
                request.current_reservations.iter().map(|reservation| reservation.reservation_ref.as_str()),
            ),
        ),
        field(
            "detector-profile-refs",
            sorted_strings_value(request.detector_profiles.iter().map(|profile| profile.profile_ref.as_str())),
        ),
        field(
            "failure-event-refs",
            sorted_strings_value(
                request
                    .failure_observations
                    .iter()
                    .flat_map(|observation| observation.supporting_event_refs.iter().map(String::as_str)),
            ),
        ),
        field("tie-break-order", strings_value(request.tie_break_order.iter().map(String::as_str))),
        field(
            "conflicting-view-refs",
            sorted_strings_value(request.conflicting_view_refs.iter().map(String::as_str)),
        ),
        field("outcome", outcome_value),
        checks(&[
            "pure-deterministic-plan",
            "tie-break-explicit",
            "capacity-residual-explicit",
            "failure-observation-not-authority",
            "plan-advisory-until-committed",
        ]),
    ]);
    let outcome_ref = canonical_hash(&value)?;
    Ok(CanonicalPlacementOutcome {
        outcome_ref,
        outcome,
        value,
    })
}

// r[impl molten.fabric_membership.recruitment]
// r[impl molten.fabric_membership.fencing]
// r[impl molten.fabric_membership.evidence]
pub fn canonical_assignment_transition(
    transition: &AssignmentTransition,
    intent_ref: &str,
    role_effect_ref: Option<&str>,
    persistence_ref: &str,
) -> Result<CanonicalAssignmentTransition> {
    let issues = validate_assignment(&transition.next);
    if !issues.is_empty() {
        return Err(validation_error("assignment transition", &issues));
    }
    validate_evidence_ref("assignment intent", intent_ref)?;
    if let Some(effect_ref) = role_effect_ref {
        validate_evidence_ref("assignment role effect", effect_ref)?;
    }
    validate_evidence_ref("assignment persistence", persistence_ref)?;
    let assignment = &transition.next;
    let value = record(ASSIGNMENT_TRANSITION_RECORD, vec![
        string(ROLE_ASSIGNMENT_SCHEMA),
        field("assignment-id", string(&assignment.assignment_id)),
        field("extension-id", string(&assignment.extension_id)),
        field("service-id", string(&assignment.service_id)),
        field("role-id", string(&assignment.role_id)),
        field("role-kind", string(&assignment.role_kind)),
        field("node-id", string(&assignment.node_id)),
        field("service-generation", u64_value(assignment.service_generation)),
        field("assignment-epoch", u64_value(assignment.assignment_epoch)),
        field("fencing-token", u64_value(assignment.fencing_token)),
        field("fencing-profile-ref", string(&assignment.fencing_profile_ref)),
        field("resource-reservation-ref", string(&assignment.resource_reservation_ref)),
        field("placement-plan-ref", string(&assignment.placement_plan_ref)),
        field("authority-ref", string(&assignment.authority_ref)),
        field("previous-state", string(transition.previous_state.as_str())),
        field("command", string(transition.kind.as_str())),
        field("next-state", string(assignment.state.as_str())),
        field("uncertain-old-owner", bool_value(assignment.uncertain_old_owner)),
        field("intent-ref", string(intent_ref)),
        field("role-effect-ref", optional_string(role_effect_ref)),
        field("persistence-ref", string(persistence_ref)),
        checks(&[
            "generation-and-epoch-fenced",
            "placement-plan-remains-separate",
            "assignment-authority-explicit",
            "role-effect-and-persistence-explicit",
        ]),
    ]);
    let transition_ref = canonical_hash(&value)?;
    Ok(CanonicalAssignmentTransition {
        transition_ref,
        transition: transition.clone(),
        intent_ref: intent_ref.to_string(),
        role_effect_ref: role_effect_ref.map(str::to_string),
        persistence_ref: persistence_ref.to_string(),
        value,
    })
}

// r[impl molten.fabric_membership.evidence]
pub fn membership_status_readback(
    view: &CanonicalMembershipView,
    assignments: &[(String, RoleAssignment)],
) -> Result<MembershipStatusReadback> {
    if assignments.len() > MAX_MEMBERSHIP_ITEMS {
        return Err(MoltenError::invalid_harness("membership readback assignment limit exceeded"));
    }
    let member_ids = view.admitted.view.members.iter().map(|member| member.node_id.clone()).collect::<Vec<_>>();
    let mut active_assignments = 0u64;
    let mut draining_assignments = 0u64;
    let mut uncertain_assignments = 0u64;
    let mut assignment_refs = Vec::with_capacity(assignments.len());
    for (assignment_ref, assignment) in assignments {
        validate_evidence_ref("assignment readback", assignment_ref)?;
        let issues = validate_assignment(assignment);
        if !issues.is_empty() {
            return Err(validation_error("assignment readback", &issues));
        }
        match assignment.state {
            AssignmentState::Active => active_assignments = checked_increment(active_assignments)?,
            AssignmentState::Draining | AssignmentState::Replacing => {
                draining_assignments = checked_increment(draining_assignments)?;
            }
            _ => {}
        }
        if assignment.uncertain_old_owner {
            uncertain_assignments = checked_increment(uncertain_assignments)?;
        }
        assignment_refs.push(assignment_ref.clone());
    }
    assignment_refs.sort();
    let value = record(MEMBERSHIP_STATUS_RECORD, vec![
        string(MEMBERSHIP_EVIDENCE_SCHEMA),
        field("view-ref", string(&view.view_ref)),
        field("view-id", string(&view.admitted.view.view_id)),
        field("view-epoch", u64_value(view.admitted.view.epoch)),
        field("provider-kind", string(view.admitted.profile.provider_kind.as_str())),
        field("member-ids", strings_value(member_ids.iter().map(String::as_str))),
        field("active-assignments", u64_value(active_assignments)),
        field("draining-assignments", u64_value(draining_assignments)),
        field("uncertain-assignments", u64_value(uncertain_assignments)),
        field("assignment-refs", strings_value(assignment_refs.iter().map(String::as_str))),
        field("non-claims", strings_value(view.admitted.profile.non_claims.iter().map(|claim| claim.as_str()))),
        checks(&[
            "bounded-aggregate-readback",
            "secrets-excluded",
            "authority-strength-preserved",
            "observations-not-promoted",
        ]),
    ]);
    let status_ref = canonical_hash(&value)?;
    Ok(MembershipStatusReadback {
        status_ref,
        view_ref: view.view_ref.clone(),
        view_id: view.admitted.view.view_id.clone(),
        view_epoch: view.admitted.view.epoch,
        provider_kind: view.admitted.profile.provider_kind,
        member_ids,
        active_assignments,
        draining_assignments,
        uncertain_assignments,
        assignment_refs,
        non_claims: view.admitted.profile.non_claims.clone(),
        value,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionMembershipPlacementContext {
    service_id: String,
    generation: u64,
    profile_id: String,
    source_profile_ref: String,
    bound_ports: Vec<String>,
}

impl ExtensionMembershipPlacementContext {
    pub fn from_host<E: SystemExtensionExecutor>(
        host: &SystemExtensionHost<E>,
        profile: &CanonicalMembershipProfile,
    ) -> Result<Self> {
        let mut bound_ports = Vec::with_capacity(MEMBERSHIP_PORT_COUNT);
        for port_id in membership_port_ids() {
            let key = FabricPortKey {
                port_id: port_id.to_string(),
                version: FABRIC_MEMBERSHIP_PORT_VERSION.to_string(),
            };
            if let Some(binding) = host.manifest().binding_for(&key) {
                if binding.binding.implementation_profile != profile.profile.profile_id {
                    return Err(MoltenError::invalid_harness(format!(
                        "system-extension membership profile {} does not match {}",
                        binding.binding.implementation_profile, profile.profile.profile_id
                    )));
                }
                bound_ports.push(port_id.to_string());
            }
        }
        if bound_ports.is_empty() {
            return Err(MoltenError::invalid_harness(
                "system extension has no admitted membership or placement fabric port binding",
            ));
        }
        Ok(Self {
            service_id: host.manifest().manifest().service_id.clone(),
            generation: host.state().generation,
            profile_id: profile.profile.profile_id.clone(),
            source_profile_ref: profile.profile.profile_ref.clone(),
            bound_ports,
        })
    }

    #[cfg(test)]
    pub(crate) fn from_test_snapshot(
        service_id: &str,
        generation: u64,
        profile: &CanonicalMembershipProfile,
        bound_ports: Vec<String>,
    ) -> Self {
        Self {
            service_id: service_id.to_string(),
            generation,
            profile_id: profile.profile.profile_id.clone(),
            source_profile_ref: profile.profile.profile_ref.clone(),
            bound_ports,
        }
    }

    pub fn admit_plan(
        &self,
        profile: &CanonicalMembershipProfile,
        view: &CanonicalMembershipView,
        service_id: &str,
        generation: u64,
    ) -> Result<()> {
        self.admit_scope(profile, FABRIC_PLACEMENT_PORT_ID, service_id, generation)?;
        if view.admitted.profile.profile_ref != self.source_profile_ref {
            return Err(MoltenError::invalid_harness("placement view uses a substituted membership source profile"));
        }
        Ok(())
    }

    pub fn admit_assignment(&self, profile: &CanonicalMembershipProfile, assignment: &RoleAssignment) -> Result<()> {
        self.admit_scope(profile, FABRIC_ASSIGNMENT_PORT_ID, &assignment.service_id, assignment.service_generation)?;
        let issues = validate_assignment(assignment);
        if !issues.is_empty() {
            return Err(validation_error("extension assignment", &issues));
        }
        Ok(())
    }

    fn admit_scope(
        &self,
        profile: &CanonicalMembershipProfile,
        port_id: &str,
        service_id: &str,
        generation: u64,
    ) -> Result<()> {
        if self.profile_id != profile.profile.profile_id {
            return Err(MoltenError::invalid_harness("membership profile substitution denied"));
        }
        if !self.bound_ports.iter().any(|bound| bound == port_id) {
            return Err(MoltenError::invalid_harness(format!(
                "membership or placement port {port_id} is not bound to the system extension"
            )));
        }
        if self.service_id != service_id {
            return Err(MoltenError::invalid_harness("membership service identity mismatch"));
        }
        if self.generation != generation {
            return Err(MoltenError::invalid_harness(
                "membership or placement operation uses a stale service generation",
            ));
        }
        Ok(())
    }
}

// r[impl molten.fabric_membership.live_sim_parity]
pub fn fabric_membership_port_descriptors(profile: &CanonicalMembershipProfile) -> Vec<FabricPortDescriptor> {
    let (provider_determinism, provider_replay) = match profile.profile.provider_kind {
        MembershipProviderKind::DeterministicSimulation => {
            (DeterminismClass::DeterministicWithRecordedInputs, ReplayClass::Recompute)
        }
        MembershipProviderKind::Static => (DeterminismClass::DeterministicWithRecordedInputs, ReplayClass::Recompute),
        MembershipProviderKind::PolicyManaged | MembershipProviderKind::ConsistencyBacked => {
            (DeterminismClass::ExternalEffect, ReplayClass::RecordedEffectRequired)
        }
    };
    let definitions = [
        (
            FABRIC_MEMBERSHIP_PORT_ID,
            FabricPortClass::Membership,
            vec!["snapshot", "eligible-members", "readback"],
            MEMBERSHIP_VIEW_SCHEMA,
            provider_determinism,
            provider_replay,
            vec![FabricAuthority::Membership, FabricAuthority::Policy],
        ),
        (
            FABRIC_FAILURE_OBSERVATION_PORT_ID,
            FabricPortClass::Membership,
            vec!["observe", "reduce", "readback"],
            FAILURE_OBSERVATION_SCHEMA,
            provider_determinism,
            provider_replay,
            vec![FabricAuthority::Membership, FabricAuthority::Time],
        ),
        (
            FABRIC_PLACEMENT_PORT_ID,
            FabricPortClass::Placement,
            vec!["plan", "explain", "compare"],
            PLACEMENT_PLAN_SCHEMA,
            DeterminismClass::Pure,
            ReplayClass::Recompute,
            vec![
                FabricAuthority::Placement,
                FabricAuthority::Policy,
                FabricAuthority::Resources,
            ],
        ),
        (
            FABRIC_ASSIGNMENT_PORT_ID,
            FabricPortClass::Placement,
            vec![
                "propose",
                "reserve",
                "assign",
                "acknowledge",
                "activate",
                "drain",
                "replace",
                "release",
            ],
            ROLE_ASSIGNMENT_SCHEMA,
            DeterminismClass::ExternalEffect,
            ReplayClass::RecordedEffectRequired,
            vec![
                FabricAuthority::Placement,
                FabricAuthority::Supervision,
                FabricAuthority::DurableState,
            ],
        ),
    ];
    let mut descriptors = Vec::with_capacity(MEMBERSHIP_PORT_COUNT);
    for (port_id, class, operations, output_schema, determinism, replay, authorities) in definitions {
        descriptors.push(FabricPortDescriptor {
            schema: FABRIC_PORT_DESCRIPTOR_SCHEMA.to_string(),
            port_id: port_id.to_string(),
            version: FABRIC_MEMBERSHIP_PORT_VERSION.to_string(),
            class,
            operation_classes: operations.into_iter().map(str::to_string).collect(),
            input_schema_refs: vec![MEMBERSHIP_SOURCE_PROFILE_SCHEMA.to_string()],
            output_schema_refs: vec![output_schema.to_string()],
            authority_requirements: authorities,
            resource_requirements: vec![FabricResource::Memory, FabricResource::LogicalTime],
            determinism,
            replay,
            implementation_profile: profile.profile.profile_id.clone(),
            conformance_refs: vec![profile.admission_ref.clone(), profile.profile.profile_ref.clone()],
            non_claims: REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
            enabled: true,
        });
    }
    descriptors
}

fn membership_port_ids() -> [&'static str; MEMBERSHIP_PORT_COUNT] {
    [
        FABRIC_MEMBERSHIP_PORT_ID,
        FABRIC_FAILURE_OBSERVATION_PORT_ID,
        FABRIC_PLACEMENT_PORT_ID,
        FABRIC_ASSIGNMENT_PORT_ID,
    ]
}

fn role_requirements_value(requirements: &RoleRequirements) -> IOValue {
    let required_labels = requirements
        .required_labels
        .iter()
        .map(|constraint| {
            record("fabric-required-label-v1", vec![
                string(&constraint.key),
                optional_string(constraint.value.as_deref()),
                string(constraint.minimum_authority.as_str()),
            ])
        })
        .collect();
    let preferred_labels = requirements
        .preferred_labels
        .iter()
        .map(|preference| {
            record("fabric-preferred-label-v1", vec![
                string(&preference.key),
                string(&preference.value),
                string(preference.minimum_authority.as_str()),
                u64_value(u64::from(preference.weight)),
            ])
        })
        .collect();
    record("fabric-role-requirements-v1", vec![
        field("extension-id", string(&requirements.extension_id)),
        field("service-id", string(&requirements.service_id)),
        field("role-kind", string(&requirements.role_kind)),
        field("replica-count", u64_value(u64::from(requirements.replica_count))),
        field("per-replica", resource_value(requirements.per_replica)),
        field("required-features", strings_value(requirements.required_features.iter().map(String::as_str))),
        field("required-labels", sequence(required_labels)),
        field("preferred-labels", sequence(preferred_labels)),
        field(
            "anti-affinity-label-keys",
            strings_value(requirements.anti_affinity_label_keys.iter().map(String::as_str)),
        ),
        field("distinct-nodes", bool_value(requirements.distinct_nodes)),
        field("avoid-suspected", bool_value(requirements.avoid_suspected)),
        field("allow-degraded", bool_value(requirements.allow_degraded)),
    ])
}

fn resource_value(resources: ResourceAmount) -> IOValue {
    record("fabric-resource-amount-v1", vec![
        field("cpu-millis", u64_value(resources.cpu_millis)),
        field("memory-bytes", u64_value(resources.memory_bytes)),
        field("storage-bytes", u64_value(resources.storage_bytes)),
    ])
}

fn validate_evidence_ref(label: &str, value: &str) -> Result<()> {
    const BLAKE3_PREFIX: &str = "blake3:";
    const BLAKE3_HEX_LENGTH: usize = 64;
    let is_valid = value.strip_prefix(BLAKE3_PREFIX).is_some_and(|hex| {
        hex.len() == BLAKE3_HEX_LENGTH && hex.chars().all(|character| matches!(character, '0'..='9' | 'a'..='f'))
    });
    if is_valid {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} ref is malformed")))
    }
}

fn checked_increment(value: u64) -> Result<u64> {
    value
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness("membership readback count overflow"))
}

fn validation_error<T: std::fmt::Debug>(label: &str, issues: &[T]) -> MoltenError {
    MoltenError::invalid_harness(format!("{label} validation failed: {issues:?}"))
}

fn field(name: &str, value: IOValue) -> IOValue {
    record("field", vec![string(name), value])
}

fn strings_value<'a>(values: impl IntoIterator<Item = &'a str>) -> IOValue {
    sequence(values.into_iter().map(string).collect())
}

fn optional_string(value: Option<&str>) -> IOValue {
    value.map_or_else(|| sequence(Vec::new()), |value| sequence(vec![string(value)]))
}

fn sorted_strings_value<'a>(values: impl IntoIterator<Item = &'a str>) -> IOValue {
    let mut values = values.into_iter().collect::<Vec<_>>();
    values.sort_unstable();
    strings_value(values)
}

fn checks(values: &[&str]) -> IOValue {
    strings_value(values.iter().copied())
}
