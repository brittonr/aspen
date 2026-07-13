use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

use super::*;

const SYNTHETIC_REF_CHUNK_HEX_CHARS: usize = 16;
const SYNTHETIC_REF_CHUNK_REPETITIONS: usize = 4;
const EXPECTED_MEMBERSHIP_PORT_COUNT: usize = 4;
const NOW: u64 = 100;
const OBSERVED_AT: u64 = 90;
const VALID_UNTIL: u64 = 110;
const MAX_AGE: u64 = 20;
const VIEW_EPOCH: u64 = 7;
const SERVICE_GENERATION: u64 = 3;
const ASSIGNMENT_EPOCH: u64 = 11;
const FENCING_TOKEN: u64 = 41;
const CPU_CAPACITY: u64 = 2_000;
const CPU_REQUEST: u64 = 500;
const MEMORY_CAPACITY: u64 = 8_000;
const MEMORY_REQUEST: u64 = 1_000;
const STORAGE_CAPACITY: u64 = 16_000;
const STORAGE_REQUEST: u64 = 2_000;
const REPLICA_COUNT: u32 = 2;

fn test_ref(label: &str) -> String {
    let mut hasher = DefaultHasher::new();
    label.hash(&mut hasher);
    let digest = hasher.finish();
    let chunk = format!("{digest:0width$x}", width = SYNTHETIC_REF_CHUNK_HEX_CHARS);
    format!("blake3:{}", chunk.repeat(SYNTHETIC_REF_CHUNK_REPETITIONS))
}

