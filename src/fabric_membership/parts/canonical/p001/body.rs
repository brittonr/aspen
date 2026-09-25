
// r[impl molten.fabric_membership.placement]
// r[impl molten.fabric_membership.authority_separation]
// r[impl molten.fabric_membership.evidence]
pub fn canonical_placement_outcome(
    view: &CanonicalMembershipView,
    request: &PlacementRequest,
) -> crate::error::Result<CanonicalPlacementOutcome> {
    let outcome =
        plan_placement(&view.admitted, request).map_err(|issues| validation_error("placement request", &issues))?;
    let outcome_value = placement_outcome_value(&outcome);
    let value = crate::preserves_rail::record(PLACEMENT_OUTCOME_RECORD, vec![
        crate::preserves_rail::string(PLACEMENT_PLAN_SCHEMA),
        field("view-ref", crate::preserves_rail::string(&view.view_ref)),
        field("view-id", crate::preserves_rail::string(&view.admitted.view.view_id)),
        field("view-epoch", crate::preserves_rail::u64_value(view.admitted.view.epoch)),
        field("policy-ref", crate::preserves_rail::string(&request.requirements.policy_ref)),
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
    let outcome_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalPlacementOutcome {
        outcome_ref,
        outcome,
        value,
    })
}

/// The planned roles and residual capacity of a plan, or the unsatisfied constraints and partial
/// selection.
fn placement_outcome_value(outcome: &PlacementOutcome) -> preserves::IOValue {
    match outcome {
        PlacementOutcome::Planned(plan) => {
            let roles = plan
                .roles
                .iter()
                .map(|role| {
                    crate::preserves_rail::record("fabric-planned-role-v1", vec![
                        field("role-ordinal", crate::preserves_rail::u64_value(u64::from(role.role_ordinal))),
                        field("node-id", crate::preserves_rail::string(&role.node_id)),
                        field("descriptor-ref", crate::preserves_rail::string(&role.descriptor_ref)),
                        field("resources", resource_value(role.resources)),
                        field("preference-score", crate::preserves_rail::u64_value(role.preference_score)),
                        field("reasons", strings_value(role.reasons.iter().map(String::as_str))),
                    ])
                })
                .collect();
            let residual = plan
                .residual_capacity
                .iter()
                .map(|(node_id, resources)| {
                    crate::preserves_rail::record("fabric-residual-capacity-v1", vec![
                        crate::preserves_rail::string(node_id),
                        resource_value(*resources),
                    ])
                })
                .collect();
            crate::preserves_rail::record("fabric-placement-plan-v1", vec![
                field("schema", crate::preserves_rail::string(&plan.schema)),
                field("roles", crate::preserves_rail::sequence(roles)),
                field("residual-capacity", crate::preserves_rail::sequence(residual)),
                field("degraded", crate::preserves_rail::bool_value(plan.degraded)),
                field("advisory-only", crate::preserves_rail::bool_value(plan.advisory_only)),
            ])
        }
        PlacementOutcome::Unsatisfied(unsatisfied) => {
            let constraints = unsatisfied
                .constraints
                .iter()
                .map(|constraint| {
                    crate::preserves_rail::record("fabric-unsatisfied-constraint-v1", vec![
                        crate::preserves_rail::string(constraint.kind.as_str()),
                        crate::preserves_rail::string(&constraint.subject),
                        crate::preserves_rail::string(&constraint.detail),
                    ])
                })
                .collect();
            crate::preserves_rail::record("fabric-unsatisfied-placement-v1", vec![
                field("constraints", crate::preserves_rail::sequence(constraints)),
                field("partial-selection", strings_value(unsatisfied.partial_selection.iter().map(String::as_str))),
            ])
        }
    }
}

// r[impl molten.modularity.fabric_boundary.compatibility]
// r[impl molten.fabric_membership.recruitment]
// r[impl molten.fabric_membership.fencing]
// r[impl molten.fabric_membership.evidence]
pub fn canonical_assignment_transition(
    transition: &AssignmentTransition,
    intent_ref: &str,
    role_effect_ref: Option<&str>,
    persistence_ref: &str,
) -> crate::error::Result<CanonicalAssignmentTransition> {
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
    let value = crate::preserves_rail::record(ASSIGNMENT_TRANSITION_RECORD, vec![
        crate::preserves_rail::string(ROLE_ASSIGNMENT_SCHEMA),
        field("assignment-id", crate::preserves_rail::string(&assignment.assignment_id)),
        field("extension-id", crate::preserves_rail::string(&assignment.extension_id)),
        field("service-id", crate::preserves_rail::string(&assignment.service_id)),
        field("role-id", crate::preserves_rail::string(&assignment.role_id)),
        field("role-kind", crate::preserves_rail::string(&assignment.role_kind)),
        field("node-id", crate::preserves_rail::string(&assignment.node_id)),
        field("service-generation", crate::preserves_rail::u64_value(assignment.service_generation)),
        field("assignment-epoch", crate::preserves_rail::u64_value(assignment.assignment_epoch)),
        field("fencing-token", crate::preserves_rail::u64_value(assignment.fencing_token)),
        field("fencing-profile-ref", crate::preserves_rail::string(&assignment.fencing_profile_ref)),
        field("resource-reservation-ref", crate::preserves_rail::string(&assignment.resource_reservation_ref)),
        field("placement-plan-ref", crate::preserves_rail::string(&assignment.placement_plan_ref)),
        field("authority-ref", crate::preserves_rail::string(&assignment.authority_ref)),
        field("previous-state", crate::preserves_rail::string(transition.previous_state.as_str())),
        field("command", crate::preserves_rail::string(transition.kind.as_str())),
        field("next-state", crate::preserves_rail::string(assignment.state.as_str())),
        field("uncertain-old-owner", crate::preserves_rail::bool_value(assignment.uncertain_old_owner)),
        field("intent-ref", crate::preserves_rail::string(intent_ref)),
        field("role-effect-ref", optional_string(role_effect_ref)),
        field("persistence-ref", crate::preserves_rail::string(persistence_ref)),
        checks(&[
            "generation-and-epoch-fenced",
            "placement-plan-remains-separate",
            "assignment-authority-explicit",
            "role-effect-and-persistence-explicit",
        ]),
    ]);
    let transition_ref = crate::preserves_rail::canonical_hash(&value)?;
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
) -> crate::error::Result<MembershipStatusReadback> {
    if assignments.len() > MAX_MEMBERSHIP_ITEMS {
        return Err(crate::error::MoltenError::invalid_harness("membership readback assignment limit exceeded"));
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
    let value = crate::preserves_rail::record(MEMBERSHIP_STATUS_RECORD, vec![
        crate::preserves_rail::string(MEMBERSHIP_EVIDENCE_SCHEMA),
        field("view-ref", crate::preserves_rail::string(&view.view_ref)),
        field("view-id", crate::preserves_rail::string(&view.admitted.view.view_id)),
        field("view-epoch", crate::preserves_rail::u64_value(view.admitted.view.epoch)),
        field("provider-kind", crate::preserves_rail::string(view.admitted.profile.provider_kind.as_str())),
        field("member-ids", strings_value(member_ids.iter().map(String::as_str))),
        field("active-assignments", crate::preserves_rail::u64_value(active_assignments)),
        field("draining-assignments", crate::preserves_rail::u64_value(draining_assignments)),
        field("uncertain-assignments", crate::preserves_rail::u64_value(uncertain_assignments)),
        field("assignment-refs", strings_value(assignment_refs.iter().map(String::as_str))),
        field("non-claims", strings_value(view.admitted.profile.non_claims.iter().map(|claim| claim.as_str()))),
        checks(&[
            "bounded-aggregate-readback",
            "secrets-excluded",
            "authority-strength-preserved",
            "observations-not-promoted",
        ]),
    ]);
    let status_ref = crate::preserves_rail::canonical_hash(&value)?;
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
