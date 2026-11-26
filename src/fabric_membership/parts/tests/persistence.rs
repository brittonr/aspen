fn initialize_acknowledged(persistence: &mut dyn AssignmentPersistence) -> RoleAssignment {
    let mut current = persistence.initialize(&proposal()).expect("initialize Proposed").assignment;
    let mut lifecycle = RecordingLifecycle::default();
    for (kind, label) in [
        (AssignmentCommandKind::Reserve, "reserve"),
        (AssignmentCommandKind::Assign, "assign"),
        (AssignmentCommandKind::Acknowledge, "acknowledge"),
    ] {
        let outcome = execute_assignment_command(
            persistence, &mut lifecycle, &current, &command(&current, kind, label), &authority_snapshot(&current),
        ).expect("pure transition");
        let AssignmentExecutionOutcome::Committed(receipt) = outcome else { panic!("expected commit") };
        current = receipt.transition.next;
    }
    assert!(lifecycle.calls.is_empty(), "initialization never fabricates role-effect evidence");
    current
}

fn initialized_memory() -> crate::fabric_membership::persistence::InMemory {
    let mut persistence = crate::fabric_membership::persistence::InMemory::default();
    assert_eq!(initialize_acknowledged(&mut persistence), acknowledged_assignment());
    persistence.intents.clear();
    persistence.commits.clear();
    persistence
}

fn assignment_store_root(label: &str) -> (crate::test_support::Workspace, crate::local_store::DurableStoreRoot) {
    let workspace = crate::test_support::Workspace::new(label).expect("owned test workspace");
    let state = workspace.state().expect("state capability");
    let root = crate::node_state::DirectoryRoot::from_dir(state.dir().try_clone().expect("clone capability"));
    root.create_layout().expect("node layout");
    let store = crate::local_store::DurableStoreRoot::from_ledger(
        &root.ledger_store().expect("ledger capability"),
        &crate::local_store::Locator::parse("assignment-owner").expect("relative namespace"),
    ).expect("durable assignment capability");
    (workspace, store)
}

// r[verify molten.fabric_membership.fencing]
#[test]
fn durable_assignment_reopens_exact_committed_state_and_rejects_stale_nonoverwrite() {
    let (_workspace, root) = assignment_store_root("assignment-reopen-current");
    let mut persistence = crate::fabric_membership::persistence::Redb::open(&root).expect("open owner");
    assert!(persistence.current("assignment-a").expect("missing read").is_none());
    let acknowledged = initialize_acknowledged(&mut persistence);
    let mut lifecycle = RecordingLifecycle::default();
    let activate = command(&acknowledged, AssignmentCommandKind::Activate, "activate");
    let outcome = execute_assignment_command(
        &mut persistence, &mut lifecycle, &acknowledged, &activate, &authority_snapshot(&acknowledged),
    ).expect("activation");
    let AssignmentExecutionOutcome::Committed(receipt) = outcome else { panic!("activation must commit") };
    let observed = persistence.current("assignment-a").expect("current").expect("assignment");
    assert_eq!(observed.assignment, receipt.transition.next);
    assert!(observed.assignment_ref.starts_with("blake3:"));
    drop(persistence);

    let mut reopened = crate::fabric_membership::persistence::Redb::open(&root).expect("reopen owner");
    assert_eq!(reopened.current("assignment-a").expect("reopened read"), Some(observed.clone()));
    assert!(reopened.initialize(&proposal()).is_err(), "initialization cannot reset active state");
    let stale_transition = apply_assignment_command(&acknowledged, &activate).expect("pure stale transition");
    assert!(reopened.record_intent(&acknowledged, &stale_transition).is_err());
    for (epoch, token, generation) in [
        (ASSIGNMENT_EPOCH + 1, FENCING_TOKEN, SERVICE_GENERATION),
        (ASSIGNMENT_EPOCH, FENCING_TOKEN + 1, SERVICE_GENERATION),
        (ASSIGNMENT_EPOCH, FENCING_TOKEN, SERVICE_GENERATION + 1),
    ] {
        let mut stale = observed.assignment.clone();
        stale.assignment_epoch = epoch;
        stale.fencing_token = token;
        stale.service_generation = generation;
        let transition = apply_assignment_command(&stale, &command(&stale, AssignmentCommandKind::BeginDrain, "drain"))
            .expect("valid only against supplied stale state");
        assert!(reopened.record_intent(&stale, &transition).is_err());
    }
    assert_eq!(reopened.current("assignment-a").expect("not overwritten"), Some(observed));
}

