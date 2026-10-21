use super::*;

fn verified_result(action: &Action) -> PriorOperation {
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

fn no_target() -> ReconcileInput {
    let mut observed = input();
    observed.manifest.policy.minimum_fault_domains = 1;
    observed.peers.truncate(1);
    observed
}

// r[verify molten.audit_f07.deficit_status]
#[test]
fn audit_insufficient_peers_remain_under_replicated() {
    let observed = no_target();
    let planned = plan(&observed).expect("no-target plan");
    assert_eq!(planned.decision, Decision::Partial);
    assert_eq!(planned.issues, vec![Issue::InsufficientPeers]);
    assert!(planned.actions.is_empty());
    let current = status(&planned, &[]);
    assert_eq!(current.under_replicated, vec![observed.manifest.contents[0].content_ref.clone()]);
    assert_eq!(current.verified_replicas, 1);
    assert!(current.active_operations.is_empty());
    assert!(current.failures.is_empty());
}

// r[verify molten.audit_f07.deficit_status]
#[test]
fn satisfied_inventory_without_actions_has_no_deficit() {
    let mut observed = input();
    let content_ref = observed.manifest.contents[0].content_ref.clone();
    observed.inventory.replicas.push(replica(&content_ref, "peer-b", "zone-b"));
    let planned = plan(&observed).expect("satisfied plan");
    assert_eq!(planned.decision, Decision::Ready);
    assert!(planned.actions.is_empty());
    assert!(planned.replica_deficits.is_empty());
    let current = status(&planned, &[]);
    assert_eq!(current.verified_replicas, DEFAULT_REPLICAS);
    assert!(current.under_replicated.is_empty());
}

// r[verify molten.audit_f07.per_content_accounting]
#[test]
fn successful_subset_cannot_clear_more_demand_than_selected_targets() {
    let mut observed = input();
    observed.manifest.policy.desired_replicas = 3;
    observed.manifest.policy.minimum_verified_replicas = 1;
    observed.manifest.policy.minimum_fault_domains = 1;
    observed.peers.truncate(2);
    let planned = plan(&observed).expect("partial-target plan");
    assert_eq!(planned.decision, Decision::Partial);
    assert_eq!(planned.actions.len(), 1);
    let completed = vec![verified_result(&planned.actions[0])];
    let current = status(&planned, &completed);
    assert_eq!(current.verified_replicas, 2);
    assert_eq!(current.under_replicated, vec![observed.manifest.contents[0].content_ref.clone()]);
    assert!(current.active_operations.is_empty());
    assert!(current.failures.is_empty());
}

// r[verify molten.audit_f07.per_content_accounting]
#[test]
fn success_for_another_object_cannot_fill_a_residual_deficit() {
    let mut observed = input();
    observed.manifest.policy.desired_replicas = 3;
    observed.manifest.policy.minimum_verified_replicas = 1;
    observed.manifest.policy.minimum_fault_domains = 1;
    observed.manifest.contents.push(rule('e'));
    let second = observed.manifest.contents[1].content_ref.clone();
    observed.inventory.replicas.extend([
        replica(&second, "peer-a", "zone-a"),
        replica(&second, "peer-b", "zone-b"),
    ]);
    let planned = plan(&observed).expect("two-content plan");
    let first = observed.manifest.contents[0].content_ref.clone();
    let results = planned
        .actions
        .iter()
        .filter(|action| {
            action.content_ref == second || (action.content_ref == first && action.target_peer == "peer-b")
        })
        .map(verified_result)
        .collect::<Vec<_>>();
    let current = status(&planned, &results);
    assert_eq!(current.verified_replicas, 5);
    assert_eq!(current.under_replicated, vec![first]);
    assert_eq!(current.active_operations.len(), 1);
}

// r[verify molten.audit_f07.per_content_accounting]
// r[verify molten.audit_f04.replay_preservation]
#[test]
fn mismatched_or_ambiguous_result_cannot_resolve_demand() {
    let planned = plan(&input()).expect("transfer plan");
    for field in 0..7 {
        let mut result = verified_result(&planned.actions[0]);
        match field {
            0 => result.content_ref = digest('c'),
            1 => result.source_peer = Some("peer-c".to_string()),
            2 => result.target_peer = "peer-c".to_string(),
            3 => result.generation = GENERATION.saturating_add(1),
            4 => result.attempt = result.attempt.saturating_add(1),
            5 => result.result_ref = None,
            _ => result.result_ref = Some("not-a-reference".to_string()),
        }
        let current = status(&planned, &[result]);
        assert_eq!(current.verified_replicas, 1);
        assert_eq!(current.under_replicated, planned.under_replicated);
    }
    let result = verified_result(&planned.actions[0]);
    let current = status(&planned, &[result.clone(), result]);
    assert_eq!(current.verified_replicas, 1);
    assert_eq!(current.under_replicated, planned.under_replicated);
}

// r[verify molten.audit_f07.per_content_accounting]
#[test]
fn bounded_replica_counts_never_clear_unmet_demand() {
    let peers = [
        ("peer-a", "zone-a"),
        ("peer-b", "zone-b"),
        ("peer-c", "zone-c"),
        ("peer-d", "zone-d"),
    ];
    for desired in 1..=peers.len() {
        for current in 1..=desired {
            for available in 1..=peers.len() {
                let mut observed = input();
                observed.manifest.policy.desired_replicas = desired;
                observed.manifest.policy.minimum_verified_replicas = 1;
                observed.manifest.policy.minimum_fault_domains = 1;
                let content_ref = observed.manifest.contents[0].content_ref.clone();
                observed.inventory.replicas =
                    peers[..current].iter().map(|(id, domain)| replica(&content_ref, id, domain)).collect();
                observed.peers = peers[..available].iter().map(|(id, domain)| peer(id, domain)).collect();
                let planned = plan(&observed).expect("bounded plan");
                for successful in 0..=planned.actions.len() {
                    let results = planned.actions[..successful].iter().map(verified_result).collect::<Vec<_>>();
                    let projected = status(&planned, &results);
                    assert_eq!(projected.verified_replicas, current.saturating_add(successful).min(desired));
                    assert_eq!(projected.under_replicated.is_empty(), current.saturating_add(successful) >= desired);
                }
            }
        }
    }
}

// r[verify molten.audit_f07.validation_claims]
#[test]
fn action_free_plan_identity_binds_missing_counts() {
    let mut observed = no_target();
    observed.manifest.policy.desired_replicas = 3;
    let before = plan(&observed).expect("two missing replicas");
    let content_ref = observed.manifest.contents[0].content_ref.clone();
    observed.inventory.replicas.push(replica(&content_ref, "peer-b", "zone-b"));
    let after = plan(&observed).expect("one missing replica");
    assert!(before.actions.is_empty());
    assert!(after.actions.is_empty());
    assert_ne!(before.plan_ref, after.plan_ref);
    assert_eq!(before.replica_deficits[0].missing_replicas, 2);
    assert_eq!(after.replica_deficits[0].missing_replicas, 1);
}
