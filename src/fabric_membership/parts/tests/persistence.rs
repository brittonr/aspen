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

fn execute_uncertain_activation(
    persistence: &mut dyn AssignmentPersistence,
    lifecycle: &mut RecordingLifecycle,
    acknowledged: &RoleAssignment,
    label: &str,
) -> UncertainAssignmentExecution {
    let activate = command(acknowledged, AssignmentCommandKind::Activate, label);
    let outcome = execute_assignment_command(persistence, lifecycle, acknowledged, &activate, &authority_snapshot(acknowledged))
        .expect("pure activation");
    let AssignmentExecutionOutcome::Uncertain(uncertain) = outcome else { panic!("activation must be uncertain") };
    uncertain
}

fn scripted(observation: RoleEffectObservation) -> RecordingLifecycle {
    RecordingLifecycle { scripted_observation: Some(observation), ..RecordingLifecycle::default() }
}

fn intent_evidence(pending: &PendingAssignmentIntent) -> RoleIntentEvidence {
    RoleIntentEvidence {
        assignment_id: pending.transition.next.assignment_id.clone(),
        transition_ref: pending.transition.next.transition_refs.last().expect("transition ref").clone(),
        kind: pending.transition.kind,
        intent_ref: pending.intent_ref.clone(),
        evidence_ref: test_ref("port-evidence"),
    }
}

fn assert_port_observed_pre_effect_abort(persistence: &mut dyn AssignmentPersistence) {
    let acknowledged = initialize_acknowledged(persistence);
    let mut refusing = RecordingLifecycle { refuse_activate: true, ..RecordingLifecycle::default() };
    let uncertain = execute_uncertain_activation(persistence, &mut refusing, &acknowledged, "activate-refused");
    assert_eq!(uncertain.phase, AssignmentExecutionPhase::RoleEffect);
    assert!(!uncertain.effect_may_have_happened);
    assert!(persistence.current("assignment-a").is_err(), "the failed effect left a pending intent");

    let pending = persistence.inspect_pending("assignment-a").expect("inspect").expect("pending intent");
    assert_eq!(pending.committed, acknowledged);
    assert_eq!(pending.transition, uncertain.transition);
    assert_eq!(Some(pending.intent_ref.clone()), uncertain.intent_ref);
    assert!(pending.committed_ref.starts_with("blake3:"));

    // A caller's `effect_may_have_happened == false` is not evidence: a port without an
    // intent-bound record cannot authorize the abort.
    let denied = persistence
        .abort_pre_effect_intent("assignment-a", &pending.intent_ref, &mut RecordingLifecycle::default())
        .expect_err("no port evidence");
    assert_eq!(denied.class(), crate::error::FailureClass::UncertainExternalOutcome);
    // Port evidence must name this exact intent, transition and kind.
    let exact = intent_evidence(&pending);
    for forged in [
        RoleIntentEvidence { intent_ref: test_ref("other-intent"), ..exact.clone() },
        RoleIntentEvidence { transition_ref: test_ref("other-transition"), ..exact.clone() },
        RoleIntentEvidence { kind: AssignmentCommandKind::Release, ..exact.clone() },
        RoleIntentEvidence { evidence_ref: "malformed".to_string(), ..exact.clone() },
    ] {
        let mut port = scripted(RoleEffectObservation::NotExecuted(forged));
        assert!(persistence.abort_pre_effect_intent("assignment-a", &pending.intent_ref, &mut port).is_err());
    }
    let mut executed = scripted(RoleEffectObservation::Executed(exact));
    assert!(persistence.abort_pre_effect_intent("assignment-a", &pending.intent_ref, &mut executed).is_err());
    assert!(persistence.abort_pre_effect_intent("assignment-a", &test_ref("other-intent"), &mut refusing).is_err());
    assert_eq!(persistence.inspect_pending("assignment-a").expect("still pending"), Some(pending.clone()));

    let restored = persistence
        .abort_pre_effect_intent("assignment-a", &pending.intent_ref, &mut refusing)
        .expect("port-observed pre-effect abort");
    assert_eq!(restored.assignment, acknowledged);
    assert_eq!(persistence.inspect_pending("assignment-a").expect("cleared"), None);
    assert_eq!(persistence.current("assignment-a").expect("current").expect("owner"), restored);
    assert!(persistence.abort_pre_effect_intent("assignment-a", &pending.intent_ref, &mut refusing).is_err());

    // The role slot proceeds with the same transition once the effect succeeds.
    let mut lifecycle = RecordingLifecycle::default();
    let activate = command(&acknowledged, AssignmentCommandKind::Activate, "activate-refused");
    let outcome = execute_assignment_command(persistence, &mut lifecycle, &acknowledged, &activate, &authority_snapshot(&acknowledged))
        .expect("activation retry");
    let AssignmentExecutionOutcome::Committed(receipt) = outcome else { panic!("retry must commit") };
    assert_eq!(receipt.transition.next.state, AssignmentState::Active);
    assert_eq!(lifecycle.calls.len(), 1);
}

