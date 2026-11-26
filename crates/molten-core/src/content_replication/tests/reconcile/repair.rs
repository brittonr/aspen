use super::*;

const SOURCE_PEER: &str = "peer-a";
const TARGET_PEER: &str = "peer-b";
const TARGET_DOMAIN: &str = "zone-b";
const FIRST_TICK: u64 = 1;
const LOSS_TICK: u64 = 2;
const FIRST_ATTEMPT: u32 = 1;
const SECOND_ATTEMPT: u32 = 2;
const THIRD_ATTEMPT: u32 = 3;
const CURRENT_SOURCE_COUNT: usize = 1;

fn transfer(plan: &Plan) -> &Action {
    plan.actions
        .iter()
        .find(|action| matches!(action.kind, ActionKind::Transfer | ActionKind::Repair | ActionKind::Handoff))
        .expect("admitted data action")
}

fn verified(action: &Action) -> PriorOperation {
    PriorOperation {
        operation_id: action.operation_id.clone(),
        content_ref: action.content_ref.clone(),
        source_peer: action.source_peer.clone(),
        target_peer: action.target_peer.clone(),
        generation: GENERATION,
        membership_epoch: MEMBERSHIP_EPOCH,
        placement_epoch: PLACEMENT_EPOCH,
        attempt: action.attempt,
        outcome: OperationOutcome::Verified,
        result_ref: Some(digest('b')),
    }
}

fn after_success() -> (ReconcileInput, Plan) {
    let mut observed = input();
    observed.peers.truncate(DEFAULT_REPLICAS);
    observed.observed_tick = FIRST_TICK;
    let initial = plan(&observed).expect("initial plan");
    observed.history.push(verified(transfer(&initial)));
    observed.observed_tick = LOSS_TICK;
    (observed, initial)
}

// r[verify molten.audit_f04.current_availability]
// r[verify molten.audit_f04.fresh_repair]
#[test]
fn audit_old_transfer_success_does_not_override_current_absence() {
    let (observed, original) = after_success();
    let snapshot = observed.clone();
    let repaired = plan(&observed).expect("fresh loss plan");
    let action = transfer(&repaired);
    assert_eq!(action.kind, ActionKind::Transfer);
    assert_eq!(action.source_peer.as_deref(), Some(SOURCE_PEER));
    assert_eq!(action.target_peer, TARGET_PEER);
    assert_eq!(action.attempt, SECOND_ATTEMPT);
    assert_ne!(action.operation_id, transfer(&original).operation_id);
    assert!(action.pin_required);
    assert!(action.preserve_protected_form);
    assert_eq!(action.prior_result_ref, None);
    let current = status(&repaired, &observed.history);
    assert_eq!(current.verified_replicas, CURRENT_SOURCE_COUNT);
    assert_eq!(current.under_replicated, vec![observed.manifest.contents[0].content_ref.clone()]);
    assert_eq!(current.active_operations, vec![action.operation_id.clone()]);
    assert_eq!(observed, snapshot);
}

// r[verify molten.audit_f04.current_availability]
// r[verify molten.audit_f04.fresh_repair]
#[test]
fn absent_and_corrupt_target_observations_require_fresh_verification() {
    for (is_present, is_verified, expected_kind) in
        [(false, true, ActionKind::Transfer), (true, false, ActionKind::Repair)]
    {
        let (mut observed, original) = after_success();
        let mut target = replica(&observed.manifest.contents[0].content_ref, TARGET_PEER, TARGET_DOMAIN);
        target.present = is_present;
        target.identity_verified = is_verified;
        observed.inventory.replicas.push(target);
        let repaired = plan(&observed).expect("contradicted prior success");
        let action = transfer(&repaired);
        assert_eq!(action.kind, expected_kind);
        assert_eq!(action.attempt, SECOND_ATTEMPT);
        assert_ne!(action.operation_id, transfer(&original).operation_id);
        assert_eq!(status(&repaired, &observed.history).verified_replicas, CURRENT_SOURCE_COUNT);
        let mut completed = observed.history.clone();
        completed.push(verified(action));
        let current = status(&repaired, &completed);
        assert_eq!(current.verified_replicas, DEFAULT_REPLICAS);
        assert!(current.under_replicated.is_empty());
        assert!(current.active_operations.is_empty());
    }
}

// r[verify molten.audit_f04.current_availability]
#[test]
fn current_verified_inventory_counts_once_despite_saved_success() {
    let (mut observed, _) = after_success();
    observed
        .inventory
        .replicas
        .push(replica(&observed.manifest.contents[0].content_ref, TARGET_PEER, TARGET_DOMAIN));
    let current = plan(&observed).expect("current verified inventory");
    assert_eq!(current.verified_replicas, DEFAULT_REPLICAS);
    assert!(current.actions.is_empty());
    let readback = status(&current, &observed.history);
    assert_eq!(readback.verified_replicas, DEFAULT_REPLICAS);
    assert!(readback.under_replicated.is_empty());
}

// r[verify molten.audit_f04.fresh_repair]
// r[verify molten.content_replication.resources_failures]
#[test]
fn historical_success_at_attempt_limit_leaves_loss_unresolved() {
    let (mut observed, _) = after_success();
    observed.manifest.repair.max_attempts = FIRST_ATTEMPT;
    let snapshot = observed.clone();
    let exhausted = plan(&observed).expect("exhausted repair plan");
    assert_eq!(exhausted.decision, Decision::Partial);
    assert!(exhausted.issues.contains(&Issue::RepairExhausted));
    assert!(exhausted.actions.iter().all(|action| action.kind == ActionKind::Defer));
    let current = status(&exhausted, &observed.history);
    assert_eq!(current.verified_replicas, CURRENT_SOURCE_COUNT);
    assert_eq!(current.under_replicated, vec![observed.manifest.contents[0].content_ref.clone()]);
    assert_eq!(observed, snapshot);
}

