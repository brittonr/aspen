use super::*;

const PEER_A: &str = "peer-a";
const PEER_B: &str = "peer-b";
const PEER_C: &str = "peer-c";
const ZONE_A: &str = "zone-a";
const ZONE_B: &str = "zone-b";
const ZONE_C: &str = "zone-c";
const CLEANUP_CLEARANCE: char = 'd';

fn observed_placements(placements: &[(&str, &str)]) -> ReconcileInput {
    let mut input = input();
    let content_ref = &input.manifest.contents[0].content_ref;
    input.inventory.replicas = placements
        .iter()
        .map(|(id, domain)| {
            let mut replica = replica(content_ref, id, domain);
            replica.pinned = false;
            replica.cleanup_clearance_ref = Some(digest(CLEANUP_CLEARANCE));
            replica
        })
        .collect();
    input.peers = placements.iter().map(|(id, domain)| peer(id, domain)).collect();
    input
}

fn three_peer_input() -> ReconcileInput {
    observed_placements(&[(PEER_A, ZONE_A), (PEER_B, ZONE_A), (PEER_C, ZONE_B)])
}

fn targets(plan: &Plan) -> Vec<&str> {
    let mut targets = plan
        .actions
        .iter()
        .filter(|action| action.kind == ActionKind::Cleanup)
        .map(|action| action.target_peer.as_str())
        .collect::<Vec<_>>();
    targets.sort();
    targets
}

