use super::*;
fn conflicting_queue_case(label: &str, has_wrong_request: bool) {
    let case = Case::new(label);
    case.start_fixture();
    case.deliver();
    let reservation = case.dedup();
    let wrong_request = local_ref("conflicting-control-request", "other").expect("wrong request ref");
    let location = local_ref(
        "node-control-inbox-path",
        &control_inbox_path(&case.envelope.request.request_ref).expect("inbox path").display(),
    )
    .expect("location ref");
    let value = queue_receipt_value(&QueueReceiptValueInput {
        decision: "pass",
        phase: "enqueue",
        operation: if has_wrong_request { "status" } else { "shutdown" },
        request_ref: if has_wrong_request {
            &wrong_request
        } else {
            &case.envelope.request.request_ref
        },
        location_ref: &location,
        diagnostics: &[],
    })
    .expect("controlled conflicting queue evidence");
    write_preserves(
        &case.reopen(),
        &queue_receipt_path(&case.envelope.request.request_ref).expect("queue path"),
        &value,
    )
    .expect("persist conflict");
    let before = case.facts();
    assert_eq!(case.assert_denied().phase, "publication-conflict");
    assert!(case.dispatch().is_err());
    assert_eq!(case.facts(), before);
    case.assert_dedup_preserved(&reservation);
}

// r[verify molten.audit_f03.preserve_state]
#[test]
fn conflicting_queue_request_ref_preserves_dedup_inbox_and_evidence() {
    conflicting_queue_case("ingress_queue_request_conflict", true);
}

// r[verify molten.audit_f03.preserve_state]
#[test]
fn conflicting_queue_operation_preserves_dedup_inbox_and_evidence() {
    conflicting_queue_case("ingress_queue_operation_conflict", false);
}

// r[verify molten.audit_f03.preserve_state]
#[test]
fn conflicting_inbox_payload_is_not_overwritten_by_recovery() {
    let case = Case::new("ingress_inbox_payload_conflict");
    let reservation = case.reserve();
    let wrong = status_request().expect("different canonical status request");
    assert_ne!(wrong.request_ref, case.envelope.request.request_ref);
    write_preserves(
        &case.reopen(),
        &control_inbox_path(&case.envelope.request.request_ref).expect("inbox path"),
        &wrong.value,
    )
    .expect("persist conflicting payload");
    let before = case.facts();
    assert_eq!(case.assert_denied().phase, "publication-conflict");
    assert_eq!(case.facts(), before);
    case.assert_dedup_preserved(&reservation);
}

// r[verify molten.audit_f03.preserve_state]
#[test]
fn conflicting_control_result_request_ref_preserves_historical_dispatch() {
    let case = Case::new("ingress_control_result_conflict");
    case.start_fixture();
    case.deliver();
    case.dispatch().expect("actual dispatch before conflicting observation");
    let reservation = case.dedup();
    let wrong = status_request().expect("different canonical request");
    assert_ne!(wrong.request_ref, case.envelope.request.request_ref);
    let wrong_status = status_local_node_with_request(&case.reopen(), &wrong).expect("actual other request result");
    let wrong_control =
        crate::node_runtime::parse_control_receipt(&wrong_status.control_receipt_value).expect("other control result");
    assert_eq!(wrong_control.request_ref, wrong.request_ref);
    write_preserves(
        &case.reopen(),
        &control_outbox_receipt_path(&case.envelope.request.request_ref).expect("result path"),
        &wrong_status.control_receipt_value,
    )
    .expect("persist conflicting result");
    let before = case.facts();
    assert_eq!(case.assert_denied().phase, "publication-conflict");
    case.assert_no_pending();
    assert!(case.dispatch().is_err());
    assert_eq!(case.facts(), before);
    case.assert_dedup_preserved(&reservation);
}

// r[verify molten.audit_f03.uncertainty]
// r[verify molten.audit_f03.preserve_state]
#[test]
fn rejected_request_before_reservation_does_not_quarantine_other_delivery() {
    let case = Case::new("ingress_admission_isolation");
    let excessive_refs: Vec<_> = (0..4_097).map(|index| format!("blake3:{index:064x}")).collect();
    let envelope = control_ingress_envelope(&ControlIngressEnvelopeInput {
        request_value: &case.envelope.request.value,
        from_peer: &case.envelope.from_peer,
        to_node: &case.envelope.to_node,
        topic: &case.envelope.topic,
        sequence: case.envelope.sequence,
        peer_bootstrap_refs: &excessive_refs,
        authority_refs: &case.envelope.authority_refs,
        policy_refs: &case.envelope.policy_refs,
        resource_refs: &case.envelope.resource_refs,
        evidence_refs: &[],
    })
    .expect("canonical envelope exceeds only delivery evidence admission");
    publish_control_ingress(&ControlIngressPublishInput {
        state_root: &case.workspace,
        envelope_value: &envelope.value,
    })
    .expect("persist rejected envelope");
    let rejected = deliver_control_ingress(&ControlIngressDeliverInput {
        state_root: &case.workspace,
        topic: &envelope.topic,
        envelope_ref: &envelope.envelope_ref,
    })
    .expect("request-local admission denial");
    let receipt = parse_control_ingress_receipt(&rejected.ingress_receipt_value).expect("rejection receipt");
    assert_eq!(receipt.decision, "deny");
    assert!(!rejected.has_enqueued);
    case.assert_no_pending();
    let admitted = case.deliver();
    let receipt = parse_control_ingress_receipt(&admitted.ingress_receipt_value).expect("valid delivery receipt");
    assert_eq!(receipt.decision, "pass");
    assert_eq!(receipt.phase, "deliver");
    assert!(admitted.has_enqueued);
    assert_eq!(case.queue().request_ref, case.envelope.request.request_ref);
}

// r[verify molten.audit_f03.preserve_state]
#[test]
fn conflicting_submission_does_not_quarantine_other_requests() {
    let case = Case::new("ingress_submission_isolation");
    let other = status_request().expect("distinct canonical request");
    assert_ne!(other.request_ref, case.envelope.request.request_ref);
    let path = control_inbox_path(&case.envelope.request.request_ref).expect("conflicting inbox path");
    write_preserves(&case.reopen(), &path, &other.value).expect("controlled inbox conflict");
    let before = case.read(&path);
    let root = case.reopen();
    assert!(super::super::submit(&root, &case.envelope.request.value).is_err());
    let submitted = super::super::submit(&root, &other.value).expect("independent request remains admissible");
    let receipt = parse_control_queue_receipt(&submitted.queue_receipt_value).expect("valid queue receipt");
    assert_eq!(receipt.decision, "pass");
    assert_eq!(receipt.request_ref, other.request_ref);
    assert_eq!(case.read(&path), before);
}
