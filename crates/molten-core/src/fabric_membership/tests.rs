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

// r[verify molten.modularity.fabric_boundary.core.plan]
// r[verify molten.modularity.fabric_boundary.validation]
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

fn admitted_with_nodes_zones(nodes: usize, zones: usize) -> AdmittedMembershipView {
    let profile = profile(MembershipProviderKind::Static);
    let descriptors: Vec<NodeDescriptor> = (0..nodes)
        .map(|i| {
            descriptor(&format!("node-{:04}", i), &format!("zone-{}", i % zones), &format!("descriptor-node-{:04}", i))
        })
        .collect();
    let view = view(&profile, &descriptors);
    validate_membership_view(&profile, &view, &descriptors, NOW, &test_ref("compatibility-v1"))
        .expect("admitted membership view")
}

fn placement_request_with(admitted: &AdmittedMembershipView, replicas: u32) -> PlacementRequest {
    PlacementRequest {
        requirements: RoleRequirements {
            extension_id: "extension-a".to_string(),
            service_id: "service-a".to_string(),
            role_kind: "replica".to_string(),
            replica_count: replicas,
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
            preferred_labels: Vec::new(),
            anti_affinity_label_keys: vec!["zone".to_string()],
            distinct_nodes: true,
            avoid_suspected: true,
            allow_degraded: false,
            policy_ref: test_ref("placement-policy"),
        },
        current_assignments: Vec::new(),
        current_reservations: Vec::new(),
        failure_observations: Vec::new(),
        detector_profiles: vec![detector_profile()],
        tie_break_order: admitted.view.members.iter().map(|member| member.node_id.clone()).collect(),
        conflicting_view_refs: Vec::new(),
        now_ticks: NOW,
    }
}

// r[verify molten.fabric_membership.placement]
#[test]
fn placement_modest_unsatisfiable_views_return_unsatisfied() {
    for (nodes, zones, replicas) in [(30usize, 3usize, 4u32), (16usize, 4usize, 5u32)] {
        let admitted = admitted_with_nodes_zones(nodes, zones);
        let request = placement_request_with(&admitted, replicas);
        let outcome = plan_placement(&admitted, &request).expect("unsatisfied outcome");
        let PlacementOutcome::Unsatisfied(unsatisfied) = outcome else {
            panic!("expected unsatisfied outcome")
        };
        assert!(
            unsatisfied
                .constraints
                .iter()
                .any(|constraint| constraint.kind == UnsatisfiedConstraintKind::AntiAffinity),
            "expected anti-affinity constraint"
        );
    }
}