// r[verify molten.fabric_membership.recoverable_pending_and_quarantined_slots]
#[test]
fn port_observed_pre_effect_failure_aborts_pending_intent_and_the_role_slot_proceeds() {
    assert_port_observed_pre_effect_abort(&mut crate::fabric_membership::persistence::InMemory::default());
    let (_workspace, root) = assignment_store_root("assignment-pre-effect-abort");
    assert_port_observed_pre_effect_abort(&mut crate::fabric_membership::persistence::Redb::open(&root).expect("durable owner"));
}

// r[verify molten.fabric_membership.recoverable_pending_and_quarantined_slots]
#[test]
fn reopened_intent_without_port_evidence_can_only_reconcile_to_quarantine() {
    let (_workspace, root) = assignment_store_root("assignment-crash-no-evidence");
    let mut persistence = crate::fabric_membership::persistence::Redb::open(&root).expect("durable owner");
    let acknowledged = initialize_acknowledged(&mut persistence);
    let mut refusing = RecordingLifecycle { refuse_activate: true, ..RecordingLifecycle::default() };
    let uncertain = execute_uncertain_activation(&mut persistence, &mut refusing, &acknowledged, "activate-crashed");
    assert!(!uncertain.effect_may_have_happened);
    drop(persistence);

    // After the crash the replacement port holds no record of the refused intent.
    let mut reopened = crate::fabric_membership::persistence::Redb::open(&root).expect("reopen after crash");
    let pending = reopened.inspect_pending("assignment-a").expect("inspect").expect("pending survives reopen");
    let mut restarted_port = RecordingLifecycle::default();
    let denied = reopened
        .abort_pre_effect_intent("assignment-a", &pending.intent_ref, &mut restarted_port)
        .expect_err("no intent-bound evidence after reopen");
    assert_eq!(denied.class(), crate::error::FailureClass::UncertainExternalOutcome);
    assert!(reopened
        .reconcile_pending_intent("assignment-a", &pending.intent_ref, PendingIntentReconciliation::ConfirmObservedEffect { lifecycle: &mut restarted_port })
        .is_err(), "an unknown outcome cannot be confirmed");
    let mut authority = RecordingQuarantineAuthority::default();
    let reconciled = reopened
        .reconcile_pending_intent("assignment-a", &pending.intent_ref, PendingIntentReconciliation::QuarantineUncertain { authority: &mut authority })
        .expect("authorized, uncertainty-preserving reconciliation");
    assert_eq!(reconciled.current.assignment.state, AssignmentState::Quarantined);
    assert!(reconciled.current.assignment.uncertain_old_owner);
    assert!(restarted_port.calls.is_empty(), "recovery never replays the role effect");
    assert_eq!(authority.requests.len(), 1);
    assert_eq!(authority.requests[0].committed_assignment_ref, pending.committed_ref, "authorization binds the committed state");
    drop(reopened);
    let reopened = crate::fabric_membership::persistence::Redb::open(&root).expect("reopen after reconcile");
    let record = reopened.quarantine_reconciliation("assignment-a").expect("read").expect("persisted reconciliation");
    assert_eq!(record.reconcile_ref, reconciled.reconcile_ref);
    assert_eq!(record.basis, QuarantineReconciliationBasis::ReconciledIntent { intent_ref: pending.intent_ref.clone() });
    assert_eq!(record.evidence_ref, test_ref(&format!("authorization-{}", pending.intent_ref)));
    assert_eq!(record.quarantined_assignment_ref, reconciled.current.assignment_ref);
}