fn source_profile(kind: MembershipProviderKind) -> MembershipSourceProfile {
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

fn descriptor(node_id: &str, zone: &str, descriptor_label: &str) -> NodeDescriptor {
    NodeDescriptor {
        schema: NODE_DESCRIPTOR_SCHEMA.to_string(),
        node_id: node_id.to_string(),
        descriptor_ref: test_ref(descriptor_label),
        compatibility_ref: test_ref("compatibility-v1"),
        labels: vec![NodeLabel {
            key: "zone".to_string(),
            value: zone.to_string(),
            authority: LabelAuthority::Authoritative,
            evidence_ref: test_ref(&format!("{descriptor_label}-zone")),
        }],
        runtime_features: vec!["system-extension-v1".to_string()],
        capacity: ResourceAmount {
            cpu_millis: CPU_CAPACITY,
            memory_bytes: MEMORY_CAPACITY,
            storage_bytes: STORAGE_CAPACITY,
        },
    }
}

fn provider_snapshot(kind: MembershipProviderKind) -> MembershipProviderSnapshot {
    let profile = source_profile(kind);
    let descriptors = vec![
        descriptor("node-a", "zone-a", "descriptor-node-a"),
        descriptor("node-b", "zone-b", "descriptor-node-b"),
    ];
    let members = descriptors
        .iter()
        .map(|descriptor| MembershipMember {
            node_id: descriptor.node_id.clone(),
            descriptor_ref: descriptor.descriptor_ref.clone(),
            eligibility_ref: test_ref(&format!("eligibility-{}", descriptor.node_id)),
        })
        .collect();
    MembershipProviderSnapshot {
        profile: profile.clone(),
        view: MembershipView {
            schema: MEMBERSHIP_VIEW_SCHEMA.to_string(),
            view_id: "view-a".to_string(),
            epoch: VIEW_EPOCH,
            source_profile_ref: profile.profile_ref,
            source_evidence_ref: test_ref("membership-source-evidence"),
            authority_ref: test_ref("membership-authority"),
            eligibility_policy_ref: test_ref("membership-eligibility-policy"),
            observed_at_ticks: OBSERVED_AT,
            valid_until_ticks: VALID_UNTIL,
            members,
        },
        descriptors,
        detector_profiles: Vec::new(),
        failure_observations: Vec::new(),
        reservations: Vec::new(),
        observed_now_ticks: NOW,
        required_compatibility_ref: test_ref("compatibility-v1"),
    }
}

fn placement_request() -> PlacementRequest {
    PlacementRequest {
        requirements: RoleRequirements {
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
            required_labels: Vec::new(),
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
        detector_profiles: Vec::new(),
        tie_break_order: vec!["node-a".to_string(), "node-b".to_string()],
        conflicting_view_refs: Vec::new(),
        now_ticks: NOW,
    }
}

fn proposal() -> AssignmentProposal {
    AssignmentProposal {
        assignment_id: "assignment-a".to_string(),
        extension_id: "extension-a".to_string(),
        service_id: "service-a".to_string(),
        role_id: "replica-0".to_string(),
        role_kind: "replica".to_string(),
        node_id: "node-a".to_string(),
        service_generation: SERVICE_GENERATION,
        assignment_epoch: ASSIGNMENT_EPOCH,
        fencing_token: FENCING_TOKEN,
        fencing_profile_ref: test_ref("fencing-profile"),
        resource_reservation_ref: test_ref("resource-reservation"),
        placement_plan_ref: test_ref("placement-plan"),
        authority_ref: test_ref("assignment-authority"),
        durable_state_ref: Some(test_ref("assignment-durable-state")),
        predecessor_assignment_ref: None,
        predecessor_epoch: None,
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

fn acknowledged_assignment() -> RoleAssignment {
    let mut assignment = propose_assignment(&proposal()).expect("proposal");
    for (kind, transition_label) in [
        (AssignmentCommandKind::Reserve, "reserve"),
        (AssignmentCommandKind::Assign, "assign"),
        (AssignmentCommandKind::Acknowledge, "acknowledge"),
    ] {
        assignment = apply_assignment_command(&assignment, &command(&assignment, kind, transition_label))
            .expect("transition")
            .next;
    }
    assignment
}

fn authority_snapshot(assignment: &RoleAssignment) -> AssignmentAuthoritySnapshot {
    AssignmentAuthoritySnapshot {
        fencing_profile: FencingProfile {
            schema: FENCING_PROFILE_SCHEMA.to_string(),
            profile_id: "process-local-fence-v1".to_string(),
            profile_ref: assignment.fencing_profile_ref.clone(),
            authority_ref: assignment.authority_ref.clone(),
            effect_port_ref: test_ref("assignment-effect-port"),
            enforcement: FencingEnforcement::ProcessLocal,
        },
        enforced_assignment_epoch: assignment.assignment_epoch,
        enforced_fencing_token: assignment.fencing_token,
        required_enforcement: FencingEnforcement::ProcessLocal,
    }
}

#[derive(Debug, Default)]
struct RecordingLifecycle {
    calls: Vec<String>,
    fail_activate: bool,
    malformed_activate_ref: bool,
}

impl RecordingLifecycle {
    fn effect(&mut self, kind: &str, assignment: &RoleAssignment) -> std::result::Result<String, RoleEffectFailure> {
        self.calls.push(format!("{kind}:{}", assignment.assignment_id));
        Ok(test_ref(&format!("role-effect-{}", self.calls.len())))
    }
}

impl ExtensionRoleLifecyclePort for RecordingLifecycle {
    fn activate(&mut self, assignment: &RoleAssignment) -> std::result::Result<String, RoleEffectFailure> {
        if self.fail_activate {
            return Err(RoleEffectFailure {
                message: "injected activation uncertainty".to_string(),
                effect_may_have_happened: true,
            });
        }
        if self.malformed_activate_ref {
            self.calls.push(format!("activate:{}", assignment.assignment_id));
            return Ok("malformed-effect-ref".to_string());
        }
        self.effect("activate", assignment)
    }

    fn begin_drain(&mut self, assignment: &RoleAssignment) -> std::result::Result<String, RoleEffectFailure> {
        self.effect("drain", assignment)
    }

    fn begin_replacement(&mut self, assignment: &RoleAssignment) -> std::result::Result<String, RoleEffectFailure> {
        self.effect("replace", assignment)
    }

    fn release(&mut self, assignment: &RoleAssignment) -> std::result::Result<String, RoleEffectFailure> {
        self.effect("release", assignment)
    }

    fn fail(&mut self, assignment: &RoleAssignment) -> std::result::Result<String, RoleEffectFailure> {
        self.effect("fail", assignment)
    }

    fn quarantine(&mut self, assignment: &RoleAssignment) -> std::result::Result<String, RoleEffectFailure> {
        self.effect("quarantine", assignment)
    }
}

// r[verify molten.fabric_membership.live_sim_parity]
// r[verify molten.fabric_membership.evidence]
// r[verify molten.fabric_membership.final_validation]
#[test]
fn static_and_simulated_providers_share_canonical_contracts() {
    let static_snapshot = provider_snapshot(MembershipProviderKind::Static);
    let mut static_provider = StaticMembershipProvider::new(static_snapshot).expect("static provider");
    let static_admitted = observe_provider(&mut static_provider).expect("observe static");
    assert_eq!(static_admitted.membership.admitted.view.epoch, VIEW_EPOCH);
    assert!(static_admitted.failures.observations.is_empty());

    let simulation_snapshot = provider_snapshot(MembershipProviderKind::DeterministicSimulation);
    let mut simulation_provider =
        DeterministicSimulationMembershipProvider::new(vec![simulation_snapshot]).expect("simulation provider");
    let simulation_admitted = observe_provider(&mut simulation_provider).expect("observe simulation");
    assert_eq!(
        simulation_admitted.membership.admitted.view.members,
        static_admitted.membership.admitted.view.members
    );

    let exhausted = observe_provider(&mut simulation_provider).expect_err("bounded deterministic stream");
    assert!(exhausted.to_string().contains("snapshot stream is exhausted"));

    let wrong_kind = StaticMembershipProvider::new(provider_snapshot(MembershipProviderKind::PolicyManaged))
        .expect_err("adapter/profile mismatch denied");
    assert!(wrong_kind.contains("does not match adapter"));

    let first = provider_snapshot(MembershipProviderKind::DeterministicSimulation);
    let mut drifted = first.clone();
    drifted.view.epoch = VIEW_EPOCH + 1;
    drifted.profile.profile_ref = test_ref("drifted-simulation-profile");
    drifted.view.source_profile_ref = drifted.profile.profile_ref.clone();
    let drift = DeterministicSimulationMembershipProvider::new(vec![first, drifted])
        .expect_err("simulation source profile drift denied");
    assert!(drift.contains("cannot drift source profile"));
}

// r[verify molten.fabric_membership.placement]
// r[verify molten.fabric_membership.authority_separation]
#[test]
fn canonical_plan_and_readback_retain_bounded_authority_evidence() {
    let mut provider =
        StaticMembershipProvider::new(provider_snapshot(MembershipProviderKind::Static)).expect("provider");
    let admitted = observe_provider(&mut provider).expect("observe");
    let placement =
        canonical_placement_outcome(&admitted.membership, &placement_request()).expect("placement evidence");
    let PlacementOutcome::Planned(plan) = placement.outcome else {
        panic!("expected plan")
    };
    assert!(plan.advisory_only);
    assert!(!placement.outcome_ref.is_empty());

    let readback = membership_status_readback(&admitted.membership, &[]).expect("readback");
    assert_eq!(readback.member_ids, vec!["node-a".to_string(), "node-b".to_string()]);
    assert_eq!(readback.active_assignments, 0);
    assert_eq!(readback.non_claims, REQUIRED_MEMBERSHIP_NON_CLAIMS);

    let profile = canonical_membership_profile(&admitted.membership.admitted.profile).expect("profile");
    let ports = fabric_membership_port_descriptors(&profile);
    assert_eq!(ports.len(), EXPECTED_MEMBERSHIP_PORT_COUNT);
    assert!(ports.iter().any(|port| port.port_id == FABRIC_ASSIGNMENT_PORT_ID));

    let context =
        ExtensionMembershipPlacementContext::from_test_snapshot("service-a", SERVICE_GENERATION, &profile, vec![
            FABRIC_PLACEMENT_PORT_ID.to_string(),
            FABRIC_ASSIGNMENT_PORT_ID.to_string(),
        ]);
    context
        .admit_plan(&profile, &admitted.membership, "service-a", SERVICE_GENERATION)
        .expect("bound placement scope");
    let assignment = propose_assignment(&proposal()).expect("assignment");
    context.admit_assignment(&profile, &assignment).expect("bound assignment scope");
    let stale = context
        .admit_plan(&profile, &admitted.membership, "service-a", SERVICE_GENERATION + 1)
        .expect_err("stale extension generation denied");
    assert!(stale.to_string().contains("stale service generation"));

    let oversized_assignment_count = MAX_MEMBERSHIP_ITEMS + 1;
    let oversized_assignments = vec![(test_ref("oversized-assignment"), assignment); oversized_assignment_count];
    let oversized = membership_status_readback(&admitted.membership, &oversized_assignments)
        .expect_err("oversized operator readback denied");
    assert!(oversized.to_string().contains("assignment limit exceeded"));
}

// r[verify molten.fabric_membership.recruitment]
// r[verify molten.fabric_membership.drain_replace]
#[test]
fn assignment_shell_records_intent_before_effect_and_surfaces_uncertainty() {
    let assignment = acknowledged_assignment();
    let activate = command(&assignment, AssignmentCommandKind::Activate, "activate");
    let authority = authority_snapshot(&assignment);
    let mut persistence = InMemoryAssignmentPersistence::default();
    let mut lifecycle = RecordingLifecycle::default();
    let committed = execute_assignment_command(&mut persistence, &mut lifecycle, &assignment, &activate, &authority)
        .expect("valid transition");
    let AssignmentExecutionOutcome::Committed(receipt) = committed else {
        panic!("activation should commit")
    };
    assert_eq!(receipt.transition.next.state, AssignmentState::Active);
    assert_eq!(lifecycle.calls, vec!["activate:assignment-a".to_string()]);
    assert_eq!(persistence.intents.len(), 1);
    assert_eq!(persistence.commits.len(), 1);

    let mut uncertain_lifecycle = RecordingLifecycle {
        fail_activate: true,
        ..RecordingLifecycle::default()
    };
    let mut uncertain_persistence = InMemoryAssignmentPersistence::default();
    let uncertain = execute_assignment_command(
        &mut uncertain_persistence,
        &mut uncertain_lifecycle,
        &assignment,
        &activate,
        &authority,
    )
    .expect("valid transition with uncertain shell result");
    let AssignmentExecutionOutcome::Uncertain(uncertain) = uncertain else {
        panic!("failed activation must be uncertain")
    };
    assert_eq!(uncertain.phase, AssignmentExecutionPhase::RoleEffect);
    assert!(uncertain.effect_may_have_happened);
    assert!(uncertain.intent_ref.is_some());
    assert!(uncertain_persistence.commits.is_empty());

    let mut commit_failure = InMemoryAssignmentPersistence {
        fail_commit: true,
        ..InMemoryAssignmentPersistence::default()
    };
    let mut successful_effect = RecordingLifecycle::default();
    let uncertain_commit =
        execute_assignment_command(&mut commit_failure, &mut successful_effect, &assignment, &activate, &authority)
            .expect("valid transition with commit uncertainty");
    let AssignmentExecutionOutcome::Uncertain(uncertain_commit) = uncertain_commit else {
        panic!("commit failure after effect must be uncertain")
    };
    assert_eq!(uncertain_commit.phase, AssignmentExecutionPhase::CommitPersistence);
    assert!(uncertain_commit.role_effect_ref.is_some());
    assert!(uncertain_commit.effect_may_have_happened);

    let mut malformed_effect = RecordingLifecycle {
        malformed_activate_ref: true,
        ..RecordingLifecycle::default()
    };
    let mut no_commit = InMemoryAssignmentPersistence::default();
    let malformed =
        execute_assignment_command(&mut no_commit, &mut malformed_effect, &assignment, &activate, &authority)
            .expect("valid transition with malformed effect evidence");
    let AssignmentExecutionOutcome::Uncertain(malformed) = malformed else {
        panic!("malformed effect evidence must be uncertain")
    };
    assert_eq!(malformed.phase, AssignmentExecutionPhase::RoleEffect);
    assert!(malformed.effect_may_have_happened);
    assert!(no_commit.commits.is_empty());

    let mut stale_authority = authority;
    stale_authority.enforced_assignment_epoch = ASSIGNMENT_EPOCH + 1;
    let mut untouched_persistence = InMemoryAssignmentPersistence::default();
    let mut untouched_lifecycle = RecordingLifecycle::default();
    let stale = execute_assignment_command(
        &mut untouched_persistence,
        &mut untouched_lifecycle,
        &assignment,
        &activate,
        &stale_authority,
    )
    .expect_err("stale assignment denied before shell effects");
    assert!(stale.iter().any(|issue| matches!(issue, AssignmentIssue::StaleAssignmentEpoch { .. })));
    assert!(untouched_persistence.intents.is_empty());
    assert!(untouched_lifecycle.calls.is_empty());
}
