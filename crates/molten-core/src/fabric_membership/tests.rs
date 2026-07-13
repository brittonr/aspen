use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

use super::*;

const SYNTHETIC_REF_CHUNK_HEX_CHARS: usize = 16;
const SYNTHETIC_REF_CHUNK_REPETITIONS: usize = 4;
const NOW: u64 = 100;
const OBSERVED_AT: u64 = 90;
const VALID_UNTIL: u64 = 110;
const MAX_AGE: u64 = 20;
const VIEW_EPOCH: u64 = 7;
const SERVICE_GENERATION: u64 = 3;
const ASSIGNMENT_EPOCH: u64 = 11;
const SUCCESSOR_EPOCH: u64 = 12;
const FENCING_TOKEN: u64 = 41;
const SUCCESSOR_TOKEN: u64 = 42;
const CPU_CAPACITY: u64 = 2_000;
const MEMORY_CAPACITY: u64 = 8_000;
const STORAGE_CAPACITY: u64 = 16_000;
const CPU_REQUEST: u64 = 500;
const MEMORY_REQUEST: u64 = 1_000;
const STORAGE_REQUEST: u64 = 2_000;
const REPLICA_COUNT: u32 = 2;
const PREFERENCE_WEIGHT: u32 = 10;
const HIGH_CONFIDENCE: u16 = 9_000;
const GRACE_DEADLINE: u64 = 120;
const AFTER_GRACE: u64 = 121;

fn test_ref(label: &str) -> String {
    let mut hasher = DefaultHasher::new();
    label.hash(&mut hasher);
    let digest = hasher.finish();
    let chunk = format!("{digest:0width$x}", width = SYNTHETIC_REF_CHUNK_HEX_CHARS);
    format!("blake3:{}", chunk.repeat(SYNTHETIC_REF_CHUNK_REPETITIONS))
}

fn profile(kind: MembershipProviderKind) -> MembershipSourceProfile {
    MembershipSourceProfile {
        schema: MEMBERSHIP_SOURCE_PROFILE_SCHEMA.to_string(),
        profile_id: "membership-profile-v1".to_string(),
        profile_ref: test_ref("membership-profile"),
        provider_kind: kind,
        authority_strength: MembershipAuthorityStrength::OperatorDeclared,
        authority_scope: "cluster-a".to_string(),
        max_view_age_ticks: MAX_AGE,
        non_claims: REQUIRED_MEMBERSHIP_NON_CLAIMS.to_vec(),
    }
}

fn label(key: &str, value: &str, authority: LabelAuthority, evidence_label: &str) -> NodeLabel {
    NodeLabel {
        key: key.to_string(),
        value: value.to_string(),
        authority,
        evidence_ref: test_ref(evidence_label),
    }
}

fn descriptor(node_id: &str, zone: &str, descriptor_label: &str) -> NodeDescriptor {
    NodeDescriptor {
        schema: NODE_DESCRIPTOR_SCHEMA.to_string(),
        node_id: node_id.to_string(),
        descriptor_ref: test_ref(descriptor_label),
        compatibility_ref: test_ref("compatibility-v1"),
        labels: vec![
            label("region", "east", LabelAuthority::OperatorDeclared, &format!("{descriptor_label}-region")),
            label("zone", zone, LabelAuthority::Authoritative, &format!("{descriptor_label}-zone")),
        ],
        runtime_features: vec!["system-extension-v1".to_string()],
        capacity: ResourceAmount {
            cpu_millis: CPU_CAPACITY,
            memory_bytes: MEMORY_CAPACITY,
            storage_bytes: STORAGE_CAPACITY,
        },
    }
}

fn descriptors() -> Vec<NodeDescriptor> {
    vec![
        descriptor("node-a", "zone-a", "descriptor-node-a"),
        descriptor("node-b", "zone-b", "descriptor-node-b"),
        descriptor("node-c", "zone-a", "descriptor-node-c"),
    ]
}

