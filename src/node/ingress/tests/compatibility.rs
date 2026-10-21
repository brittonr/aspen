use super::*;
// r[verify molten.audit_f03.recover_publication]
// r[verify molten.audit_f03.no_blind_replay]
#[test]
fn legacy_dedup_without_publication_is_unresolved_not_successful() {
    let case = Case::new("ingress_legacy_unresolved");
    let reservation = case.dedup();
    assert!(reservation.should_commit_side_effect);
    let before = case.facts();
    assert_eq!(case.assert_denied().phase, "outcome-unknown");
    assert_eq!(case.facts(), before);
    case.assert_no_pending();
    case.assert_dedup_preserved(&reservation);
}

// r[verify molten.audit_f03.no_blind_replay]
#[test]
fn legacy_dedup_with_exact_inbox_repairs_observation_without_fabricated_intent() {
    let case = Case::new("ingress_legacy_inbox");
    let reservation = case.dedup();
    case.write_inbox();
    let before = case.read(&control_inbox_path(&case.envelope.request.request_ref).expect("inbox path"));
    let delivered = case.deliver();
    assert!(!delivered.has_enqueued);
    assert_eq!(
        parse_control_ingress_receipt(&delivered.ingress_receipt_value).expect("observed receipt").decision,
        "pass"
    );
    assert_eq!(case.queue().phase, "publication-observed");
    assert_eq!(case.read(&control_inbox_path(&case.envelope.request.request_ref).expect("inbox path")), before);
    let root = case.reopen();
    let delivery = root.delivery_store().expect("delivery store");
    let mut store = crate::delivery_idempotency::PublicationStore::open(&delivery).expect("journal");
    assert!(
        store
            .read(&case.envelope.operation_ref, &case.envelope.request.request_ref)
            .expect("read legacy intent")
            .is_none()
    );
    drop(store);
    case.assert_dedup_preserved(&reservation);
}

// r[verify molten.audit_f03.uncertainty]
// r[verify molten.audit_f03.no_blind_replay]
#[test]
fn legacy_dedup_with_queue_only_does_not_infer_absent_dispatch() {
    let case = Case::new("ingress_legacy_queue_only");
    let reservation = case.dedup();
    super::super::effects::publish_queue(&case.reopen(), &case.envelope.request, "enqueue")
        .expect("controlled queue observation without inbox or intent");
    let before = case.facts();
    assert_eq!(case.assert_denied().phase, "outcome-unknown");
    case.assert_no_pending();
    assert_eq!(case.facts(), before);
    case.assert_dedup_preserved(&reservation);
}

// r[verify molten.audit_f03.no_blind_replay]
#[test]
fn legacy_dedup_with_exact_result_suppresses_reenqueue_without_intent() {
    let case = Case::new("ingress_legacy_dispatched");
    case.start_fixture();
    let reservation = case.dedup();
    case.write_inbox();
    case.deliver();
    let dispatched = case.dispatch().expect("actual legacy dispatch");
    case.reopen()
        .remove_regular_file(&queue_receipt_path(&case.envelope.request.request_ref).expect("queue path"))
        .expect("controlled original receipt loss");
    let before = case.facts();
    let delivered = case.deliver();
    let receipt = parse_control_ingress_receipt(&delivered.ingress_receipt_value).expect("legacy result observation");
    assert_eq!(receipt.decision, "pass");
    assert_eq!(receipt.phase, "completed-dispatch");
    assert!(!delivered.has_enqueued);
    let value = read_ledger_artifact(&case.reopen(), delivered.queue_receipt_ref.as_deref().expect("observation ref"))
        .expect("dispatch observation artifact");
    assert_eq!(
        parse_control_queue_receipt(&value).expect("queue observation").location_ref,
        dispatched.control_receipt_ref
    );
    case.assert_no_pending();
    assert_eq!(case.facts(), before);
    case.assert_dedup_preserved(&reservation);
}
