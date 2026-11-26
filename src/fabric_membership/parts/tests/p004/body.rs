// Quarantine reconciliation, legacy migration and fenced quarantine succession.

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