// r[verify molten.fabric_membership.recoverable_pending_and_quarantined_slots]
#[test]
fn crashed_effectless_intent_aborts_from_its_persisted_kind_after_reopen() {
    let (_workspace, root) = assignment_store_root("assignment-effectless-abort");
    let mut persistence = crate::fabric_membership::persistence::Redb::open(&root).expect("durable owner");
    let proposed = persistence.initialize(&proposal()).expect("initialize").assignment;
    let reserve = apply_assignment_command(&proposed, &command(&proposed, AssignmentCommandKind::Reserve, "reserve-crash"))
        .expect("reserve");
    let intent_ref = persistence.record_intent(&proposed, &reserve).expect("intent");
    drop(persistence);

    let mut reopened = crate::fabric_membership::persistence::Redb::open(&root).expect("reopen after crash");
    assert!(reopened.current("assignment-a").is_err());
    assert_eq!(reopened.inspect_pending("assignment-a").expect("inspect").expect("pending").intent_ref, intent_ref);
    // A reserve has no role effect, so no port evidence is needed or consulted.
    let mut port = RecordingLifecycle::default();
    let restored = reopened.abort_pre_effect_intent("assignment-a", &intent_ref, &mut port).expect("effectless abort");
    assert_eq!(restored.assignment, proposed);
    assert!(port.calls.is_empty());
    drop(reopened);
    let reopened = crate::fabric_membership::persistence::Redb::open(&root).expect("reopen after abort");
    assert_eq!(reopened.current("assignment-a").expect("durable abort"), Some(restored));
}

fn assert_quarantine_successor_requires_record_and_observed_fence(
    persistence: &mut dyn AssignmentPersistence,
    quarantined: &CurrentAssignment,
) -> CurrentAssignment {
    let record = persistence.quarantine_reconciliation("assignment-a").expect("read").expect("persisted reconciliation");
    assert_eq!(record.quarantined_assignment_ref, quarantined.assignment_ref);
    let successor = successor_proposal(quarantined);
    let successor_assignment = propose_assignment(&successor).expect("pure successor");
    assert!(persistence.initialize(&successor).is_err(), "a quarantined owner is not a released predecessor");
    let clearance = QuarantineClearance {
        quarantined_assignment_ref: quarantined.assignment_ref.clone(),
        reconciliation_ref: record.reconcile_ref.clone(),
    };
    // The successor runs on another node, so only a cross-node fence can exclude the old owner.
    assert_ne!(successor_assignment.node_id, quarantined.assignment.node_id);
    let advanced = || ObservedFence(Some(enforced_fence(&successor_assignment, FencingEnforcement::QuorumOrdered)));
    for (invalid, mut fence) in [
        // A reconciliation the owner never persisted.
        (QuarantineClearance { reconciliation_ref: test_ref("fabricated-reconciliation"), ..clearance.clone() }, advanced()),
        (QuarantineClearance { quarantined_assignment_ref: test_ref("other-assignment"), ..clearance.clone() }, advanced()),
        // The fence cannot be observed, or the authority still enforces the quarantined owner.
        (clearance.clone(), ObservedFence(None)),
        (clearance.clone(), ObservedFence(Some(enforced_fence(&quarantined.assignment, FencingEnforcement::QuorumOrdered)))),
        // An advanced but node-local fence cannot exclude an owner on another node.
        (clearance.clone(), ObservedFence(Some(enforced_fence(&successor_assignment, FencingEnforcement::NodeLocalDurable)))),
    ] {
        assert!(persistence.initialize_quarantine_successor(&successor, &invalid, &mut fence).is_err(), "{invalid:?}");
        assert_eq!(persistence.current("assignment-a").expect("owner kept"), Some(quarantined.clone()));
        assert!(persistence.current("assignment-b").expect("no partial successor").is_none());
    }
    let stale_token = AssignmentProposal { fencing_token: quarantined.assignment.fencing_token, ..successor.clone() };
    assert!(persistence.initialize_quarantine_successor(&stale_token, &clearance, &mut advanced()).is_err());
    let admitted = persistence
        .initialize_quarantine_successor(&successor, &clearance, &mut advanced())
        .expect("reconciled, observed-fenced successor");
    assert_eq!(admitted.assignment.state, AssignmentState::Proposed);
    assert!(persistence.current("assignment-a").is_err(), "the quarantined ID is superseded");
    assert_eq!(persistence.current("assignment-b").expect("successor"), Some(admitted.clone()));
    admitted
}

