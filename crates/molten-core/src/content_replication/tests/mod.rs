mod edge;

use super::*;

const DIGEST_HEX_LENGTH: usize = 64;
const GENERATION: u64 = 1;
const MEMBERSHIP_EPOCH: u64 = 2;
const PLACEMENT_EPOCH: u64 = 3;
const CONTENT_BYTES: u64 = 64;
const DEFAULT_REPLICAS: usize = 2;
const DEFAULT_DOMAINS: usize = 2;
const TRANSFER_LIMIT: usize = 4;
const TRANSFER_BYTE_LIMIT: u64 = 256;
const QUEUE_LIMIT: usize = 16;
const TIMER_LIMIT: usize = 4;
const DIAGNOSTIC_LIMIT: usize = 16;
const HEX_RADIX: u32 = 16;

fn digest(byte: char) -> String {
    format!("blake3:{}", byte.to_string().repeat(DIGEST_HEX_LENGTH))
}

fn manifest() -> Manifest {
    Manifest {
        service_id: "content-replication-fixture".to_string(),
        generation: GENERATION,
        membership_epoch: MEMBERSHIP_EPOCH,
        placement_epoch: PLACEMENT_EPOCH,
        authority_ref: digest('1'),
        identity_ref: digest('2'),
        content_profile_ref: digest('3'),
        transport_profile_ref: digest('4'),
        retention_policy_ref: digest('5'),
        evidence_profile_ref: digest('6'),
        ports: REQUIRED_PORTS.iter().map(ToString::to_string).collect(),
        policy: ReplicaPolicy {
            desired_replicas: DEFAULT_REPLICAS,
            minimum_verified_replicas: DEFAULT_REPLICAS,
            minimum_fault_domains: DEFAULT_DOMAINS,
        },
        repair: RepairPolicy {
            max_attempts: MAX_REPAIR_ATTEMPTS,
            allow_handoff: true,
            cleanup_after_handoff: true,
        },
        resources: ResourceLimits {
            max_concurrent_transfers: TRANSFER_LIMIT,
            max_transfer_bytes: TRANSFER_BYTE_LIMIT,
            max_queue_depth: QUEUE_LIMIT,
            max_timers: TIMER_LIMIT,
            max_diagnostics: DIAGNOSTIC_LIMIT,
        },
        contents: vec![rule('7')],
        non_claims: NON_CLAIMS.iter().map(ToString::to_string).collect(),
    }
}

fn rule(byte: char) -> ReplicaRule {
    ReplicaRule {
        content_ref: digest(byte),
        manifest_ref: digest('8'),
        encoded_bytes: CONTENT_BYTES,
        protected: true,
        transform_ref: Some(digest('9')),
        cleanup_authority_ref: Some(digest('a')),
    }
}

fn peer(id: &str, domain: &str) -> Peer {
    Peer {
        peer_id: id.to_string(),
        fault_domain: domain.to_string(),
        membership_epoch: MEMBERSHIP_EPOCH,
        placement_epoch: PLACEMENT_EPOCH,
        available: true,
        capacity_bytes: TRANSFER_BYTE_LIMIT,
    }
}

fn replica(content_ref: &str, peer_id: &str, domain: &str) -> Replica {
    Replica {
        content_ref: content_ref.to_string(),
        peer_id: peer_id.to_string(),
        fault_domain: domain.to_string(),
        generation: GENERATION,
        membership_epoch: MEMBERSHIP_EPOCH,
        placement_epoch: PLACEMENT_EPOCH,
        present: true,
        identity_verified: true,
        pinned: true,
        protected: true,
        manifest_ref: digest('8'),
        cleanup_clearance_ref: None,
    }
}

fn input() -> ReconcileInput {
    let manifest = manifest();
    let content_ref = manifest.contents[0].content_ref.clone();
    ReconcileInput {
        manifest,
        inventory: Inventory {
            replicas: vec![replica(&content_ref, "peer-a", "zone-a")],
        },
        peers: vec![
            peer("peer-a", "zone-a"),
            peer("peer-b", "zone-b"),
            peer("peer-c", "zone-c"),
        ],
        history: Vec::new(),
        observed_tick: 1,
    }
}

