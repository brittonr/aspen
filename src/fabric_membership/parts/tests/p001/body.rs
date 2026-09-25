
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

// r[verify molten.modularity.fabric_boundary.shell.denial]
// r[verify molten.modularity.fabric_boundary.validation]
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

    assert_failed_shell_steps_are_uncertain(&assignment, &activate, &authority);

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

/// A failed role effect, a failed commit after the effect, and malformed effect evidence are each
/// uncertain.
fn assert_failed_shell_steps_are_uncertain(
    assignment: &RoleAssignment,
    activate: &AssignmentCommand,
    authority: &AssignmentAuthoritySnapshot,
) {
    let mut uncertain_lifecycle = RecordingLifecycle {
        fail_activate: true,
        ..RecordingLifecycle::default()
    };
    let mut uncertain_persistence = InMemoryAssignmentPersistence::default();
    let uncertain = execute_assignment_command(
        &mut uncertain_persistence,
        &mut uncertain_lifecycle,
        assignment,
        activate,
        authority,
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
        execute_assignment_command(&mut commit_failure, &mut successful_effect, assignment, activate, authority)
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
    let malformed = execute_assignment_command(&mut no_commit, &mut malformed_effect, assignment, activate, authority)
        .expect("valid transition with malformed effect evidence");
    let AssignmentExecutionOutcome::Uncertain(malformed) = malformed else {
        panic!("malformed effect evidence must be uncertain")
    };
    assert_eq!(malformed.phase, AssignmentExecutionPhase::RoleEffect);
    assert!(malformed.effect_may_have_happened);
    assert!(no_commit.commits.is_empty());
}