fn view(profile: &MembershipSourceProfile, descriptors: &[NodeDescriptor]) -> MembershipView {
    MembershipView {
        schema: MEMBERSHIP_VIEW_SCHEMA.to_string(),
        view_id: "view-a".to_string(),
        epoch: VIEW_EPOCH,
        source_profile_ref: profile.profile_ref.clone(),
        source_evidence_ref: test_ref("membership-source-evidence"),
        authority_ref: test_ref("membership-authority"),
        eligibility_policy_ref: test_ref("membership-eligibility-policy"),
        observed_at_ticks: OBSERVED_AT,
        valid_until_ticks: VALID_UNTIL,
        members: descriptors
            .iter()
            .map(|descriptor| MembershipMember {
                node_id: descriptor.node_id.clone(),
                descriptor_ref: descriptor.descriptor_ref.clone(),
                eligibility_ref: test_ref(&format!("eligibility-{}", descriptor.node_id)),
            })
            .collect(),
    }
}

fn admitted() -> AdmittedMembershipView {
    let profile = profile(MembershipProviderKind::Static);
    let descriptors = descriptors();
    let view = view(&profile, &descriptors);
    validate_membership_view(&profile, &view, &descriptors, NOW, &test_ref("compatibility-v1"))
        .expect("admitted membership view")
}

fn detector_profile() -> FailureDetectorProfile {
    FailureDetectorProfile {
        profile_id: "detector-v1".to_string(),
        profile_ref: test_ref("failure-detector-profile"),
        time_basis_ref: test_ref("failure-detector-time-basis"),
        max_observation_age_ticks: MAX_AGE,
        non_claims: REQUIRED_FAILURE_NON_CLAIMS.to_vec(),
    }
}

fn observation(node_id: &str, class: FailureObservationClass, event_label: &str) -> FailureObservation {
    FailureObservation {
        schema: FAILURE_OBSERVATION_SCHEMA.to_string(),
        subject_node_id: node_id.to_string(),
        detector_profile_ref: test_ref("failure-detector-profile"),
        class,
        observed_at_ticks: OBSERVED_AT,
        valid_until_ticks: VALID_UNTIL,
        confidence_basis_points: HIGH_CONFIDENCE,
        supporting_event_refs: vec![test_ref(event_label)],
    }
}

fn requirements() -> RoleRequirements {
    RoleRequirements {
        extension_id: "extension-a".to_string(),
        service_id: "service-a".to_string(),
        role_kind: "replica".to_string(),
        replica_count: REPLICA_COUNT,
        per_replica: ResourceAmount {
            cpu_millis: CPU_REQUEST,
            memory_bytes: MEMORY_REQUEST,
            storage_bytes: STORAGE_REQUEST,
        },
        required_features: vec!["system-extension-v1".to_string()],
        required_labels: vec![HardLabelConstraint {
            key: "region".to_string(),
            value: Some("east".to_string()),
            minimum_authority: LabelAuthority::OperatorDeclared,
        }],
        preferred_labels: vec![PreferredLabel {
            key: "zone".to_string(),
            value: "zone-b".to_string(),
            minimum_authority: LabelAuthority::Authoritative,
            weight: PREFERENCE_WEIGHT,
        }],
        anti_affinity_label_keys: vec!["zone".to_string()],
        distinct_nodes: true,
        avoid_suspected: true,
        allow_degraded: false,
        policy_ref: test_ref("placement-policy"),
    }
}

fn placement_request() -> PlacementRequest {
    PlacementRequest {
        requirements: requirements(),
        current_assignments: Vec::new(),
        current_reservations: Vec::new(),
        failure_observations: Vec::new(),
        detector_profiles: vec![detector_profile()],
        tie_break_order: vec!["node-b".to_string(), "node-a".to_string(), "node-c".to_string()],
        conflicting_view_refs: Vec::new(),
        now_ticks: NOW,
    }
}