// r[verify molten.fabric_membership.placement]
#[test]
fn placement_joint_anti_affinity_unsatisfiable_reaches_search_limit() {
    // Two anti-affinity keys whose individual distinct values are sufficient
    // but whose joint feasibility fails: zones {0,1,2} only racks {0,1},
    // zones {3,4} racks {2,3,4}. 5 replicas impossible.
    let nodes_per_pair = 20usize;
    let mut descriptors = Vec::new();
    for zone in 0..3usize {
        for rack in 0..2usize {
            for _ in 0..nodes_per_pair {
                let idx = descriptors.len();
                let mut d =
                    descriptor(&format!("node-{:05}", idx), &format!("zone-{}", zone), &format!("descriptor-{}", idx));
                // Labels stay strictly ordered by key: rack < region < zone.
                d.labels.insert(
                    0,
                    label("rack", &format!("rack-{}", rack), LabelAuthority::Authoritative, &format!("rack-{}", idx)),
                );
                descriptors.push(d);
            }
        }
    }
    for zone in 3..5usize {
        for rack in 2..5usize {
            for _ in 0..nodes_per_pair {
                let idx = descriptors.len();
                let mut d =
                    descriptor(&format!("node-{:05}", idx), &format!("zone-{}", zone), &format!("descriptor-{}", idx));
                d.labels.insert(
                    0,
                    label("rack", &format!("rack-{}", rack), LabelAuthority::Authoritative, &format!("rack-{}", idx)),
                );
                descriptors.push(d);
            }
        }
    }
    // sort descriptors by node_id for view ordering
    descriptors.sort_by(|a, b| a.node_id.cmp(&b.node_id));
    let profile = profile(MembershipProviderKind::Static);
    let view = view(&profile, &descriptors);
    let admitted = validate_membership_view(&profile, &view, &descriptors, NOW, &test_ref("compatibility-v1"))
        .expect("admitted membership view");

    let mut request = PlacementRequest {
        requirements: RoleRequirements {
            extension_id: "extension-a".to_string(),
            service_id: "service-a".to_string(),
            role_kind: "replica".to_string(),
            replica_count: 5,
            per_replica: ResourceAmount {
                cpu_millis: CPU_REQUEST,
                memory_bytes: MEMORY_REQUEST,
                storage_bytes: STORAGE_REQUEST,
            },
            required_features: vec!["system-extension-v1".to_string()],
            required_labels: Vec::new(),
            preferred_labels: Vec::new(),
            anti_affinity_label_keys: vec!["rack".to_string(), "zone".to_string()],
            distinct_nodes: true,
            avoid_suspected: true,
            allow_degraded: false,
            policy_ref: test_ref("placement-policy"),
        },
        current_assignments: Vec::new(),
        current_reservations: Vec::new(),
        failure_observations: Vec::new(),
        detector_profiles: Vec::new(),
        tie_break_order: admitted.view.members.iter().map(|m| m.node_id.clone()).collect(),
        conflicting_view_refs: Vec::new(),
        now_ticks: NOW,
    };

    let result = plan_placement(&admitted, &request);
    assert!(result.is_err(), "expected search limit exceeded");
    let issues = result.unwrap_err();
    assert!(issues.contains(&MembershipIssue::SearchLimitExceeded));

    // Same view with 4 replicas returns a plan.
    request.requirements.replica_count = 4;
    let outcome = plan_placement(&admitted, &request).expect("plan");
    assert!(matches!(outcome, PlacementOutcome::Planned(_)));
}

// r[verify molten.fabric_membership.placement]
#[test]
fn placement_precheck_counts_held_anti_values() {
    let admitted = admitted_with_nodes_zones(4, 2);
    // Two distinct zones among candidates; one held assignment occupies zone-0.
    let mut request = placement_request_with(&admitted, 2);
    // hold node-0000 (zone-0)
    request.current_assignments = vec![CurrentRoleAssignment {
        assignment_ref: test_ref("current-assignment-node-0000"),
        node_id: "node-0000".to_string(),
        service_id: "service-a".to_string(),
        role_kind: "replica".to_string(),
        assignment_epoch: ASSIGNMENT_EPOCH,
        active: true,
    }];
    let outcome = plan_placement(&admitted, &request).expect("unsatisfied");
    let PlacementOutcome::Unsatisfied(u) = outcome else {
        panic!("expected unsatisfied")
    };
    assert!(u.constraints.iter().any(|c| c.kind == UnsatisfiedConstraintKind::AntiAffinity));
}

