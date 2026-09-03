use molten_core::coordination_delivery::*;

use super::super::*;
use super::support::*;

fn receipt() -> DeliveryCommitReceipt {
    DeliveryCommitReceipt {
        queue_id: QUEUE_ID.to_string(),
        request_ref: reference('1'),
        operation_ref: reference('2'),
        before_state_ref: reference('3'),
        after_state_ref: reference('4'),
        revision: 1,
        status: DeliveryServiceStatus::Applied,
        currentness: DeliveryCurrentness::Linearizable,
        durability: DeliveryDurabilityOutcome::Durable,
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
    let first = canonical_delivery_commit_receipt(&receipt).expect("receipt");
    let second = canonical_delivery_commit_receipt(&receipt).expect("repeat receipt");
    assert_eq!(first.receipt_ref, second.receipt_ref);
    assert_eq!(first.bytes, second.bytes);

    let mut authority = receipt.clone();
    authority.authorizes_worker_effects = true;
    assert!(canonical_delivery_commit_receipt(&authority).is_err());

    let mut exactly_once = receipt;
    exactly_once.claims_exactly_once = true;
    assert!(canonical_delivery_commit_receipt(&exactly_once).is_err());
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
        identify_canonical_delivery_status(&status).expect("status"),
        identify_canonical_delivery_status(&status.clone()).expect("repeat status")
    );
    let mut invalid = status;
    invalid.payloads_rendered = true;
    assert!(identify_canonical_delivery_status(&invalid).is_err());
}