// r[verify molten.fabric_membership.recruitment]
#[test]
fn durable_assignment_retains_uncertain_intent_and_never_replays_role_effect_on_reopen() {
    let (_workspace, root) = assignment_store_root("assignment-reopen-pending");
    let mut persistence = crate::fabric_membership::persistence::Redb::open(&root).expect("open owner");
    let acknowledged = initialize_acknowledged(&mut persistence);
    let activate = command(&acknowledged, AssignmentCommandKind::Activate, "activate-uncertain");
    let mut uncertain_effect = RecordingLifecycle { malformed_activate_ref: true, ..RecordingLifecycle::default() };
    let outcome = execute_assignment_command(
        &mut persistence, &mut uncertain_effect, &acknowledged, &activate, &authority_snapshot(&acknowledged),
    ).expect("activation uncertainty");
    let AssignmentExecutionOutcome::Uncertain(uncertain) = outcome else { panic!("malformed role receipt is uncertain") };
    assert_eq!(uncertain.phase, AssignmentExecutionPhase::RoleEffect);
    assert!(uncertain.effect_may_have_happened);
    assert_eq!(uncertain_effect.calls.len(), 1);
    assert!(persistence.current("assignment-a").is_err(), "pending is not committed currentness");
    drop(persistence);

    let mut reopened = crate::fabric_membership::persistence::Redb::open(&root).expect("reopen pending owner");
    assert!(reopened.current("assignment-a").is_err());
    assert!(reopened.initialize(&proposal()).is_err());
    let mut retry_effect = RecordingLifecycle::default();
    let outcome = execute_assignment_command(
        &mut reopened, &mut retry_effect, &acknowledged, &activate, &authority_snapshot(&acknowledged),
    ).expect("retry is explicitly uncertain");
    let AssignmentExecutionOutcome::Uncertain(retry) = outcome else { panic!("pending intent must deny replay") };
    assert_eq!(retry.phase, AssignmentExecutionPhase::IntentPersistence);
    assert!(retry_effect.calls.is_empty());
    assert!(reopened.current("assignment-a").is_err());
}

fn assert_strict_intent_protocol(persistence: &mut dyn AssignmentPersistence) {
    let acknowledged = initialize_acknowledged(persistence);
    let transition = apply_assignment_command(&acknowledged,
        &command(&acknowledged, AssignmentCommandKind::Activate, "activate-cas")).expect("activation");
    let mut forged = transition.clone();
    forged.next.node_id = "node-forged".to_string();
    assert!(persistence.record_intent(&acknowledged, &forged).is_err(), "must rederive pure transition");
    let intent = persistence.record_intent(&acknowledged, &transition).expect("intent");
    assert!(persistence.current("assignment-a").is_err());
    assert!(persistence.record_intent(&acknowledged, &transition).is_err(), "no duplicate intent replay");
    assert!(persistence.commit(&transition, &test_ref("wrong-intent"), Some(&test_ref("effect"))).is_err());
    assert!(persistence.commit(&forged, &intent, Some(&test_ref("effect"))).is_err());
    assert!(persistence.commit(&transition, &intent, None).is_err(), "activation needs actual effect evidence");
    assert!(persistence.commit(&transition, &intent, Some("malformed")).is_err());
    assert!(persistence.current("assignment-a").is_err(), "failed commit leaves pending intact");
    persistence.commit(&transition, &intent, Some(&test_ref("actual-effect"))).expect("exact commit");
    let current = persistence.current("assignment-a").expect("committed read").expect("assignment");
    assert_eq!(current.assignment, transition.next);
    assert!(persistence.commit(&transition, &intent, Some(&test_ref("actual-effect"))).is_err(), "stale commit cannot overwrite");
    assert_eq!(persistence.current("assignment-a").expect("unchanged"), Some(current));
}