/// Exhaustively compares the candidate-set search with the previous ordering search on small views.
// r[verify molten.fabric_membership.placement]
#[test]
fn placement_set_search_matches_the_ordering_search_on_small_views() {
    let profile = profile(MembershipProviderKind::Static);
    let mut compared = 0_usize;
    for nodes in 1..=5_usize {
        for zones in 1..=3_usize {
            let descriptors: Vec<NodeDescriptor> = (0..nodes)
                .map(|i| {
                    descriptor(&format!("node-{i:04}"), &format!("zone-{}", i % zones), &format!("descriptor-{i}"))
                })
                .collect();
            let view = view(&profile, &descriptors);
            let admitted = validate_membership_view(&profile, &view, &descriptors, NOW, &test_ref("compatibility-v1"))
                .expect("admitted membership view");
            for replicas in 1..=4_u32 {
                for distinct_nodes in [true, false] {
                    for anti_affinity in [Vec::new(), vec!["zone".to_string()]] {
                        for assigned in 0..=nodes.min(2) {
                            let mut request = placement_request_with(&admitted, replicas);
                            request.requirements.distinct_nodes = distinct_nodes;
                            request.requirements.anti_affinity_label_keys = anti_affinity.clone();
                            request.current_assignments = (0..assigned)
                                .map(|i| CurrentRoleAssignment {
                                    assignment_ref: test_ref(&format!("current-assignment-{i}")),
                                    node_id: format!("node-{i:04}"),
                                    service_id: "service-a".to_string(),
                                    role_kind: "replica".to_string(),
                                    assignment_epoch: ASSIGNMENT_EPOCH,
                                    active: true,
                                })
                                .collect();
                            let ordering = plan_placement_by_orderings(&admitted, &request);
                            if ordering
                                .as_ref()
                                .is_err_and(|issues| issues.contains(&MembershipIssue::SearchLimitExceeded))
                            {
                                continue;
                            }
                            assert_eq!(
                                plan_placement(&admitted, &request),
                                ordering,
                                "nodes={nodes} zones={zones} replicas={replicas} distinct={distinct_nodes} \
                                 anti={anti_affinity:?} assigned={assigned}"
                            );
                            compared += 1;
                        }
                    }
                }
            }
        }
    }
    assert!(compared > 500, "compared {compared} cases");
}

fn four_zone_admitted() -> AdmittedMembershipView {
    let profile = profile(MembershipProviderKind::Static);
    let descriptors = vec![
        descriptor("node-a", "zone-a", "descriptor-node-a"),
        descriptor("node-b", "zone-b", "descriptor-node-b"),
        descriptor("node-c", "zone-c", "descriptor-node-c"),
        descriptor("node-d", "zone-d", "descriptor-node-d"),
    ];
    let view = view(&profile, &descriptors);
    validate_membership_view(&profile, &view, &descriptors, NOW, &test_ref("compatibility-v1"))
        .expect("admitted membership view")
}

const HEALTHY_PREFERENCE: u32 = 10;
const DEGRADED_EXTRA_PREFERENCE: u32 = 90;

/// node-a scores 100 but is `Unavailable`; every other node is healthy and scores 10.
fn degraded_preference_request(admitted: &AdmittedMembershipView, replicas: u32) -> PlacementRequest {
    let mut request = placement_request_with(admitted, replicas);
    request.requirements.allow_degraded = true;
    request.requirements.preferred_labels = vec![
        PreferredLabel {
            key: "region".to_string(),
            value: "east".to_string(),
            minimum_authority: LabelAuthority::OperatorDeclared,
            weight: HEALTHY_PREFERENCE,
        },
        PreferredLabel {
            key: "zone".to_string(),
            value: "zone-a".to_string(),
            minimum_authority: LabelAuthority::Authoritative,
            weight: DEGRADED_EXTRA_PREFERENCE,
        },
    ];
    request.failure_observations = vec![observation(
        "node-a",
        FailureObservationClass::Unavailable,
        "unavailable-node-a",
    )];
    request
}

fn planned(outcome: Result<PlacementOutcome, Vec<MembershipIssue>>) -> PlacementPlan {
    match outcome.expect("placement outcome") {
        PlacementOutcome::Planned(plan) => plan,
        PlacementOutcome::Unsatisfied(unsatisfied) => panic!("expected a plan, got {unsatisfied:?}"),
    }
}

