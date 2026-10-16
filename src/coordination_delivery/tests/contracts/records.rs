use molten_core::coordination_delivery::*;

use super::super::*;
use super::support::*;

fn receipt() -> CommitReceipt {
    CommitReceipt {
        queue_id: QUEUE_ID.to_string(),
        request_ref: reference('1'),
        operation_ref: reference('2'),
        before_state_ref: reference('3'),
        after_state_ref: reference('4'),
        revision: 1,
        status: ServiceStatus::Applied,
        currentness: DeliveryCurrentness::Linearizable,
        durability: DurabilityOutcome::Durable,
        engine_epoch: ENGINE_EPOCH,
        timer_refs: vec![reference('5')],
        failed_timer_refs: Vec::new(),
        status_ref: Some(reference('6')),
        issue: None,
        authorizes_future_mutation: false,
        authorizes_worker_effects: false,
        claims_exactly_once: false,
        non_claims: required_delivery_non_claims(),
    }
}

// r[verify molten.coordination_delivery.final_validation]
#[test]
fn canonical_receipt_is_stable_and_rejects_authority_overclaims() {
    let receipt = receipt();
    let first = canonical_commit_receipt(&receipt).expect("receipt");
    let second = canonical_commit_receipt(&receipt).expect("repeat receipt");
    assert_eq!(first.receipt_ref, second.receipt_ref);
    assert_eq!(first.bytes, second.bytes);

    let mut authority = receipt.clone();
    authority.authorizes_worker_effects = true;
    assert!(canonical_commit_receipt(&authority).is_err());

    let mut exactly_once = receipt;
    exactly_once.claims_exactly_once = true;
    assert!(canonical_commit_receipt(&exactly_once).is_err());
}

// r[verify molten.coordination_delivery.retry_dlq_policy]
#[test]
fn status_identity_is_ordered_bounded_and_payload_free() {
    let status = DeliveryStatus {
        schema: DELIVERY_STATUS_SCHEMA.to_string(),
        queue_id: QUEUE_ID.to_string(),
        state_ref: reference('1'),
        revision: 1,
        policy_ref: reference('4'),
        maximum_attempts: MAX_ATTEMPTS,
        ready_count: 0,
        retry_count: 0,
        in_flight_count: 1,
        dead_letter_count: 0,
        completed_count: 0,
        failed_attempt_count: 0,
        active_claims: vec![ActiveDeliveryStatus {
            item_ref: reference('2'),
            delivery_id: reference('3'),
            consumer_id: ACTOR_ID.to_string(),
            attempt: 1,
            visibility_deadline_tick: INITIAL_TICK + VISIBILITY_TICKS,
        }],
        resource_refs: vec![reference('5')],
        evidence_refs: vec![reference('6')],
        truncated: false,
        payloads_rendered: false,
    };
    assert_eq!(
        identify_canonical_status(&status).expect("status"),
        identify_canonical_status(&status).expect("repeat status")
    );
    let mut invalid = status;
    invalid.payloads_rendered = true;
    assert!(identify_canonical_status(&invalid).is_err());
}

// r[verify molten.audit_f08.original_binding]
// r[verify molten.audit_f08.unavailable_result]
// r[verify molten.audit_f08.validation_claims]
#[test]
fn unavailable_duplicate_round_trips_without_rewriting_original() {
    let policy = policy();
    let manifest = manifest(&policy);
    let mut commit = MemoryCommitPort::new(CommitMode::Apply);
    let mut timers = timer_port(false);
    let mut statuses = MemoryStatusPort::default();
    let enqueue = enqueue_request(&manifest, '1');
    apply_current_request(&mut commit, &mut timers, &mut statuses, &manifest, &policy, &enqueue);
    let claim = request(&manifest, '2', INITIAL_TICK, DeliveryOperation::Claim);
    let claimed = apply_current_request(&mut commit, &mut timers, &mut statuses, &manifest, &policy, &claim);
    let original_receipt_bytes = claimed.receipt.bytes.clone();
    let original_transition_bytes = serde_json::to_vec(&claimed.transition).expect("serialize historical claim");
    let token = claimed.transition.token.clone().expect("original claim token");
    let extension = request(&manifest, '3', INITIAL_TICK + 1, DeliveryOperation::ExtendLease { token });
    let extended = apply_current_request(&mut commit, &mut timers, &mut statuses, &manifest, &policy, &extension);
    let duplicate = apply_current_request(&mut commit, &mut timers, &mut statuses, &manifest, &policy, &claim);
    assert_eq!(duplicate.transition.token, None);
    let encoded = serde_json::to_vec(&duplicate.transition).expect("serialize unavailable replay");
    let decoded: DeliveryTransition = serde_json::from_slice(&encoded).expect("decode unavailable replay");
    assert_eq!(decoded, duplicate.transition);
    assert_eq!(decoded.operation_ref, claimed.transition.operation_ref);
    assert_eq!(decoded.prior_operation_ref, Some(claimed.transition.operation_ref.clone()));
    let historical: DeliveryTransition = serde_json::from_slice(&original_transition_bytes).expect("old claim");
    assert_eq!(historical, claimed.transition);
    assert_ne!(historical.token, extended.transition.token);

    let canonical = crate::preserves_rail::parse_canonical_bytes(&duplicate.receipt.bytes).expect("canonical replay");
    assert_eq!(canonical, duplicate.receipt.value);
    assert!(
        crate::preserves_rail::contains_structural_content_ref(&canonical, &historical.operation_ref)
            .expect("original operation identity")
    );
    assert!(
        !crate::preserves_rail::contains_structural_content_ref(&canonical, &extended.transition.operation_ref)
            .expect("no replacement operation")
    );
    let original = crate::preserves_rail::parse_canonical_bytes(&original_receipt_bytes).expect("historical receipt");
    assert_eq!(original, claimed.receipt.value);
    assert_eq!(
        crate::preserves_rail::canonical_bytes(&original).expect("unchanged old bytes"),
        original_receipt_bytes
    );

    let mut malformed = serde_json::to_value(&decoded).expect("replay JSON");
    malformed["token"] = serde_json::json!({ "token_ref": reference('f') });
    assert!(serde_json::from_value::<DeliveryTransition>(malformed).is_err());
    let truncated = &duplicate.receipt.bytes[..duplicate.receipt.bytes.len() - 1];
    assert!(crate::preserves_rail::parse_canonical_bytes(truncated).is_err());
}