fn assert_post_effect_reconcile_and_quarantine_succession(persistence: &mut dyn AssignmentPersistence) -> CurrentAssignment {
    let acknowledged = initialize_acknowledged(persistence);
    let mut uncertain_effect = RecordingLifecycle { fail_activate: true, ..RecordingLifecycle::default() };
    let uncertain = execute_uncertain_activation(persistence, &mut uncertain_effect, &acknowledged, "activate-maybe");
    assert!(uncertain.effect_may_have_happened);
    let intent_ref = uncertain.intent_ref.clone().expect("intent recorded");

    let denied = persistence
        .abort_pre_effect_intent("assignment-a", &intent_ref, &mut uncertain_effect)
        .expect_err("possible effect is never pre-effect");
    assert_eq!(denied.class(), crate::error::FailureClass::UncertainExternalOutcome);
    let denied = persistence
        .reconcile_pending_intent("assignment-a", &intent_ref, PendingIntentReconciliation::ConfirmObservedEffect { lifecycle: &mut uncertain_effect })
        .expect_err("unknown outcome cannot be confirmed");
    assert_eq!(denied.class(), crate::error::FailureClass::UncertainExternalOutcome);

    // Quarantine needs the quarantine authority's decision for this exact request.
    let mut denying = RecordingQuarantineAuthority { deny: true, ..RecordingQuarantineAuthority::default() };
    assert!(persistence
        .reconcile_pending_intent("assignment-a", &intent_ref, PendingIntentReconciliation::QuarantineUncertain { authority: &mut denying })
        .is_err());
    let mut misbound = RecordingQuarantineAuthority { answer_other_request: true, ..RecordingQuarantineAuthority::default() };
    assert!(persistence
        .reconcile_pending_intent("assignment-a", &intent_ref, PendingIntentReconciliation::QuarantineUncertain { authority: &mut misbound })
        .is_err());
    let mut authority = RecordingQuarantineAuthority::default();
    assert!(persistence
        .reconcile_pending_intent("assignment-a", &test_ref("wrong-intent"), PendingIntentReconciliation::QuarantineUncertain { authority: &mut authority })
        .is_err());
    assert!(authority.requests.is_empty(), "a wrong intent is refused before asking the authority");
    assert!(persistence.inspect_pending("assignment-a").expect("inspect").is_some(), "failed reconciliation keeps the intent");
    assert!(persistence.quarantine_reconciliation("assignment-a").expect("read").is_none());

    let reconciled = persistence
        .reconcile_pending_intent("assignment-a", &intent_ref, PendingIntentReconciliation::QuarantineUncertain { authority: &mut authority })
        .expect("authorized uncertain quarantine");
    assert!(reconciled.reconcile_ref.starts_with("blake3:"));
    assert_eq!(reconciled.current.assignment.state, AssignmentState::Quarantined);
    assert!(reconciled.current.assignment.uncertain_old_owner, "uncertainty is preserved");
    assert_eq!(uncertain_effect.calls.len(), 1, "the pending role effect is never replayed");
    assert_eq!(persistence.inspect_pending("assignment-a").expect("inspect"), None);
    let quarantined = persistence.current("assignment-a").expect("current").expect("quarantined owner");
    assert_eq!(quarantined, reconciled.current);
    let record = persistence.quarantine_reconciliation("assignment-a").expect("read").expect("record");
    assert_eq!(record.reconcile_ref, reconciled.reconcile_ref);
    assert_eq!(record.evidence_ref, test_ref(&format!("authorization-{intent_ref}")));
    assert_quarantine_successor_requires_record_and_observed_fence(persistence, &quarantined)
}