fn proposal(predecessor: bool) -> AssignmentProposal {
    AssignmentProposal {
        assignment_id: if predecessor { "assignment-b" } else { "assignment-a" }.to_string(),
        extension_id: "extension-a".to_string(),
        service_id: "service-a".to_string(),
        role_id: "replica-0".to_string(),
        role_kind: "replica".to_string(),
        node_id: if predecessor { "node-b" } else { "node-a" }.to_string(),
        service_generation: SERVICE_GENERATION,
        assignment_epoch: if predecessor { SUCCESSOR_EPOCH } else { ASSIGNMENT_EPOCH },
        fencing_token: if predecessor { SUCCESSOR_TOKEN } else { FENCING_TOKEN },
        fencing_profile_ref: test_ref("fencing-profile"),
        resource_reservation_ref: test_ref("resource-reservation"),
        placement_plan_ref: test_ref("placement-plan"),
        authority_ref: test_ref("assignment-authority"),
        durable_state_ref: Some(test_ref("assignment-durable-state")),
        predecessor_assignment_ref: predecessor.then(|| test_ref("predecessor-assignment")),
        predecessor_epoch: predecessor.then_some(ASSIGNMENT_EPOCH),
    }
}

fn command(assignment: &RoleAssignment, kind: AssignmentCommandKind, transition_label: &str) -> AssignmentCommand {
    AssignmentCommand {
        kind,
        assignment_id: assignment.assignment_id.clone(),
        service_generation: assignment.service_generation,
        assignment_epoch: assignment.assignment_epoch,
        fencing_token: assignment.fencing_token,
        transition_ref: test_ref(transition_label),
        uncertain_old_owner: false,
    }
}

fn active_assignment() -> RoleAssignment {
    let mut assignment = propose_assignment(&proposal(false)).expect("proposal");
    for (kind, transition_label) in [
        (AssignmentCommandKind::Reserve, "reserve"),
        (AssignmentCommandKind::Assign, "assign"),
        (AssignmentCommandKind::Acknowledge, "acknowledge"),
        (AssignmentCommandKind::Activate, "activate"),
    ] {
        assignment = apply_assignment_command(&assignment, &command(&assignment, kind, transition_label))
            .expect("valid assignment transition")
            .next;
    }
    assignment
}

// r[verify molten.fabric_membership.membership_views]
// r[verify molten.fabric_membership.locality]
#[test]
fn source_scoped_view_admits_sorted_compatible_descriptors_and_rejects_stale_or_advisory_inputs() {
    let profile = profile(MembershipProviderKind::Static);
    let descriptors = descriptors();
    let view = view(&profile, &descriptors);
    let admitted = validate_membership_view(&profile, &view, &descriptors, NOW, &test_ref("compatibility-v1"))
        .expect("valid view");
    assert_eq!(admitted.view.epoch, VIEW_EPOCH);
    assert_eq!(admitted.descriptors.len(), descriptors.len());

    let stale_now = VALID_UNTIL + 1;
    let stale = validate_membership_view(&profile, &view, &descriptors, stale_now, &test_ref("compatibility-v1"))
        .expect_err("stale view denied");
    assert!(stale.contains(&MembershipIssue::StaleView));

    let mut duplicate = view.clone();
    duplicate.members.swap(0, 1);
    let unordered = validate_membership_view(&profile, &duplicate, &descriptors, NOW, &test_ref("compatibility-v1"))
        .expect_err("unordered members denied");
    assert!(unordered.contains(&MembershipIssue::MembersNotStrictlyOrdered));

    let mut incompatible = descriptors.clone();
    incompatible[0].compatibility_ref = test_ref("incompatible-profile");
    let mismatch = validate_membership_view(&profile, &view, &incompatible, NOW, &test_ref("compatibility-v1"))
        .expect_err("incompatible descriptor denied");
    assert!(mismatch.contains(&MembershipIssue::CompatibilityMismatch("node-a".to_string())));
}

