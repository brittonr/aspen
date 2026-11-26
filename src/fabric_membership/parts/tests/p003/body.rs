// Recovery of pending assignment intents by port-observed abort or quarantine.

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
