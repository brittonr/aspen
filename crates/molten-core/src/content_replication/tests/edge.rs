use super::*;

#[test]
fn conflicting_operation_and_repair_exhaustion_fail_closed() {
    let first = plan(&input()).expect("initial plan");
    let transfer = first.actions.first().expect("transfer");
    let mut conflict = input();
    conflict.history.push(PriorOperation {
        operation_id: transfer.operation_id.clone(),
        content_ref: transfer.content_ref.clone(),
        source_peer: transfer.source_peer.clone(),
        target_peer: "peer-c".to_string(),
        generation: GENERATION,
        membership_epoch: MEMBERSHIP_EPOCH,
        placement_epoch: PLACEMENT_EPOCH,
        attempt: 1,
        outcome: OperationOutcome::Verified,
        result_ref: Some(digest('c')),
    });
    assert_eq!(plan(&conflict), Err(Issue::ConflictingOperation));

    let mut exhausted = input();
    let content_ref = exhausted.manifest.contents[0].content_ref.clone();
    let mut target = replica(&content_ref, "peer-b", "zone-b");
    target.identity_verified = false;
    exhausted.inventory.replicas.push(target);
    for attempt in 1..=MAX_REPAIR_ATTEMPTS {
        exhausted.history.push(PriorOperation {
            operation_id: digest(char::from_digit(attempt.saturating_sub(1), HEX_RADIX).expect("hex digit")),
            content_ref: content_ref.clone(),
            source_peer: Some("peer-a".to_string()),
            target_peer: "peer-b".to_string(),
            generation: GENERATION,
            membership_epoch: MEMBERSHIP_EPOCH,
            placement_epoch: PLACEMENT_EPOCH,
            attempt,
            outcome: OperationOutcome::Failed,
            result_ref: None,
        });
    }
    let result = plan(&exhausted).expect("exhausted plan");
    assert!(result.issues.contains(&Issue::RepairExhausted));
    assert!(result.actions.iter().any(|action| action.kind == ActionKind::Defer));
}

#[test]
fn retention_and_cleanup_authority_fence_excess_replicas() {
    let mut excess = input();
    excess.manifest.policy.desired_replicas = 1;
    excess.manifest.policy.minimum_verified_replicas = 1;
    excess.manifest.policy.minimum_fault_domains = 1;
    let content_ref = excess.manifest.contents[0].content_ref.clone();
    excess.inventory.replicas.push(replica(&content_ref, "peer-b", "zone-b"));
    let pinned = plan(&excess).expect("pinned cleanup plan");
    assert!(pinned.issues.contains(&Issue::ActiveRetentionPin));
    assert!(!pinned.actions.iter().any(|action| action.kind == ActionKind::Cleanup));

    let extra = excess.inventory.replicas.last_mut().expect("extra replica");
    extra.pinned = false;
    extra.cleanup_clearance_ref = Some(digest('d'));
    let admitted = plan(&excess).expect("cleanup plan");
    let cleanup = admitted.actions.iter().find(|action| action.kind == ActionKind::Cleanup).expect("cleanup action");
    assert_eq!(cleanup.cleanup_authority_ref, Some(digest('a')));
    assert_eq!(cleanup.prior_result_ref, Some(digest('d')));
}

#[test]
fn protected_form_and_resource_pressure_never_weaken_policy() {
    let mut protected = input();
    protected.inventory.replicas[0].protected = false;
    let denied = plan(&protected).expect("protected plan");
    assert!(denied.issues.contains(&Issue::ProtectedFormMismatch));
    assert!(
        !denied
            .actions
            .iter()
            .any(|action| { matches!(action.kind, ActionKind::Transfer | ActionKind::Repair | ActionKind::Handoff) })
    );

    let mut pressure = input();
    pressure.manifest.contents.push(rule('e'));
    let second_ref = pressure.manifest.contents[1].content_ref.clone();
    pressure.inventory.replicas.push(replica(&second_ref, "peer-a", "zone-a"));
    pressure.manifest.resources.max_transfer_bytes = CONTENT_BYTES;
    let bounded = plan(&pressure).expect("bounded plan");
    assert!(bounded.issues.contains(&Issue::ByteBudgetExhausted));
    assert_eq!(
        bounded
            .actions
            .iter()
            .filter(|action| matches!(action.kind, ActionKind::Transfer | ActionKind::Repair | ActionKind::Handoff))
            .count(),
        1
    );
}

#[test]
fn malformed_manifest_and_status_nonclaims_are_explicit() {
    let mut malformed = input();
    malformed.manifest.ports.pop();
    malformed.manifest.non_claims.pop();
    let denied = plan(&malformed).expect("denied plan");
    assert_eq!(denied.decision, Decision::Denied);
    assert!(denied.issues.contains(&Issue::MissingPort));
    assert!(denied.issues.contains(&Issue::InvalidManifest));

    let healthy = plan(&input()).expect("healthy plan");
    let readback = status(&healthy, &[]);
    assert_eq!(readback.non_claims.len(), NON_CLAIMS.len());
    assert_eq!(readback.placement_epoch, PLACEMENT_EPOCH);
}