// r[verify molten.fabric_membership.failure_detector]
#[test]
fn failure_observations_reduce_deterministically_without_mutating_membership() {
    let admitted = admitted();
    let profiles = vec![detector_profile()];
    let observations = vec![
        observation("node-a", FailureObservationClass::Reachable, "reachable-node-a"),
        observation("node-a", FailureObservationClass::Suspected, "suspected-node-a"),
    ];
    let reduced = reduce_failure_observations(&admitted, &profiles, &observations, NOW).expect("observations reduce");
    assert_eq!(reduced["node-a"].class, FailureObservationClass::Suspected);
    assert_eq!(admitted.view.members.len(), descriptors().len());

    let unknown = vec![observation(
        "node-z",
        FailureObservationClass::Unavailable,
        "unavailable-node-z",
    )];
    let issues = reduce_failure_observations(&admitted, &profiles, &unknown, NOW).expect_err("unknown subject denied");
    assert!(issues.contains(&MembershipIssue::UnknownObservationSubject("node-z".to_string())));
}

// r[verify molten.fabric_membership.placement]
// r[verify molten.fabric_membership.authority_separation]
#[test]
fn placement_is_deterministic_capacity_bounded_and_advisory() {
    let admitted = admitted();
    let request = placement_request();
    let first = plan_placement(&admitted, &request).expect("plan");
    let second = plan_placement(&admitted, &request).expect("repeat plan");
    assert_eq!(first, second);
    let PlacementOutcome::Planned(plan) = first else {
        panic!("expected planned placement")
    };
    assert!(plan.advisory_only);
    assert!(!plan.degraded);
    assert_eq!(plan.roles.len(), usize::try_from(REPLICA_COUNT).expect("small replica count"));
    assert_eq!(plan.roles[0].node_id, "node-b");
    assert_ne!(
        admitted.descriptors[&plan.roles[0].node_id].label("zone").expect("zone").value,
        admitted.descriptors[&plan.roles[1].node_id].label("zone").expect("zone").value
    );
    assert_eq!(plan.residual_capacity["node-b"].cpu_millis, CPU_CAPACITY - CPU_REQUEST);

    let mut with_current_assignment = placement_request();
    with_current_assignment.requirements.replica_count = 1;
    with_current_assignment.current_assignments = vec![CurrentRoleAssignment {
        assignment_ref: test_ref("current-assignment-node-b"),
        node_id: "node-b".to_string(),
        service_id: "service-a".to_string(),
        role_kind: "replica".to_string(),
        assignment_epoch: ASSIGNMENT_EPOCH,
        active: true,
    }];
    let PlacementOutcome::Planned(with_current_plan) =
        plan_placement(&admitted, &with_current_assignment).expect("plan around current assignment")
    else {
        panic!("current assignment should leave one distinct zone")
    };
    assert_eq!(with_current_plan.roles[0].node_id, "node-a");
}