// r[verify molten.fabric_membership.fencing]
#[test]
fn durable_and_memory_owners_share_exact_intent_and_commit_contract() {
    assert_strict_intent_protocol(&mut crate::fabric_membership::persistence::InMemory::default());
    let (_workspace, root) = assignment_store_root("assignment-exact-intent");
    assert_strict_intent_protocol(&mut crate::fabric_membership::persistence::Redb::open(&root).expect("durable owner"));
}

fn successor_proposal(predecessor: &CurrentAssignment) -> AssignmentProposal {
    let mut next = proposal();
    next.assignment_id = "assignment-b".to_string();
    next.node_id = "node-b".to_string();
    next.assignment_epoch = predecessor.assignment.assignment_epoch + 1;
    next.fencing_token = predecessor.assignment.fencing_token + 1;
    next.predecessor_epoch = Some(predecessor.assignment.assignment_epoch);
    next.predecessor_assignment_ref = Some(predecessor.assignment_ref.clone());
    next
}

fn assert_role_slot_fencing(persistence: &mut dyn AssignmentPersistence) -> CurrentAssignment {
    let acknowledged = initialize_acknowledged(persistence);
    let before_release = persistence.current("assignment-a").expect("current").expect("owner");
    assert!(persistence.initialize(&successor_proposal(&before_release)).is_err(), "nonreleased role cannot transfer");
    let release = apply_assignment_command(&acknowledged,
        &command(&acknowledged, AssignmentCommandKind::Release, "release-role-slot")).expect("release command");
    let intent = persistence.record_intent(&acknowledged, &release).expect("release intent");
    assert!(persistence.initialize(&successor_proposal(&before_release)).is_err(), "pending role cannot transfer");
    persistence.commit(&release, &intent, Some(&test_ref("actual-release-effect"))).expect("released role");
    let predecessor = persistence.current("assignment-a").expect("released current").expect("released owner");
    let successor = successor_proposal(&predecessor);
    for invalid in [
        AssignmentProposal { fencing_token: predecessor.assignment.fencing_token, ..successor.clone() },
        AssignmentProposal { assignment_epoch: predecessor.assignment.assignment_epoch, ..successor.clone() },
        AssignmentProposal { service_generation: predecessor.assignment.service_generation - 1, ..successor.clone() },
        AssignmentProposal { predecessor_assignment_ref: Some(test_ref("wrong-predecessor")), ..successor.clone() },
        AssignmentProposal { predecessor_epoch: Some(predecessor.assignment.assignment_epoch - 1), ..successor.clone() },
        AssignmentProposal { predecessor_assignment_ref: None, predecessor_epoch: None, ..successor.clone() },
    ] {
        assert!(persistence.initialize(&invalid).is_err());
        assert_eq!(persistence.current("assignment-a").expect("failed successor preserves owner"), Some(predecessor.clone()));
        assert!(persistence.current("assignment-b").expect("no partial successor").is_none());
    }
    let successor = persistence.initialize(&successor).expect("exact fenced successor");
    assert_eq!(successor.assignment.state, AssignmentState::Proposed);
    assert!(persistence.current("assignment-a").is_err(), "superseded ID cannot admit actor work");
    assert!(persistence.commit(&release, &intent, Some(&test_ref("actual-release-effect"))).is_err());
    assert!(persistence.initialize(&proposal()).is_err(), "historical ID cannot reset high-water mark");
    assert_eq!(persistence.current("assignment-b").expect("new owner"), Some(successor.clone()));
    successor
}

// r[verify molten.fabric_membership.drain_replace]
#[test]
fn role_slot_high_water_survives_reopen_and_supersedes_old_assignment_ids() {
    assert_role_slot_fencing(&mut crate::fabric_membership::persistence::InMemory::default());
    let (_workspace, root) = assignment_store_root("assignment-role-high-water");
    let mut persistence = crate::fabric_membership::persistence::Redb::open(&root).expect("durable owner");
    let successor = assert_role_slot_fencing(&mut persistence);
    drop(persistence);
    let reopened = crate::fabric_membership::persistence::Redb::open(&root).expect("reopen role index");
    assert!(reopened.current("assignment-a").is_err());
    assert_eq!(reopened.current("assignment-b").expect("durable role slot"), Some(successor));
}

