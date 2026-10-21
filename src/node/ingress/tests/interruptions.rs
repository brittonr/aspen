use super::*;
// r[verify molten.audit_f03.uncertainty]
// r[verify molten.audit_f03.no_blind_replay]
// r[verify molten.audit_f03.preserve_state]
#[test]
fn before_result_publication_blocks_replay_of_executed_status_effect() {
    let case = Case::new("ingress_before_result");
    case.start_fixture();
    case.deliver();
    let reservation = case.dedup();
    case.set_phase(crate::core_api::node_ingress::PublicationPhase::Dispatching);
    // Execute the real status operation, stopping before its outbox result publication.
    let status = status_local_node_with_request(&case.reopen(), &case.envelope.request).expect("actual status effect");
    assert_eq!(status.status, "running");
    assert!(
        case.read(&control_outbox_receipt_path(&case.envelope.request.request_ref).expect("result path"))
            .is_none()
    );
    let before = case.facts();
    assert_eq!(case.assert_denied().phase, "outcome-unknown");
    assert!(case.dispatch().is_err());
    assert_eq!(case.facts(), before);
    case.assert_dedup_preserved(&reservation);
}

// r[verify molten.audit_f03.no_blind_replay]
#[test]
fn observes_result_before_dispatch_receipt_without_executing_again() {
    let case = Case::new("ingress_before_dispatch_receipt");
    case.start_fixture();
    case.deliver();
    case.set_phase(crate::core_api::node_ingress::PublicationPhase::Dispatching);
    let dispatched =
        dispatch_status_request(&case.reopen(), &case.envelope.request).expect("actual result publication");
    assert!(
        case.read(&dispatch_receipt_path(&case.envelope.request.request_ref).expect("dispatch path"))
            .is_none()
    );
    let before = case.facts();
    let delivered = case.deliver();
    let receipt = parse_control_ingress_receipt(&delivered.ingress_receipt_value).expect("observed result receipt");
    assert_eq!(receipt.decision, "pass");
    assert_eq!(receipt.phase, "completed-dispatch");
    assert!(!delivered.has_enqueued);
    assert_eq!(case.facts(), before);
    let control =
        crate::node_runtime::parse_control_receipt(&dispatched.control_receipt_value).expect("control result");
    assert_eq!(control.request_ref, case.envelope.request.request_ref);
}

// r[verify molten.audit_f03.uncertainty]
// r[verify molten.audit_f03.preserve_state]
#[test]
fn ambiguous_publication_phase_with_absent_inbox_never_replays() {
    let case = Case::new("ingress_ambiguous_publication");
    let reservation = case.reserve();
    case.set_phase(crate::core_api::node_ingress::PublicationPhase::Publishing);
    let before = case.facts();
    assert_eq!(case.assert_denied().phase, "outcome-unknown");
    assert_eq!(case.facts(), before);
    case.assert_no_pending();
    case.assert_dedup_preserved(&reservation);
}

// r[verify molten.audit_f03.uncertainty]
// r[verify molten.audit_f03.preserve_state]
#[test]
fn unknown_intent_phase_latches_durable_quarantine() {
    let case = Case::new("ingress_unknown_phase");
    let reservation = case.reserve();
    let metadata = crate::preserves_rail::record("node-ingress-publication-v1", vec![
        crate::preserves_rail::string("molten.node.ingress-publication.v1"),
        crate::preserves_rail::record("phase", vec![crate::preserves_rail::string("unknown-commit")]),
    ]);
    case.set_metadata(&metadata);
    case.write_inbox();
    let before = case.facts();
    case.assert_denied();
    let root = case.reopen();
    let delivery = root.delivery_store().expect("delivery store");
    let store = crate::delivery_idempotency::PublicationStore::open(&delivery).expect("reopened quarantine");
    assert!(store.require_healthy().is_err());
    drop(store);
    case.assert_denied();
    assert_eq!(case.facts(), before);
    case.assert_dedup_preserved(&reservation);
}

// r[verify molten.audit_f03.uncertainty]
// r[verify molten.audit_f03.no_blind_replay]
// r[verify molten.audit_f03.preserve_state]
#[test]
fn durable_quarantine_blocks_enqueue_and_control_dispatch() {
    let case = Case::new("ingress_durable_quarantine");
    case.start_fixture();
    case.deliver();
    let reservation = case.dedup();
    {
        let root = case.reopen();
        let delivery = root.delivery_store().expect("delivery store");
        let mut store = crate::delivery_idempotency::PublicationStore::open(&delivery).expect("journal");
        store.quarantine().expect("persist ambiguous commit quarantine");
    }
    let before = case.facts();
    case.assert_denied();
    assert!(case.dispatch().is_err());
    case.assert_denied();
    assert_eq!(case.facts(), before);
    case.assert_dedup_preserved(&reservation);
}

// r[verify molten.audit_f03.uncertainty]
// r[verify molten.audit_f03.preserve_state]
#[test]
fn unavailable_observation_is_not_absence_or_permission_to_enqueue() {
    let case = Case::new("ingress_unavailable_observation");
    let reservation = case.reserve();
    let queue_path = queue_receipt_path(&case.envelope.request.request_ref).expect("queue path");
    case.reopen().create_dir_all(&queue_path).expect("unreadable non-file observation fixture");
    case.assert_denied();
    assert_eq!(
        case.reopen().entry_kind(&queue_path).expect("preserved observation"),
        Some(crate::node_state::EntryKind::Directory)
    );
    case.assert_no_pending();
    assert!(
        case.read(&control_outbox_receipt_path(&case.envelope.request.request_ref).expect("result path"))
            .is_none()
    );
    case.assert_dedup_preserved(&reservation);
    let other = status_request().expect("independent request");
    let submitted = super::super::submit(&case.reopen(), &other.value).expect("unaffected submission");
    let receipt = parse_control_queue_receipt(&submitted.queue_receipt_value).expect("independent queue receipt");
    assert_eq!(receipt.decision, "pass");
    assert_eq!(receipt.request_ref, other.request_ref);
}

// r[verify molten.audit_f03.uncertainty]
// r[verify molten.audit_f03.preserve_state]
#[test]
fn existing_empty_store_is_not_reinitialized_as_fresh_history() {
    let case = Case::new("ingress_empty_store");
    let root = case.reopen();
    let delivery = root.delivery_store().expect("delivery store");
    let path = crate::local_store::Locator::parse("delivery-idempotency.redb").expect("database member");
    drop(delivery.root().create_database_file(&path).expect("controlled empty existing database"));
    assert_eq!(case.assert_denied().phase, "outcome-unknown");
    let file = delivery.root().open_database_file(&path).expect("preserved database member");
    assert_eq!(file.metadata().expect("database metadata").len(), 0);
    case.assert_no_pending();
}