fn assert_residual_policy(input: &ReconcileInput, plan: &Plan) {
    let mut residual = input.inventory.replicas.iter().collect::<Vec<_>>();
    for action in plan.actions.iter().filter(|action| action.kind == ActionKind::Cleanup) {
        residual.retain(|replica| replica.peer_id != action.target_peer);
        let domains = residual
            .iter()
            .map(|replica| replica.fault_domain.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        assert!(residual.len() >= input.manifest.policy.desired_replicas);
        assert!(residual.len() >= input.manifest.policy.minimum_verified_replicas);
        assert!(domains.len() >= input.manifest.policy.minimum_fault_domains);
    }
}

// r[verify molten.audit_f05.residual_policy]
// r[verify molten.audit_f05.validation_claims]
#[test]
fn audit_preserves_required_fault_domains() {
    let input = three_peer_input();
    let result = plan(&input).expect("domain-preserving cleanup plan");
    assert_eq!(targets(&result), vec![PEER_B]);
    assert_eq!(result.decision, Decision::Ready);
    assert!(result.deferred.is_empty());
    assert_residual_policy(&input, &result);
}

// r[verify molten.audit_f05.selection]
#[test]
fn reordered_inventories_preserve_actions_and_plan_reference() {
    let input = three_peer_input();
    let expected = plan(&input).expect("original cleanup plan");
    for rotation in 0..input.inventory.replicas.len() {
        for reverse in [false, true] {
            let mut reordered = input.clone();
            reordered.inventory.replicas.rotate_left(rotation);
            reordered.peers.rotate_left(rotation);
            if reverse {
                reordered.inventory.replicas.reverse();
                reordered.peers.reverse();
            }
            assert_eq!(plan(&reordered).expect("reordered cleanup plan"), expected);
        }
    }
}

// r[verify molten.audit_f05.residual_policy]
// r[verify molten.audit_f05.selection]
#[test]
fn later_removals_use_cumulative_remainder_not_original_domains() {
    let mut input = observed_placements(&[
        (PEER_A, ZONE_A),
        (PEER_B, ZONE_B),
        (PEER_C, ZONE_B),
        ("peer-d", ZONE_C),
        ("peer-e", ZONE_C),
    ]);
    input.manifest.policy.desired_replicas = 3;
    input.manifest.policy.minimum_verified_replicas = 3;
    input.manifest.policy.minimum_fault_domains = 3;
    let result = plan(&input).expect("multiple cleanup plan");
    assert_eq!(targets(&result), vec![PEER_C, "peer-e"]);
    assert_eq!(result.decision, Decision::Ready);
    assert_residual_policy(&input, &result);
}

// r[verify molten.audit_f05.selection]
// r[verify molten.audit_f05.authority_preservation]
#[test]
fn pinned_or_uncleared_first_candidate_does_not_hide_safe_alternative() {
    for has_pin in [false, true] {
        let mut input = three_peer_input();
        input.inventory.replicas[1].pinned = has_pin;
        if !has_pin {
            input.inventory.replicas[1].cleanup_clearance_ref = None;
        }
        let result = plan(&input).expect("alternative cleanup plan");
        assert_eq!(targets(&result), vec![PEER_A]);
        assert_eq!(result.decision, Decision::Ready);
        assert!(result.issues.is_empty());
        assert_residual_policy(&input, &result);
    }
}

// r[verify molten.audit_f05.selection]
#[test]
fn no_safe_candidate_retains_excess_with_one_bounded_deferral() {
    let mut input = three_peer_input();
    input.inventory.replicas[0].pinned = true;
    input.inventory.replicas[1].pinned = true;
    input.manifest.resources.max_diagnostics = 1;
    let result = plan(&input).expect("conservative cleanup plan");
    assert!(targets(&result).is_empty());
    assert_eq!(result.decision, Decision::Partial);
    assert_eq!(result.deferred, vec![input.manifest.contents[0].content_ref.clone()]);
    assert_eq!(result.issues, vec![Issue::InsufficientFaultDomains]);
    assert_eq!(result.actions.iter().filter(|action| action.kind == ActionKind::Defer).count(), 1);
}

// r[verify molten.audit_f05.residual_policy]
#[test]
fn already_unsatisfied_domain_policy_never_authorizes_removal() {
    let input = observed_placements(&[(PEER_A, ZONE_A), (PEER_B, ZONE_A), (PEER_C, ZONE_A)]);
    let result = plan(&input).expect("unsatisfied domain plan");
    assert!(targets(&result).is_empty());
    assert_eq!(result.decision, Decision::Partial);
    assert_eq!(result.issues, vec![Issue::InsufficientFaultDomains]);
}

// r[verify molten.audit_f05.authority_preservation]
#[test]
fn authority_and_disabled_policy_remain_independent_gates() {
    let mut input = three_peer_input();
    input.manifest.contents[0].cleanup_authority_ref = None;
    let missing = plan(&input).expect("missing authority plan");
    assert!(targets(&missing).is_empty());
    assert_eq!(missing.issues, vec![Issue::MissingCleanupAuthority]);

    input.manifest.contents[0].cleanup_authority_ref = Some(digest('a'));
    input.manifest.repair.cleanup_after_handoff = false;
    let disabled = plan(&input).expect("disabled cleanup plan");
    assert!(targets(&disabled).is_empty());
    assert_eq!(disabled.issues, vec![Issue::ActiveRetentionPin]);
}

// r[verify molten.audit_f05.authority_preservation]
#[test]
fn stale_replica_cannot_replace_the_only_current_replica_in_a_required_domain() {
    let mut input = three_peer_input();
    let mut stale = replica(&input.manifest.contents[0].content_ref, "peer-d", ZONE_B);
    stale.placement_epoch = PLACEMENT_EPOCH.saturating_sub(1);
    stale.pinned = false;
    stale.cleanup_clearance_ref = Some(digest(CLEANUP_CLEARANCE));
    input.inventory.replicas.push(stale);
    input.peers.push(peer("peer-d", ZONE_B));
    let result = plan(&input).expect("stale alternative cleanup plan");
    assert_eq!(targets(&result), vec![PEER_B]);
}

// r[verify molten.audit_f05.selection]
// r[verify molten.content_replication.resources_failures]
#[test]
fn removals_and_unresolved_deferral_respect_queue_capacity() {
    let mut input = observed_placements(&[(PEER_A, ZONE_A), (PEER_B, ZONE_A), (PEER_C, ZONE_B), ("peer-d", ZONE_B)]);
    input.manifest.resources.max_queue_depth = 1;
    assert_eq!(plan(&input), Err(Issue::QueueExhausted));

    input.inventory.replicas[0].pinned = true;
    input.inventory.replicas[1].pinned = true;
    input.inventory.replicas[2].pinned = true;
    assert_eq!(plan(&input), Err(Issue::QueueExhausted));
}