// r[verify molten.fabric_membership.healthy_first_placement_and_time_basis_reduction]
#[test]
fn higher_preference_unavailable_node_does_not_displace_healthy_nodes() {
    let admitted = four_zone_admitted();
    for replicas in [1_u32, 2, 3] {
        let request = degraded_preference_request(&admitted, replicas);
        for outcome in [
            plan_placement(&admitted, &request),
            plan_placement_by_orderings(&admitted, &request),
        ] {
            let plan = planned(outcome);
            assert!(!plan.degraded, "replicas={replicas}: healthy nodes suffice");
            assert!(plan.roles.iter().all(|role| role.node_id != "node-a"), "replicas={replicas}: {plan:?}");
            assert!(plan.roles.iter().all(|role| role.preference_score == u64::from(HEALTHY_PREFERENCE)));
        }
    }
}

// r[verify molten.fabric_membership.healthy_first_placement_and_time_basis_reduction]
#[test]
fn degraded_nodes_are_used_only_when_healthy_nodes_are_insufficient() {
    let admitted = four_zone_admitted();
    let request = degraded_preference_request(&admitted, 4);
    let plan = planned(plan_placement(&admitted, &request));
    assert!(plan.degraded, "a fourth replica needs the unavailable node");
    let degraded_roles = plan.roles.iter().filter(|role| role.node_id == "node-a").count();
    assert_eq!(degraded_roles, 1);
    assert_eq!(plan_placement_by_orderings(&admitted, &request).map(|outcome| planned(Ok(outcome))), Ok(plan));

    let mut denied = request;
    denied.requirements.allow_degraded = false;
    let PlacementOutcome::Unsatisfied(unsatisfied) = plan_placement(&admitted, &denied).expect("outcome") else {
        panic!("degraded fallback requires allow_degraded")
    };
    assert!(
        unsatisfied
            .constraints
            .iter()
            .any(|constraint| constraint.kind == UnsatisfiedConstraintKind::FailurePolicy)
    );
}

/// The first plan found uses the fewest degraded replicas even when a lexicographically earlier
/// candidate set would mix a healthy node with a degraded one.
// r[verify molten.fabric_membership.healthy_first_placement_and_time_basis_reduction]
#[test]
fn degraded_fallback_minimizes_degraded_replicas() {
    let profile = profile(MembershipProviderKind::Static);
    // node-a conflicts with node-b on `rack` and with node-c on `zone`; node-b and node-c are
    // compatible; node-d is degraded and compatible with node-a.
    let rack = |mut descriptor: NodeDescriptor, value: &str| {
        let evidence = format!("rack-{}", descriptor.node_id);
        descriptor.labels.insert(0, label("rack", value, LabelAuthority::Authoritative, &evidence));
        descriptor
    };
    let descriptors = vec![
        rack(descriptor("node-a", "zone-a", "descriptor-node-a"), "rack-1"),
        rack(descriptor("node-b", "zone-b", "descriptor-node-b"), "rack-1"),
        rack(descriptor("node-c", "zone-a", "descriptor-node-c"), "rack-2"),
        rack(descriptor("node-d", "zone-d", "descriptor-node-d"), "rack-3"),
    ];
    let view = view(&profile, &descriptors);
    let admitted = validate_membership_view(&profile, &view, &descriptors, NOW, &test_ref("compatibility-v1"))
        .expect("admitted membership view");
    let mut request = placement_request_with(&admitted, 2);
    request.requirements.allow_degraded = true;
    request.requirements.anti_affinity_label_keys = vec!["rack".to_string(), "zone".to_string()];
    request.failure_observations = vec![observation(
        "node-d",
        FailureObservationClass::Suspected,
        "suspected-node-d",
    )];
    // node-a ranks first among healthy nodes; {node-a, node-d} would be found before {node-b, node-c}.
    let plan = planned(plan_placement(&admitted, &request));
    assert!(!plan.degraded, "{plan:?}");
    let nodes = plan.roles.iter().map(|role| role.node_id.as_str()).collect::<Vec<_>>();
    assert!(!nodes.contains(&"node-d"), "{nodes:?}");
    assert_eq!(planned(plan_placement_by_orderings(&admitted, &request)), plan);
}