// r[verify molten.fabric_membership.recoverable_pending_and_quarantined_slots]
#[test]
fn post_effect_uncertainty_reconciles_to_quarantine_and_a_fenced_successor() {
    assert_post_effect_reconcile_and_quarantine_succession(&mut crate::fabric_membership::persistence::InMemory::default());
    let (_workspace, root) = assignment_store_root("assignment-post-effect-reconcile");
    let mut persistence = crate::fabric_membership::persistence::Redb::open(&root).expect("durable owner");
    let successor = assert_post_effect_reconcile_and_quarantine_succession(&mut persistence);
    drop(persistence);
    let reopened = crate::fabric_membership::persistence::Redb::open(&root).expect("reopen");
    assert!(reopened.current("assignment-a").is_err());
    assert_eq!(reopened.current("assignment-b").expect("durable successor"), Some(successor));
}

fn assert_committed_quarantine_records_its_effect(persistence: &mut dyn AssignmentPersistence) {
    let acknowledged = initialize_acknowledged(persistence);
    let mut lifecycle = RecordingLifecycle::default();
    let quarantine = command(&acknowledged, AssignmentCommandKind::Quarantine, "quarantine-committed");
    let outcome = execute_assignment_command(persistence, &mut lifecycle, &acknowledged, &quarantine, &authority_snapshot(&acknowledged))
        .expect("quarantine");
    let AssignmentExecutionOutcome::Committed(receipt) = outcome else { panic!("quarantine must commit") };
    let quarantined = persistence.current("assignment-a").expect("current").expect("quarantined");
    assert_eq!(quarantined.assignment, receipt.transition.next);
    let record = persistence.quarantine_reconciliation("assignment-a").expect("read").expect("record");
    assert_eq!(Some(&record.evidence_ref), lifecycle.executed.last().map(|(_, _, effect_ref)| effect_ref));
    assert_quarantine_successor_requires_record_and_observed_fence(persistence, &quarantined);
}

// r[verify molten.fabric_membership.recoverable_pending_and_quarantined_slots]
#[test]
fn committed_quarantine_records_its_role_effect_as_reconciliation() {
    assert_committed_quarantine_records_its_effect(&mut crate::fabric_membership::persistence::InMemory::default());
    let (_workspace, root) = assignment_store_root("assignment-committed-quarantine");
    assert_committed_quarantine_records_its_effect(&mut crate::fabric_membership::persistence::Redb::open(&root).expect("durable owner"));
}

