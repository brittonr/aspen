use super::*;
// r[verify molten.audit_f03.recover_publication]
// r[verify molten.audit_f03.preserve_state]
// r[verify molten.audit_f03.validation]
#[test]
fn reopens_before_dedup_and_suppresses_exact_publication_duplicate() {
    let case = Case::new("ingress_before_dedup");
    case.assert_no_pending();
    let first = case.deliver();
    assert!(first.has_enqueued);
    let receipt = parse_control_ingress_receipt(&first.ingress_receipt_value).expect("fresh receipt");
    assert_eq!(receipt.decision, "pass");
    assert_eq!(receipt.phase, "deliver");
    assert_eq!(case.queue().request_ref, case.envelope.request.request_ref);
    let facts = case.facts();
    let duplicate = case.deliver();
    assert!(!duplicate.has_enqueued);
    assert_eq!(duplicate.queue_receipt_ref, first.queue_receipt_ref);
    assert_eq!(
        parse_control_ingress_receipt(&duplicate.ingress_receipt_value).expect("duplicate receipt").decision,
        "pass"
    );
    assert_eq!(case.facts(), facts);
}

// r[verify molten.audit_f03.recover_publication]
// r[verify molten.audit_f03.validation]
#[test]
fn recovers_reopened_atomic_dedup_intent_before_inbox() {
    let case = Case::new("ingress_after_dedup");
    let reservation = case.reserve();
    case.assert_no_pending();
    let delivered = case.deliver();
    let receipt = parse_control_ingress_receipt(&delivered.ingress_receipt_value).expect("recovery receipt");
    assert_eq!(receipt.decision, "pass");
    assert_eq!(receipt.phase, "recovered-publication");
    assert!(delivered.has_enqueued);
    assert_eq!(case.queue().request_ref, case.envelope.request.request_ref);
    let root = case.reopen();
    let inbox = read_preserves(&root, &control_inbox_path(&case.envelope.request.request_ref).expect("path"))
        .expect("recovered inbox");
    assert_eq!(inbox, case.envelope.request.value);
    assert!(
        case.read(&control_outbox_receipt_path(&case.envelope.request.request_ref).expect("result path"))
            .is_none()
    );
    case.assert_dedup_preserved(&reservation);
}

// r[verify molten.audit_f03.no_blind_replay]
// r[verify molten.audit_f03.preserve_state]
#[test]
fn repairs_receipt_after_reopened_inbox_without_reenqueue_or_dispatch() {
    let case = Case::new("ingress_after_inbox");
    let reservation = case.reserve();
    case.set_phase(crate::core_api::node_ingress::PublicationPhase::Publishing);
    case.write_inbox();
    let path = control_inbox_path(&case.envelope.request.request_ref).expect("inbox path");
    let before = case.read(&path);
    let delivered = case.deliver();
    let receipt = parse_control_ingress_receipt(&delivered.ingress_receipt_value).expect("observed receipt");
    assert_eq!(receipt.decision, "pass");
    assert_eq!(receipt.phase, "publication-observed");
    assert!(!delivered.has_enqueued);
    assert_eq!(case.read(&path), before);
    assert_eq!(case.queue().phase, "publication-observed");
    assert_eq!(case.queue().request_ref, case.envelope.request.request_ref);
    assert!(
        case.read(&control_outbox_receipt_path(&case.envelope.request.request_ref).expect("result path"))
            .is_none()
    );
    case.assert_dedup_preserved(&reservation);
}

// r[verify molten.audit_f03.no_blind_replay]
// r[verify molten.audit_f03.preserve_state]
#[test]
fn repeated_completed_dispatch_observation_preserves_immutable_result() {
    let case = Case::new("ingress_after_dispatch");
    case.start_fixture();
    case.deliver();
    let dispatched = case.dispatch().expect("actual status dispatch");
    assert_eq!(dispatched.request_ref, case.envelope.request.request_ref);
    let root = case.reopen();
    root.remove_regular_file(&queue_receipt_path(&case.envelope.request.request_ref).expect("queue path"))
        .expect("controlled missing original queue receipt");
    let before = case.facts();
    let first = case.deliver();
    let second = case.deliver();
    for delivered in [&first, &second] {
        let receipt = parse_control_ingress_receipt(&delivered.ingress_receipt_value).expect("completed receipt");
        assert_eq!(receipt.decision, "pass");
        assert_eq!(receipt.phase, "completed-dispatch");
        assert!(!delivered.has_enqueued);
        let value =
            read_ledger_artifact(&case.reopen(), delivered.queue_receipt_ref.as_deref().expect("observation ref"))
                .expect("observed dispatch artifact");
        let queue = parse_control_queue_receipt(&value).expect("dispatch observation");
        assert_eq!(queue.phase, "dispatch-observed");
        assert_eq!(queue.location_ref, dispatched.control_receipt_ref);
        assert_eq!(queue.request_ref, case.envelope.request.request_ref);
    }
    assert_eq!(first.queue_receipt_ref, second.queue_receipt_ref);
    assert_eq!(case.facts(), before);
    case.assert_no_pending();
    assert!(case.dispatch().is_err());
    assert_eq!(case.facts(), before);
}

// r[verify molten.audit_f03.recover_publication]
// r[verify molten.audit_f03.no_blind_replay]
#[test]
fn concurrent_submissions_serialize_without_losing_publication() {
    let case = Case::new("ingress_concurrent_submission");
    let root = case.reopen();
    let barrier = std::sync::Barrier::new(8);
    let queue_ref = std::thread::scope(|scope| {
        let handles: [_; 8] = std::array::from_fn(|_| {
            scope.spawn(|| {
                barrier.wait();
                super::super::submit(&root, &case.envelope.request.value)
            })
        });
        let mut queue_ref = None;
        for handle in handles {
            let submitted = handle.join().expect("submission thread").expect("serialized submission");
            assert_eq!(submitted.request_ref, case.envelope.request.request_ref);
            let receipt = parse_control_queue_receipt(&submitted.queue_receipt_value).expect("queue receipt");
            assert_eq!(receipt.decision, "pass");
            if let Some(prior) = &queue_ref {
                assert_eq!(&submitted.queue_receipt_ref, prior);
            } else {
                queue_ref = Some(submitted.queue_receipt_ref);
            }
        }
        queue_ref.expect("canonical publication reference")
    });
    assert_eq!(case.queue().receipt_ref, queue_ref);
    let path = control_inbox_path(&case.envelope.request.request_ref).expect("inbox path");
    assert_eq!(read_preserves(&root, &path).expect("published request"), case.envelope.request.value);
}