// r[verify molten.fabric_membership.placement]
#[test]
fn split_views_weak_labels_suspicion_and_capacity_fail_closed() {
    let admitted = admitted();
    let mut split = placement_request();
    split.conflicting_view_refs = vec![test_ref("conflicting-view")];
    let PlacementOutcome::Unsatisfied(split_outcome) = plan_placement(&admitted, &split).expect("split outcome") else {
        panic!("split views must not plan")
    };
    assert_eq!(split_outcome.constraints[0].kind, UnsatisfiedConstraintKind::ConflictingViews);

    let mut suspected = placement_request();
    suspected.failure_observations = vec![
        observation("node-a", FailureObservationClass::Suspected, "suspected-node-a"),
        observation("node-b", FailureObservationClass::Unavailable, "unavailable-node-b"),
    ];
    let PlacementOutcome::Unsatisfied(suspected_outcome) =
        plan_placement(&admitted, &suspected).expect("suspected outcome")
    else {
        panic!("failure policy must deny insufficient candidates")
    };
    assert!(
        suspected_outcome
            .constraints
            .iter()
            .any(|constraint| constraint.kind == UnsatisfiedConstraintKind::FailurePolicy)
    );

    let mut advisory = admitted.clone();
    advisory.descriptors.get_mut("node-a").expect("node-a").labels[0].authority = LabelAuthority::Advisory;
    advisory.descriptors.get_mut("node-b").expect("node-b").labels[0].authority = LabelAuthority::Advisory;
    advisory.descriptors.get_mut("node-c").expect("node-c").labels[0].authority = LabelAuthority::Advisory;
    let PlacementOutcome::Unsatisfied(label_outcome) =
        plan_placement(&advisory, &placement_request()).expect("label outcome")
    else {
        panic!("advisory labels must not satisfy hard authority")
    };
    assert!(
        label_outcome
            .constraints
            .iter()
            .any(|constraint| constraint.kind == UnsatisfiedConstraintKind::RequiredLabel)
    );

    let mut exhausted = placement_request();
    exhausted.current_reservations = admitted
        .view
        .members
        .iter()
        .map(|member| CapacityReservation {
            reservation_ref: test_ref(&format!("reservation-{}", member.node_id)),
            node_id: member.node_id.clone(),
            resources: admitted.descriptors[&member.node_id].capacity,
            assignment_epoch: ASSIGNMENT_EPOCH,
            released: false,
        })
        .collect();
    let PlacementOutcome::Unsatisfied(capacity_outcome) =
        plan_placement(&admitted, &exhausted).expect("capacity outcome")
    else {
        panic!("exhausted capacity must not plan")
    };
    assert!(
        capacity_outcome
            .constraints
            .iter()
            .any(|constraint| constraint.kind == UnsatisfiedConstraintKind::InsufficientCapacity)
    );
}

// r[verify molten.fabric_membership.recruitment]
// r[verify molten.fabric_membership.fencing]
#[test]
fn recruitment_requires_ack_and_rejects_delayed_or_stale_fenced_work() {
    let active = active_assignment();
    assert_eq!(active.state, AssignmentState::Active);

    let draining =
        apply_assignment_command(&active, &command(&active, AssignmentCommandKind::BeginDrain, "begin-drain"))
            .expect("begin drain")
            .next;
    let released = apply_assignment_command(&draining, &command(&draining, AssignmentCommandKind::Release, "release"))
        .expect("release")
        .next;
    let delayed = apply_assignment_command(
        &released,
        &command(&released, AssignmentCommandKind::Acknowledge, "delayed-acknowledge"),
    )
    .expect_err("delayed acknowledgement denied");
    assert!(delayed.contains(&AssignmentIssue::InvalidTransition {
        state: AssignmentState::Released,
        command: AssignmentCommandKind::Acknowledge,
    }));

    let profile = FencingProfile {
        schema: FENCING_PROFILE_SCHEMA.to_string(),
        profile_id: "process-fence-v1".to_string(),
        profile_ref: test_ref("fencing-profile"),
        authority_ref: test_ref("assignment-authority"),
        effect_port_ref: test_ref("fenced-effect-port"),
        enforcement: FencingEnforcement::ProcessLocal,
    };
    let operation = FencedOperation {
        assignment_id: active.assignment_id.clone(),
        service_generation: active.service_generation,
        assignment_epoch: active.assignment_epoch,
        fencing_token: active.fencing_token,
        fencing_profile_ref: active.fencing_profile_ref.clone(),
        authority_ref: active.authority_ref.clone(),
        required_enforcement: FencingEnforcement::QuorumOrdered,
    };
    let weak = validate_fenced_operation(&active, &profile, &operation, active.assignment_epoch, active.fencing_token)
        .expect_err("weak fencing overclaim denied");
    assert!(weak.contains(&AssignmentIssue::FencingStrengthInsufficient {
        available: FencingEnforcement::ProcessLocal,
        required: FencingEnforcement::QuorumOrdered,
    }));

    let stale = validate_fenced_operation(&active, &profile, &operation, SUCCESSOR_EPOCH, SUCCESSOR_TOKEN)
        .expect_err("stale assignment denied");
    assert!(stale.iter().any(|issue| matches!(issue, AssignmentIssue::StaleAssignmentEpoch { .. })));
    assert!(stale.iter().any(|issue| matches!(issue, AssignmentIssue::StaleFencingToken { .. })));

    let successor = propose_assignment(&proposal(true)).expect("successor advances epoch");
    assert_eq!(successor.predecessor_epoch, Some(ASSIGNMENT_EPOCH));
}