fn assert_observed_effect_confirms(persistence: &mut dyn AssignmentPersistence) {
    let acknowledged = initialize_acknowledged(persistence);
    let mut lost_ack = RecordingLifecycle { lost_activate_ack: true, ..RecordingLifecycle::default() };
    let uncertain = execute_uncertain_activation(persistence, &mut lost_ack, &acknowledged, "activate-confirm");
    assert!(uncertain.effect_may_have_happened);
    let intent_ref = uncertain.intent_ref.clone().expect("intent");
    let pending = persistence.inspect_pending("assignment-a").expect("inspect").expect("pending");

    // Confirmation needs the port's executed-effect evidence for this exact intent.
    let exact = intent_evidence(&pending);
    for mut port in [
        RecordingLifecycle::default(),
        scripted(RoleEffectObservation::Executed(RoleIntentEvidence { intent_ref: test_ref("other-intent"), ..exact.clone() })),
        scripted(RoleEffectObservation::Executed(RoleIntentEvidence { transition_ref: test_ref("other-transition"), ..exact.clone() })),
        scripted(RoleEffectObservation::NotExecuted(exact.clone())),
    ] {
        let denied = persistence
            .reconcile_pending_intent("assignment-a", &intent_ref, PendingIntentReconciliation::ConfirmObservedEffect { lifecycle: &mut port })
            .expect_err("not confirmed");
        assert_eq!(denied.class(), crate::error::FailureClass::UncertainExternalOutcome);
    }
    assert_eq!(persistence.inspect_pending("assignment-a").expect("still pending"), Some(pending));

    let reconciled = persistence
        .reconcile_pending_intent("assignment-a", &intent_ref, PendingIntentReconciliation::ConfirmObservedEffect { lifecycle: &mut lost_ack })
        .expect("observed effect");
    assert_eq!(reconciled.current.assignment, uncertain.transition.next);
    assert_eq!(reconciled.current.assignment.state, AssignmentState::Active);
    assert!(!reconciled.current.assignment.uncertain_old_owner);
    assert_eq!(lost_ack.calls.len(), 1, "confirmation never replays the effect");
    assert_eq!(persistence.current("assignment-a").expect("current"), Some(reconciled.current));
    assert!(persistence
        .reconcile_pending_intent("assignment-a", &intent_ref, PendingIntentReconciliation::ConfirmObservedEffect { lifecycle: &mut lost_ack })
        .is_err(), "a resolved intent cannot be reconciled twice");
}

// r[verify molten.fabric_membership.recoverable_pending_and_quarantined_slots]
#[test]
fn observed_effect_reconciles_by_committing_the_exact_pending_transition() {
    assert_observed_effect_confirms(&mut crate::fabric_membership::persistence::InMemory::default());
    let (_workspace, root) = assignment_store_root("assignment-confirmed-reconcile");
    assert_observed_effect_confirms(&mut crate::fabric_membership::persistence::Redb::open(&root).expect("durable owner"));
}

/// Rewrites a durable store into the format written before reconciliation records existed.
fn drop_reconciliation_table(root: &crate::local_store::DurableStoreRoot) {
    let path = crate::local_store::Locator::parse("assignments.redb").expect("database locator");
    let file = root.root().open_database_file(&path).expect("existing database");
    let database = redb::Database::builder().create_file(file).expect("raw database");
    let write = database.begin_write().expect("write");
    let table: redb::TableDefinition<&str, &[u8]> =
        redb::TableDefinition::new("fabric-assignment-quarantine-reconciliations-v1");
    assert!(write.delete_table(table).expect("delete table"), "the reconciliation table existed");
    write.commit().expect("commit legacy format");
}