// r[verify molten.audit_f04.fresh_repair]
// r[verify molten.content_replication.planner]
#[test]
fn latest_attempt_wins_across_outcomes_kinds_and_history_order() {
    let (mut observed, _) = after_success();
    let mut target = replica(&observed.manifest.contents[0].content_ref, TARGET_PEER, TARGET_DOMAIN);
    target.identity_verified = false;
    observed.inventory.replicas.push(target);
    let second = plan(&observed).expect("second repair");
    let mut failed = verified(transfer(&second));
    failed.outcome = OperationOutcome::Failed;
    observed.history.push(failed);
    let third = plan(&observed).expect("third attempt");
    assert_eq!(transfer(&third).attempt, THIRD_ATTEMPT);
    observed.history.reverse();
    observed.peers.reverse();
    assert_eq!(plan(&observed).expect("reordered history"), third);
    assert_eq!(status(&third, &observed.history).verified_replicas, CURRENT_SOURCE_COUNT);
}

// r[verify molten.audit_f04.replay_preservation]
#[test]
fn selected_operation_id_collision_rejects_without_input_mutation() {
    for field in 0..5 {
        let (mut observed, _) = after_success();
        let prior = &mut observed.history[0];
        match field {
            0 => prior.content_ref = digest('c'),
            1 => prior.target_peer = "peer-c".to_string(),
            2 => prior.generation = GENERATION.saturating_add(1),
            3 => prior.membership_epoch = MEMBERSHIP_EPOCH.saturating_sub(1),
            _ => prior.placement_epoch = PLACEMENT_EPOCH.saturating_sub(1),
        }
        let snapshot = observed.clone();
        assert_eq!(plan(&observed), Err(Issue::ConflictingOperation));
        assert_eq!(observed, snapshot);
    }
}

// r[verify molten.audit_f04.replay_preservation]
#[test]
fn malformed_and_conflicting_history_deny_without_planned_effects() {
    let (mut malformed, _) = after_success();
    malformed.history[0].operation_id = "not-a-reference".to_string();
    let snapshot = malformed.clone();
    let denied = plan(&malformed).expect("malformed history denial");
    assert_eq!(denied.decision, Decision::Denied);
    assert!(denied.issues.contains(&Issue::InvalidReference));
    assert!(denied.actions.is_empty());
    assert_eq!(malformed, snapshot);

    let (mut conflicting, _) = after_success();
    let mut contradiction = conflicting.history[0].clone();
    contradiction.result_ref = Some(digest('c'));
    conflicting.history.push(contradiction);
    let snapshot = conflicting.clone();
    let denied = plan(&conflicting).expect("conflicting history denial");
    assert_eq!(denied.decision, Decision::Denied);
    assert!(denied.issues.contains(&Issue::ConflictingOperation));
    assert!(denied.actions.is_empty());
    assert_eq!(conflicting, snapshot);
}

// r[verify molten.audit_f04.replay_preservation]
// r[verify molten.content_replication.epoch_fencing]
#[test]
fn genuinely_stale_success_cannot_consume_current_attempts_or_availability() {
    for field in 0..3 {
        let (mut observed, _) = after_success();
        match field {
            0 => {
                observed.manifest.generation = GENERATION.saturating_add(1);
                observed.inventory.replicas[0].generation = observed.manifest.generation;
            }
            1 => {
                observed.manifest.membership_epoch = MEMBERSHIP_EPOCH.saturating_add(1);
                observed.inventory.replicas[0].membership_epoch = observed.manifest.membership_epoch;
                for peer in &mut observed.peers {
                    peer.membership_epoch = observed.manifest.membership_epoch;
                }
            }
            _ => {
                observed.manifest.placement_epoch = PLACEMENT_EPOCH.saturating_add(1);
                observed.inventory.replicas[0].placement_epoch = observed.manifest.placement_epoch;
                for peer in &mut observed.peers {
                    peer.placement_epoch = observed.manifest.placement_epoch;
                }
            }
        }
        let fresh = plan(&observed).expect("stale historical success");
        assert_eq!(transfer(&fresh).attempt, FIRST_ATTEMPT);
        assert_ne!(transfer(&fresh).operation_id, observed.history[0].operation_id);
        let current = status(&fresh, &observed.history);
        assert_eq!(current.verified_replicas, CURRENT_SOURCE_COUNT);
        assert_eq!(current.under_replicated, vec![observed.manifest.contents[0].content_ref.clone()]);
    }
}

// r[verify molten.audit_f04.replay_preservation]
#[test]
fn saved_plan_readback_retains_original_outcome_not_new_success() {
    let (observed, original) = after_success();
    let original_snapshot = original.clone();
    let historical = status(&original, &observed.history);
    assert_eq!(historical.verified_replicas, DEFAULT_REPLICAS);
    assert!(historical.active_operations.is_empty());
    assert!(historical.under_replicated.is_empty());
    assert_eq!(original, original_snapshot);

    let fresh = plan(&observed).expect("new reconciliation");
    assert_ne!(transfer(&fresh).operation_id, transfer(&original).operation_id);
    assert_eq!(status(&fresh, &observed.history).verified_replicas, CURRENT_SOURCE_COUNT);
}