fn detector(profile_label: &str, basis_label: &str) -> FailureDetectorProfile {
    FailureDetectorProfile {
        profile_id: format!("detector-{profile_label}"),
        profile_ref: test_ref(profile_label),
        time_basis_ref: test_ref(basis_label),
        ..detector_profile()
    }
}

fn timed_observation(
    node_id: &str,
    class: FailureObservationClass,
    detector_label: &str,
    observed_at_ticks: u64,
) -> FailureObservation {
    FailureObservation {
        detector_profile_ref: test_ref(detector_label),
        observed_at_ticks,
        ..observation(node_id, class, &format!("{detector_label}-{node_id}-{observed_at_ticks}"))
    }
}

// r[verify molten.fabric_membership.healthy_first_placement_and_time_basis_reduction]
#[test]
fn failure_reduction_is_independent_of_input_order() {
    let admitted = admitted();
    let profiles = vec![
        detector("detector-one", "basis-shared"),
        detector("detector-two", "basis-shared"),
    ];
    let observations = vec![
        timed_observation("node-a", FailureObservationClass::Suspected, "detector-one", OBSERVED_AT),
        timed_observation("node-a", FailureObservationClass::Suspected, "detector-two", OBSERVED_AT),
        timed_observation("node-b", FailureObservationClass::Unavailable, "detector-one", OBSERVED_AT - 5),
        timed_observation("node-b", FailureObservationClass::Reachable, "detector-two", OBSERVED_AT),
    ];
    let forward = reduce_failure_observations(&admitted, &profiles, &observations, NOW).expect("forward");
    let mut reversed_input = observations.clone();
    reversed_input.reverse();
    let reversed = reduce_failure_observations(&admitted, &profiles, &reversed_input, NOW).expect("reversed");
    assert_eq!(forward, reversed);
    let expected_tie_winner = test_ref("detector-one").max(test_ref("detector-two"));
    assert_eq!(forward["node-a"].detector_profile_ref, expected_tie_winner);
    assert_eq!(forward["node-b"].class, FailureObservationClass::Reachable, "same basis: latest wins");
    assert_eq!(forward["node-b"].observed_at_ticks, OBSERVED_AT);
}

// r[verify molten.fabric_membership.healthy_first_placement_and_time_basis_reduction]
#[test]
fn failure_reduction_never_compares_ticks_across_time_bases() {
    let admitted = admitted();
    let profiles = vec![
        detector("detector-basis-a", "basis-a"),
        detector("detector-basis-b", "basis-b"),
    ];
    // Raw ticks would say node-a recovered (95 > 80), but the ticks belong to unrelated bases.
    let observations = vec![
        timed_observation("node-a", FailureObservationClass::Unavailable, "detector-basis-a", OBSERVED_AT - 10),
        timed_observation("node-a", FailureObservationClass::Reachable, "detector-basis-b", OBSERVED_AT + 5),
    ];
    for input in [
        observations.clone(),
        observations.iter().rev().cloned().collect::<Vec<_>>(),
    ] {
        let reduced = reduce_failure_observations(&admitted, &profiles, &input, NOW).expect("reduction");
        assert_eq!(reduced["node-a"].class, FailureObservationClass::Unavailable);
        assert_eq!(reduced["node-a"].detector_profile_ref, test_ref("detector-basis-a"));
    }

    // Within one shared basis the later Reachable observation does supersede the earlier one.
    let shared = vec![
        detector("detector-basis-a", "basis-a"),
        detector("detector-basis-b", "basis-a"),
    ];
    let reduced = reduce_failure_observations(&admitted, &shared, &observations, NOW).expect("shared basis");
    assert_eq!(reduced["node-a"].class, FailureObservationClass::Reachable);
}