// r[verify molten.fabric_membership.recoverable_pending_and_quarantined_slots]
#[test]
fn legacy_quarantined_record_migrates_only_with_exact_authorization() {
    let (_workspace, root) = assignment_store_root("assignment-legacy-quarantine");
    let mut persistence = crate::fabric_membership::persistence::Redb::open(&root).expect("durable owner");
    let acknowledged = initialize_acknowledged(&mut persistence);
    let mut lifecycle = RecordingLifecycle::default();
    let quarantine = command(&acknowledged, AssignmentCommandKind::Quarantine, "legacy-quarantine");
    let outcome = execute_assignment_command(&mut persistence, &mut lifecycle, &acknowledged, &quarantine, &authority_snapshot(&acknowledged))
        .expect("quarantine");
    assert!(matches!(outcome, AssignmentExecutionOutcome::Committed(_)));
    drop(persistence);
    drop_reconciliation_table(&root);

    // The old-format store reopens; its quarantined record has no reconciliation and no intent.
    let mut reopened = crate::fabric_membership::persistence::Redb::open(&root).expect("reopen legacy store");
    let quarantined = reopened.current("assignment-a").expect("current").expect("quarantined owner");
    assert_eq!(quarantined.assignment.state, AssignmentState::Quarantined);
    assert_eq!(reopened.inspect_pending("assignment-a").expect("inspect"), None);
    assert_eq!(reopened.quarantine_reconciliation("assignment-a").expect("read"), None);
    let successor = successor_proposal(&quarantined);
    let successor_assignment = propose_assignment(&successor).expect("pure successor");
    let fence = || ObservedFence(Some(enforced_fence(&successor_assignment, FencingEnforcement::QuorumOrdered)));
    let unreconciled = QuarantineClearance {
        quarantined_assignment_ref: quarantined.assignment_ref.clone(),
        reconciliation_ref: test_ref("unrecorded-reconciliation"),
    };
    assert!(reopened.initialize_quarantine_successor(&successor, &unreconciled, &mut fence()).is_err());

    let mut denying = RecordingQuarantineAuthority { deny: true, ..RecordingQuarantineAuthority::default() };
    assert!(reopened.reconcile_legacy_quarantine("assignment-a", &mut denying).is_err());
    let mut misbound = RecordingQuarantineAuthority { answer_other_request: true, ..RecordingQuarantineAuthority::default() };
    assert!(reopened.reconcile_legacy_quarantine("assignment-a", &mut misbound).is_err());
    assert_eq!(reopened.quarantine_reconciliation("assignment-a").expect("read"), None, "failed migration records nothing");

    let mut authority = RecordingQuarantineAuthority::default();
    let migrated = reopened.reconcile_legacy_quarantine("assignment-a", &mut authority).expect("authorized migration");
    let quarantine_transition = quarantined.assignment.transition_refs.last().expect("transition").clone();
    assert_eq!(migrated.basis, QuarantineReconciliationBasis::LegacyRecord { transition_ref: quarantine_transition.clone() });
    assert_eq!(migrated.quarantined_assignment_ref, quarantined.assignment_ref);
    assert_eq!(authority.requests[0].committed_assignment_ref, quarantined.assignment_ref);
    assert_eq!(migrated.evidence_ref, test_ref(&format!("authorization-{quarantine_transition}")));
    assert!(reopened.reconcile_legacy_quarantine("assignment-a", &mut authority).is_err(), "migration never overwrites");
    drop(reopened);

    let mut reopened = crate::fabric_membership::persistence::Redb::open(&root).expect("reopen migrated store");
    assert_eq!(reopened.quarantine_reconciliation("assignment-a").expect("read"), Some(migrated.clone()));
    let clearance = QuarantineClearance {
        quarantined_assignment_ref: quarantined.assignment_ref.clone(),
        reconciliation_ref: migrated.reconcile_ref.clone(),
    };
    let admitted = reopened.initialize_quarantine_successor(&successor, &clearance, &mut fence()).expect("successor");
    assert_eq!(reopened.current("assignment-b").expect("successor"), Some(admitted));
}

// r[verify molten.fabric_membership.recoverable_pending_and_quarantined_slots]
#[test]
fn legacy_migration_requires_a_quarantined_record_without_pending_intent() {
    let mut persistence = crate::fabric_membership::persistence::InMemory::default();
    let acknowledged = initialize_acknowledged(&mut persistence);
    let mut authority = RecordingQuarantineAuthority::default();
    assert!(persistence.reconcile_legacy_quarantine("assignment-a", &mut authority).is_err(), "not quarantined");
    let mut uncertain_effect = RecordingLifecycle { fail_activate: true, ..RecordingLifecycle::default() };
    execute_uncertain_activation(&mut persistence, &mut uncertain_effect, &acknowledged, "activate-before-migration");
    assert!(persistence.reconcile_legacy_quarantine("assignment-a", &mut authority).is_err(), "pending intent");
    assert!(authority.requests.is_empty(), "nothing is asked of the authority for an ineligible record");
    assert!(persistence.reconcile_legacy_quarantine("assignment-missing", &mut authority).is_err());
}