#[test]
fn stable_placement_is_receiver_driven_and_fault_domain_aware() {
    let first = plan(&input()).expect("first plan");
    let mut reordered = input();
    reordered.peers.reverse();
    reordered.inventory.replicas.reverse();
    let second = plan(&reordered).expect("reordered plan");
    assert_eq!(first, second);
    assert_eq!(first.decision, Decision::Ready);
    assert_eq!(first.actions.len(), 1);
    let action = &first.actions[0];
    assert_eq!(action.kind, ActionKind::Transfer);
    assert_eq!(action.source_peer.as_deref(), Some("peer-a"));
    assert_eq!(action.target_peer, "peer-b");
    assert!(action.pin_required);
    assert!(action.preserve_protected_form);
}

#[test]
fn insufficient_peers_and_fault_domains_defer_without_ambient_targets() {
    let mut insufficient = input();
    insufficient.peers.truncate(1);
    let result = plan(&insufficient).expect("partial plan");
    assert_eq!(result.decision, Decision::Partial);
    assert!(result.issues.contains(&Issue::InsufficientPeers));
    assert!(result.actions.iter().all(|action| action.kind == ActionKind::Defer));
    assert!(result.actions.iter().all(|action| action.target_peer == "unassigned"));

    let mut domains = input();
    domains.manifest.policy.desired_replicas = 3;
    domains.manifest.policy.minimum_verified_replicas = 3;
    domains.manifest.policy.minimum_fault_domains = 3;
    domains.peers[1].fault_domain = "zone-a".to_string();
    domains.peers[2].fault_domain = "zone-a".to_string();
    let result = plan(&domains).expect("domain plan");
    assert!(result.issues.contains(&Issue::InsufficientFaultDomains));
}

#[test]
fn stale_epochs_never_satisfy_current_targets() {
    let mut stale = input();
    stale.inventory.replicas[0].placement_epoch = PLACEMENT_EPOCH.saturating_sub(1);
    let result = plan(&stale).expect("stale plan");
    assert_eq!(result.verified_replicas, 0);
    assert!(result.under_replicated.contains(&stale.manifest.contents[0].content_ref));
    assert!(result.actions.iter().any(|action| {
        matches!(action.kind, ActionKind::Transfer | ActionKind::Handoff) && action.target_peer != "peer-a"
    }));
}

#[test]
fn stale_target_handoffs_without_authorizing_stale_cleanup() {
    let mut handoff = input();
    let content_ref = handoff.manifest.contents[0].content_ref.clone();
    let mut stale_target = replica(&content_ref, "peer-b", "zone-b");
    stale_target.placement_epoch = PLACEMENT_EPOCH.saturating_sub(1);
    stale_target.pinned = false;
    stale_target.cleanup_clearance_ref = Some(digest('d'));
    handoff.inventory.replicas.push(stale_target);
    let result = plan(&handoff).expect("handoff plan");
    assert!(result.actions.iter().any(|action| action.kind == ActionKind::Handoff));
    assert!(!result.actions.iter().any(|action| action.kind == ActionKind::Cleanup));

    handoff.manifest.policy.desired_replicas = 1;
    handoff.manifest.policy.minimum_verified_replicas = 1;
    handoff.manifest.policy.minimum_fault_domains = 1;
    let stable = plan(&handoff).expect("stale cleanup plan");
    assert!(!stable.actions.iter().any(|action| action.kind == ActionKind::Cleanup));
}

#[test]
fn corrupt_replica_repairs_and_exact_terminal_operation_reuses() {
    let mut corrupt = input();
    let content_ref = corrupt.manifest.contents[0].content_ref.clone();
    let mut target = replica(&content_ref, "peer-b", "zone-b");
    target.identity_verified = false;
    corrupt.inventory.replicas.push(target);
    let first = plan(&corrupt).expect("repair plan");
    let repair = first.actions.iter().find(|action| action.kind == ActionKind::Repair).expect("repair action");
    corrupt.history.push(PriorOperation {
        operation_id: repair.operation_id.clone(),
        content_ref: repair.content_ref.clone(),
        source_peer: repair.source_peer.clone(),
        target_peer: repair.target_peer.clone(),
        generation: GENERATION,
        membership_epoch: MEMBERSHIP_EPOCH,
        placement_epoch: PLACEMENT_EPOCH,
        attempt: 1,
        outcome: OperationOutcome::Verified,
        result_ref: Some(digest('b')),
    });
    let repeated = plan(&corrupt).expect("reused plan");
    assert!(repeated.actions.iter().any(|action| action.kind == ActionKind::Reuse));
}