fn observed_fence(epoch: u64, token: u64, enforcement: FencingEnforcement) -> FenceObservation {
    FenceObservation {
        fencing_profile: FencingProfile {
            schema: FENCING_PROFILE_SCHEMA.to_string(),
            profile_id: "observed-fence-v1".to_string(),
            profile_ref: test_ref("fencing-profile"),
            authority_ref: test_ref("assignment-authority"),
            effect_port_ref: test_ref("fenced-effect-port"),
            enforcement,
        },
        enforced_assignment_epoch: epoch,
        enforced_fencing_token: token,
    }
}

fn quarantine_clearance() -> QuarantineClearance {
    QuarantineClearance {
        quarantined_assignment_ref: test_ref("predecessor-assignment"),
        reconciliation_ref: test_ref("quarantine-reconciliation"),
    }
}

fn persisted_reconciliation(quarantined: &RoleAssignment) -> QuarantineReconciliation {
    QuarantineReconciliation {
        assignment_id: quarantined.assignment_id.clone(),
        quarantined_assignment_ref: test_ref("predecessor-assignment"),
        basis: QuarantineReconciliationBasis::CommittedEffect {
            intent_ref: test_ref("quarantine-intent"),
        },
        evidence_ref: test_ref("quarantine-effect"),
        reconcile_ref: test_ref("quarantine-reconciliation"),
    }
}

// r[verify molten.fabric_membership.recoverable_pending_and_quarantined_slots]
#[test]
fn quarantined_predecessor_admits_only_a_reconciled_and_observed_fenced_successor() {
    let active = active_assignment();
    let quarantined =
        apply_assignment_command(&active, &command(&active, AssignmentCommandKind::Quarantine, "quarantine"))
            .expect("quarantine")
            .next;
    assert_eq!(quarantined.state, AssignmentState::Quarantined);
    let predecessor_ref = test_ref("predecessor-assignment");
    // The successor runs on node-b while the quarantined owner may still run on node-a.
    let successor = propose_assignment(&proposal(true)).expect("successor");
    assert_ne!(successor.node_id, quarantined.node_id);
    let record = persisted_reconciliation(&quarantined);
    let fence = observed_fence(SUCCESSOR_EPOCH, SUCCESSOR_TOKEN, FencingEnforcement::QuorumOrdered);
    let validate = |predecessor: &RoleAssignment,
                    successor: &RoleAssignment,
                    clearance: &QuarantineClearance,
                    record: Option<&QuarantineReconciliation>,
                    fence: &FenceObservation| {
        validate_quarantined_successor(predecessor, &predecessor_ref, successor, clearance, record, fence)
    };

    // The released-predecessor rule alone never admits a quarantined slot.
    assert!(
        validate_assignment_successor(&quarantined, &predecessor_ref, &successor)
            .contains(&AssignmentIssue::AssignmentNotReleased(AssignmentState::Quarantined))
    );
    assert_eq!(validate(&quarantined, &successor, &quarantine_clearance(), Some(&record), &fence), Vec::new());

    // A clearance citing a reconciliation the owner never persisted is denied.
    let mut fabricated = quarantine_clearance();
    fabricated.reconciliation_ref = test_ref("fabricated-reconciliation");
    assert!(
        validate(&quarantined, &successor, &fabricated, Some(&record), &fence)
            .contains(&AssignmentIssue::QuarantineReconciliationMismatch)
    );
    assert!(
        validate(&quarantined, &successor, &quarantine_clearance(), None, &fence)
            .contains(&AssignmentIssue::QuarantineReconciliationMissing)
    );
    let mut other_assignment_record = record.clone();
    other_assignment_record.quarantined_assignment_ref = test_ref("other-quarantined-assignment");
    assert!(
        validate(&quarantined, &successor, &quarantine_clearance(), Some(&other_assignment_record), &fence)
            .contains(&AssignmentIssue::QuarantineReconciliationMismatch)
    );
    // A migrated legacy record must bind the quarantined assignment's own quarantine transition.
    let mut legacy = record.clone();
    legacy.basis = QuarantineReconciliationBasis::LegacyRecord {
        transition_ref: quarantined.transition_refs.last().expect("quarantine transition").clone(),
    };
    assert_eq!(validate(&quarantined, &successor, &quarantine_clearance(), Some(&legacy), &fence), Vec::new());
    legacy.basis = QuarantineReconciliationBasis::LegacyRecord {
        transition_ref: test_ref("other-transition"),
    };
    assert!(
        validate(&quarantined, &successor, &quarantine_clearance(), Some(&legacy), &fence)
            .contains(&AssignmentIssue::QuarantineReconciliationMismatch)
    );
    let mut wrong_predecessor = quarantine_clearance();
    wrong_predecessor.quarantined_assignment_ref = test_ref("other-assignment");
    assert!(
        validate(&quarantined, &successor, &wrong_predecessor, Some(&record), &fence)
            .contains(&AssignmentIssue::QuarantineClearanceMismatch)
    );

    // The observed fence, not a caller assertion, must already enforce the successor.
    let unadvanced = observed_fence(ASSIGNMENT_EPOCH, FENCING_TOKEN, FencingEnforcement::QuorumOrdered);
    assert!(validate(&quarantined, &successor, &quarantine_clearance(), Some(&record), &unadvanced).contains(
        &AssignmentIssue::StaleFencingToken {
            expected: FENCING_TOKEN,
            actual: SUCCESSOR_TOKEN,
        }
    ));

    let mut stale_successor = successor.clone();
    stale_successor.fencing_token = FENCING_TOKEN;
    assert!(
        validate(&quarantined, &stale_successor, &quarantine_clearance(), Some(&record), &fence)
            .iter()
            .any(|issue| matches!(issue, AssignmentIssue::StaleFencingToken { .. }))
    );
    assert!(
        validate(&active, &successor, &quarantine_clearance(), Some(&record), &fence)
            .contains(&AssignmentIssue::AssignmentNotQuarantined(AssignmentState::Active))
    );
}

