use super::*;

// r[verify molten.audit_f05.authority_preservation]
// r[verify molten.audit_f05.selection]
#[test]
fn pinned_redundant_replicas_and_missing_authority_retain_excess() {
    for has_authority in [false, true] {
        let mut fixture = CleanupFixture::new();
        if has_authority {
            fixture.content.inventory.replicas[0].pinned = true;
            fixture.content.inventory.replicas[1].pinned = true;
        } else {
            fixture.instance.manifest.contents[0].cleanup_authority_ref = None;
        }
        let inventory = fixture.content.inventory.clone();
        let retention = fixture.retention_state();
        let history = fixture.effects.durable.history.clone();
        let outcome = fixture.run().expect("deferred cleanup");
        assert_eq!(outcome.plan.decision, Decision::Partial);
        assert_eq!(outcome.receipt.decision, ReceiptDecision::Partial);
        assert_eq!(fixture.content.inventory, inventory);
        assert_eq!(fixture.retention_state(), retention);
        fixture.assert_no_operation_success(&history);
        assert!(!fixture.events.borrow().contains(&"authorize-cleanup"));
        assert!(!fixture.events.borrow().contains(&"cleanup"));
    }
}

// r[verify molten.audit_f05.authority_preservation]
#[test]
fn runtime_pin_and_stale_clearance_reject_before_content_cleanup() {
    for has_active_pin in [false, true] {
        let mut fixture = CleanupFixture::new();
        fixture.retention.has_active_pin = has_active_pin;
        if !has_active_pin {
            fixture.retention.clearance_ref = digest('c');
        }
        fixture.assert_rejected_before_deletion();
        assert!(fixture.events.borrow().contains(&"authorize-cleanup"));
        assert!(!fixture.events.borrow().contains(&"cleanup"));
    }
}

// r[verify molten.audit_f05.authority_preservation]
#[test]
fn denied_stale_malformed_or_failed_retention_admission_has_no_deletion() {
    for response in [
        RetentionResponse::Denied,
        RetentionResponse::StaleGeneration,
        RetentionResponse::Malformed,
        RetentionResponse::AdapterError,
    ] {
        let mut fixture = CleanupFixture::new();
        fixture.retention.response = response;
        fixture.assert_rejected_before_deletion();
        assert!(fixture.events.borrow().contains(&"authorize-cleanup"));
        assert!(!fixture.events.borrow().contains(&"cleanup"));
    }
}

// r[verify molten.audit_f05.authority_preservation]
#[test]
fn authority_and_membership_or_placement_epoch_rejection_precedes_cleanup_admission() {
    for rejected_gate in ["authority", "membership", "placement"] {
        let mut fixture = CleanupFixture::new();
        match rejected_gate {
            "authority" => fixture.facts.authority.admitted = false,
            "membership" => fixture.facts.membership.current = false,
            "placement" => fixture.facts.placement.placement_epoch = PLACEMENT_EPOCH.saturating_add(1),
            _ => unreachable!("closed gate fixture"),
        }
        fixture.assert_rejected_before_deletion();
        assert!(!fixture.events.borrow().contains(&"authorize-cleanup"));
        assert!(!fixture.events.borrow().contains(&"cleanup"));
    }
}

// r[verify molten.audit_f05.authority_preservation]
#[test]
fn content_adapter_denial_preserves_inventory_retention_and_durable_history() {
    let mut fixture = CleanupFixture::new();
    fixture.content.is_cleanup_denied = true;
    fixture.assert_rejected_before_deletion();
    assert!(fixture.events.borrow().contains(&"authorize-cleanup"));
    assert!(fixture.events.borrow().contains(&"cleanup"));
}

// r[verify molten.audit_f05.validation_claims]
#[test]
fn malformed_posteffect_cleanup_result_is_not_a_durable_success_or_a_rollback_claim() {
    let mut fixture = CleanupFixture::new();
    fixture.content.is_result_malformed = true;
    let retention = fixture.retention_state();
    let history = fixture.effects.durable.history.clone();
    assert!(fixture.run().is_err());
    let peers = fixture
        .content
        .inventory
        .replicas
        .iter()
        .map(|replica| replica.peer_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(peers, vec![PEER_A, PEER_C]);
    assert_eq!(fixture.retention_state(), retention);
    fixture.assert_no_operation_success(&history);
    assert_eq!(fixture.effects.durable.status_count, 0);
    assert_eq!(fixture.effects.receipts.count, 0);
}