// r[verify molten.fabric_membership.drain_replace]
#[test]
fn planned_drain_and_failed_replacement_keep_uncertainty_explicit() {
    let active = active_assignment();
    let draining =
        apply_assignment_command(&active, &command(&active, AssignmentCommandKind::BeginDrain, "planned-drain"))
            .expect("drain")
            .next;
    let waiting = DrainProgress {
        assignment_id: draining.assignment_id.clone(),
        assignment_epoch: draining.assignment_epoch,
        new_work_stopped: true,
        handoff_required: true,
        handoff_complete: false,
        checkpoint_ref: None,
        role_stopped: false,
        release_acknowledged: false,
        grace_deadline_ticks: GRACE_DEADLINE,
        now_ticks: NOW,
    };
    assert_eq!(evaluate_drain(&draining, &waiting).expect("wait"), DrainDecision::Continue);
    let timed_out = DrainProgress {
        now_ticks: AFTER_GRACE,
        ..waiting
    };
    assert_eq!(
        evaluate_drain(&draining, &timed_out).expect("bounded timeout"),
        DrainDecision::ForceReleaseUncertain
    );

    let released_assignment =
        apply_assignment_command(&draining, &command(&draining, AssignmentCommandKind::Release, "drain-release"))
            .expect("release after drain")
            .next;
    let reservation = CapacityReservation {
        reservation_ref: released_assignment.resource_reservation_ref.clone(),
        node_id: released_assignment.node_id.clone(),
        resources: ResourceAmount {
            cpu_millis: CPU_REQUEST,
            memory_bytes: MEMORY_REQUEST,
            storage_bytes: STORAGE_REQUEST,
        },
        assignment_epoch: released_assignment.assignment_epoch,
        released: false,
    };
    let released_reservation = release_capacity_reservation(&reservation, &released_assignment)
        .expect("released assignment frees reservation");
    assert!(released_reservation.released);
    let mut wrong_reservation = reservation;
    wrong_reservation.node_id = "node-b".to_string();
    assert_eq!(
        release_capacity_reservation(&wrong_reservation, &released_assignment),
        Err(AssignmentIssue::ResourceReservationMismatch)
    );

    let mut replace = command(&draining, AssignmentCommandKind::BeginReplacement, "failure-replacement");
    replace.uncertain_old_owner = true;
    let replacing = apply_assignment_command(&draining, &replace).expect("replace uncertain owner").next;
    assert_eq!(replacing.state, AssignmentState::Replacing);
    assert!(replacing.uncertain_old_owner);
}

// r[verify molten.fabric_membership.live_sim_parity]
#[test]
fn provider_parity_detects_drift_without_cross_profile_state_leakage() {
    let live = admitted();
    let mut simulation = live.clone();
    simulation.profile.provider_kind = MembershipProviderKind::DeterministicSimulation;
    simulation.profile.profile_ref = test_ref("simulation-profile");
    simulation.profile.authority_strength = MembershipAuthorityStrength::ObservationOnly;
    simulation.view.source_profile_ref = simulation.profile.profile_ref.clone();
    simulation.view.source_evidence_ref = test_ref("simulation-source-evidence");
    simulation.view.authority_ref = test_ref("simulation-authority");
    let observations = BTreeMap::new();
    let reservations = Vec::new();
    validate_provider_parity(&live, &simulation, &observations, &observations, &reservations, &reservations)
        .expect("provider-specific authority can preserve one semantic contract");
    assert_ne!(live.profile.profile_ref, simulation.profile.profile_ref);

    simulation.view.epoch = SUCCESSOR_EPOCH;
    let issues =
        validate_provider_parity(&live, &simulation, &observations, &observations, &reservations, &reservations)
            .expect_err("view drift detected");
    assert!(issues.contains(&MembershipIssue::ProviderParityMismatch("membership-view")));
}