// r[verify molten.fabric_membership.recoverable_pending_and_quarantined_slots]
#[test]
fn quarantine_succession_derives_fence_strength_from_node_identities() {
    let active = active_assignment();
    let quarantined =
        apply_assignment_command(&active, &command(&active, AssignmentCommandKind::Quarantine, "quarantine"))
            .expect("quarantine")
            .next;
    let predecessor_ref = test_ref("predecessor-assignment");
    let record = persisted_reconciliation(&quarantined);
    let cross_node = propose_assignment(&proposal(true)).expect("cross-node successor");
    let mut same_node = cross_node.clone();
    same_node.node_id = quarantined.node_id.clone();
    assert_eq!(
        minimum_quarantine_succession_enforcement(&quarantined, &cross_node),
        FencingEnforcement::QuorumOrdered
    );
    assert_eq!(
        minimum_quarantine_succession_enforcement(&quarantined, &same_node),
        FencingEnforcement::NodeLocalDurable
    );
    let validate = |successor: &RoleAssignment, enforcement: FencingEnforcement| {
        validate_quarantined_successor(
            &quarantined,
            &predecessor_ref,
            successor,
            &quarantine_clearance(),
            Some(&record),
            &observed_fence(SUCCESSOR_EPOCH, SUCCESSOR_TOKEN, enforcement),
        )
    };
    // A node-local fence cannot exclude an old owner on another node.
    assert!(validate(&cross_node, FencingEnforcement::NodeLocalDurable).contains(
        &AssignmentIssue::FencingStrengthInsufficient {
            available: FencingEnforcement::NodeLocalDurable,
            required: FencingEnforcement::QuorumOrdered,
        }
    ));
    assert!(validate(&cross_node, FencingEnforcement::ProcessLocal).contains(
        &AssignmentIssue::FencingStrengthInsufficient {
            available: FencingEnforcement::ProcessLocal,
            required: FencingEnforcement::QuorumOrdered,
        }
    ));
    assert_eq!(validate(&cross_node, FencingEnforcement::ExternallyEnforced), Vec::new());
    assert_eq!(validate(&same_node, FencingEnforcement::NodeLocalDurable), Vec::new());
    assert!(validate(&same_node, FencingEnforcement::ProcessLocal).contains(
        &AssignmentIssue::FencingStrengthInsufficient {
            available: FencingEnforcement::ProcessLocal,
            required: FencingEnforcement::NodeLocalDurable,
        }
    ));
}