#[test]
fn canonical_current_identity_binds_optional_refs_history_epoch_and_token() {
    let mut first = crate::fabric_membership::persistence::InMemory::default();
    let baseline = first.initialize(&proposal()).expect("baseline");
    for next in [
        AssignmentProposal { durable_state_ref: None, ..proposal() },
        AssignmentProposal { fencing_token: FENCING_TOKEN + 1, ..proposal() },
        AssignmentProposal { assignment_epoch: ASSIGNMENT_EPOCH + 1, ..proposal() },
        AssignmentProposal { node_id: "node-other".to_string(), ..proposal() },
    ] {
        let mut owner = crate::fabric_membership::persistence::InMemory::default();
        assert_ne!(owner.initialize(&next).expect("changed proposal").assignment_ref, baseline.assignment_ref);
    }
    let reserve = |label: &str| {
        let mut owner = crate::fabric_membership::persistence::InMemory::default();
        let initial = owner.initialize(&proposal()).expect("initial").assignment;
        let transition = apply_assignment_command(&initial, &command(&initial, AssignmentCommandKind::Reserve, label))
            .expect("reserve");
        let intent = owner.record_intent(&initial, &transition).expect("intent");
        owner.commit(&transition, &intent, None).expect("reserve commit");
        owner.current("assignment-a").expect("current").expect("reserved")
    };
    assert_ne!(reserve("history-a").assignment_ref, reserve("history-b").assignment_ref);
    let memory = initialized_memory().current("assignment-a").expect("memory current");
    let (_workspace, root) = assignment_store_root("assignment-canonical-parity");
    let mut durable = crate::fabric_membership::persistence::Redb::open(&root).expect("durable");
    initialize_acknowledged(&mut durable);
    assert_eq!(durable.current("assignment-a").expect("durable current"), memory);
}

#[test]
fn existing_empty_assignment_database_is_not_fresh_history() {
    let (_workspace, root) = assignment_store_root("assignment-empty-existing");
    let path = crate::local_store::Locator::parse("assignments.redb").expect("database locator");
    drop(root.root().create_database_file(&path).expect("empty interrupted database"));
    assert!(crate::fabric_membership::persistence::Redb::open(&root).is_err());
    assert_eq!(root.root().open_database_file(&path).expect("retained database").metadata().expect("metadata").len(), 0);
}

#[test]
fn pure_successor_validator_uses_role_slot_not_node_and_requires_exact_released_predecessor() {
    let mut owner = initialized_memory();
    let assignment = acknowledged_assignment();
    let release = apply_assignment_command(&assignment,
        &command(&assignment, AssignmentCommandKind::Release, "release-validator")).expect("release");
    let intent = owner.record_intent(&assignment, &release).expect("intent");
    owner.commit(&release, &intent, Some(&test_ref("release-validator-effect"))).expect("released");
    let predecessor = owner.current("assignment-a").expect("current").expect("predecessor");
    let successor = propose_assignment(&successor_proposal(&predecessor)).expect("pure successor");
    assert!(validate_assignment_successor(&predecessor.assignment, &predecessor.assignment_ref, &successor).is_empty());
    let mut wrong_slot = successor.clone();
    wrong_slot.role_id = "different-role".to_string();
    assert!(validate_assignment_successor(&predecessor.assignment, &predecessor.assignment_ref, &wrong_slot)
        .contains(&AssignmentIssue::RoleOwnershipMismatch));
    let mut same_id = successor;
    same_id.assignment_id = predecessor.assignment.assignment_id.clone();
    assert!(validate_assignment_successor(&predecessor.assignment, &predecessor.assignment_ref, &same_id)
        .contains(&AssignmentIssue::RoleOwnershipMismatch));
}
