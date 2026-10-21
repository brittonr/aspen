use super::super::tests::support::*;
use super::*;

fn observed() -> ReconcileInput {
    let manifest = manifest();
    ReconcileInput {
        inventory: Inventory {
            replicas: vec![source_replica(&manifest)],
        },
        manifest,
        peers: vec![peer("peer-a", "zone-a"), peer("peer-b", "zone-b")],
        history: Vec::new(),
        observed_tick: 1,
    }
}

fn recorded(action: &Action, outcome: OperationOutcome) -> PriorOperation {
    PriorOperation {
        operation_id: action.operation_id.clone(),
        content_ref: action.content_ref.clone(),
        source_peer: action.source_peer.clone(),
        target_peer: action.target_peer.clone(),
        generation: GENERATION,
        membership_epoch: MEMBERSHIP_EPOCH,
        placement_epoch: PLACEMENT_EPOCH,
        attempt: action.attempt,
        outcome,
        result_ref: Some(digest('b')),
    }
}

// r[verify molten.audit_f04.replay_preservation]
#[test]
fn exact_saved_data_operation_replays_its_outcome_without_duplicate_effects() {
    let observed = observed();
    let plan = molten_core::content_replication::plan(&observed).expect("saved plan");
    for outcome in [
        OperationOutcome::Verified,
        OperationOutcome::Failed,
        OperationOutcome::Cancelled,
        OperationOutcome::Uncertain,
        OperationOutcome::Corrupt,
    ] {
        let operation = recorded(&plan.actions[0], outcome);
        let record = canonical_operation(&operation).expect("historical operation");
        let history = vec![operation.clone()];
        let events = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut facts = super::super::tests::support::FactPorts::admitted(&events);
        let mut effects = super::super::tests::adapters::EffectPorts::admitted(&observed.manifest, &events);
        effects.durable.history = history.clone();
        let execution = execute_actions(
            &observed.manifest,
            &plan,
            &history,
            &mut Vec::new(),
            super::super::tests::ports(&mut facts, &mut effects),
        )
        .expect("exact operation replay");
        assert_eq!(execution.operations, history);
        assert_eq!(canonical_operation(&execution.operations[0]).expect("unchanged operation").bytes, record.bytes);
        let projected = molten_core::content_replication::status(&plan, &execution.operations);
        assert_eq!(projected.under_replicated.is_empty(), outcome == OperationOutcome::Verified);
        assert_eq!(projected.failures.is_empty(), outcome == OperationOutcome::Verified);
        assert!(projected.active_operations.is_empty());
        drop(execution);
        assert_eq!(effects.durable.history, history);
        assert!(effects.durable.stored.is_empty());
        assert_eq!(effects.transport.calls, 0);
        assert!(events.borrow().is_empty());
    }
}

// r[verify molten.audit_f04.replay_preservation]
#[test]
fn mismatched_duplicate_or_unfinished_saved_operations_reject_before_effects() {
    let observed = observed();
    let plan = molten_core::content_replication::plan(&observed).expect("saved plan");
    for mismatch in 0..9 {
        let mut operation = recorded(&plan.actions[0], OperationOutcome::Verified);
        match mismatch {
            0 => operation.content_ref = digest('c'),
            1 => operation.source_peer = Some("peer-c".to_string()),
            2 => operation.target_peer = "peer-c".to_string(),
            3 => operation.generation = GENERATION.saturating_add(1),
            4 => operation.attempt = operation.attempt.saturating_add(1),
            5 => operation.result_ref = None,
            6 => operation.result_ref = Some("malformed-result".to_string()),
            7 => operation.outcome = OperationOutcome::Planned,
            _ => {}
        }
        let mut history = vec![operation.clone()];
        if mismatch == 8 {
            history.push(operation);
        }
        let events = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut facts = super::super::tests::support::FactPorts::admitted(&events);
        let mut effects = super::super::tests::adapters::EffectPorts::admitted(&observed.manifest, &events);
        effects.durable.history = history.clone();
        let inventory = effects.content.inventory.clone();
        assert!(
            execute_actions(
                &observed.manifest,
                &plan,
                &history,
                &mut Vec::new(),
                super::super::tests::ports(&mut facts, &mut effects)
            )
            .is_err()
        );
        assert_eq!(effects.durable.history, history);
        assert_eq!(effects.content.inventory, inventory);
        assert!(effects.durable.stored.is_empty());
        assert_eq!(effects.transport.calls, 0);
        assert!(events.borrow().is_empty());
    }
}

// r[verify molten.audit_f05.authority_preservation]
// r[verify molten.audit_f05.validation_claims]
// r[verify molten.audit_f04.replay_preservation]
#[test]
fn recorded_cleanup_success_cannot_bypass_current_deletion_admission() {
    let mut observed = observed();
    let mut redundant = source_replica(&observed.manifest);
    redundant.peer_id = "peer-b".to_string();
    let mut unique = redundant.clone();
    unique.peer_id = "peer-c".to_string();
    unique.fault_domain = "zone-b".to_string();
    observed.inventory.replicas.extend([redundant, unique]);
    for replica in &mut observed.inventory.replicas {
        replica.pinned = false;
        replica.cleanup_clearance_ref = Some(digest('d'));
    }
    observed.peers = vec![
        peer("peer-a", "zone-a"),
        peer("peer-b", "zone-a"),
        peer("peer-c", "zone-b"),
    ];
    let plan = molten_core::content_replication::plan(&observed).expect("safe cleanup plan");
    assert_eq!(plan.actions[0].kind, ActionKind::Cleanup);
    let history = vec![recorded(&plan.actions[0], OperationOutcome::Verified)];
    let events = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut facts = super::super::tests::support::FactPorts::admitted(&events);
    let mut effects = super::super::tests::adapters::EffectPorts::admitted(&observed.manifest, &events);
    effects.durable.history = history.clone();
    effects.retention.cleanup_admitted = false;
    assert!(
        execute_actions(
            &observed.manifest,
            &plan,
            &history,
            &mut Vec::new(),
            super::super::tests::ports(&mut facts, &mut effects)
        )
        .is_err()
    );
    assert_eq!(effects.content.cleanup_count, 0);
    assert_eq!(effects.durable.history, history);
    assert!(effects.durable.stored.is_empty());
    assert!(events.borrow().contains(&"authorize-cleanup"));
    assert!(!events.borrow().contains(&"cleanup"));
}