/// Four anti-affine replicas over 27 healthy nodes in each of three zones plus one degraded node
/// in a fourth zone: the zero-degraded pass is infeasible and must not consume the step bound
/// before the degraded fallback runs.
// r[verify molten.fabric_membership.healthy_first_placement_and_time_basis_reduction]
#[test]
fn degraded_fallback_stays_reachable_when_healthy_zones_are_insufficient() {
    const HEALTHY_PER_ZONE: usize = 27;
    const HEALTHY_ZONES: usize = 3;
    let profile = profile(MembershipProviderKind::Static);
    let mut descriptors = (0..HEALTHY_PER_ZONE * HEALTHY_ZONES)
        .map(|i| {
            descriptor(&format!("node-{i:04}"), &format!("zone-{}", i % HEALTHY_ZONES), &format!("descriptor-{i}"))
        })
        .collect::<Vec<_>>();
    let degraded_index = descriptors.len();
    let degraded_node = format!("node-{degraded_index:04}");
    descriptors.push(descriptor(&degraded_node, "zone-3", &format!("descriptor-{degraded_index}")));
    let view = view(&profile, &descriptors);
    let admitted = validate_membership_view(&profile, &view, &descriptors, NOW, &test_ref("compatibility-v1"))
        .expect("admitted membership view");
    let mut request = placement_request_with(&admitted, 4);
    request.requirements.allow_degraded = true;
    request.failure_observations = vec![observation(
        &degraded_node,
        FailureObservationClass::Unavailable,
        "unavailable-fourth-zone",
    )];

    let plan = planned(plan_placement(&admitted, &request));
    assert!(plan.degraded);
    assert_eq!(plan.roles.len(), 4);
    assert_eq!(plan.roles.iter().filter(|role| role.node_id == degraded_node).count(), 1);
    let zones = plan
        .roles
        .iter()
        .map(|role| admitted.descriptors[&role.node_id].label("zone").expect("zone").value.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(zones.len(), 4, "every replica lands in a distinct zone");

    // Without the degraded fallback the request is unsatisfied, not a search-limit failure.
    request.requirements.allow_degraded = false;
    let PlacementOutcome::Unsatisfied(unsatisfied) = plan_placement(&admitted, &request).expect("bounded outcome")
    else {
        panic!("three healthy zones cannot host four anti-affine replicas")
    };
    assert!(
        unsatisfied
            .constraints
            .iter()
            .any(|constraint| constraint.kind == UnsatisfiedConstraintKind::AntiAffinity)
    );
}

// r[verify molten.fabric_membership.recoverable_pending_and_quarantined_slots]
#[test]
fn quarantine_may_record_an_uncertain_old_owner() {
    let active = active_assignment();
    let mut quarantine = command(&active, AssignmentCommandKind::Quarantine, "uncertain-quarantine");
    quarantine.uncertain_old_owner = true;
    let quarantined = apply_assignment_command(&active, &quarantine).expect("uncertain quarantine").next;
    assert_eq!(quarantined.state, AssignmentState::Quarantined);
    assert!(quarantined.uncertain_old_owner);
    let mut drain = command(&active, AssignmentCommandKind::BeginDrain, "uncertain-drain");
    drain.uncertain_old_owner = true;
    assert!(apply_assignment_command(&active, &drain).is_err(), "other commands still cannot claim uncertainty");
}
