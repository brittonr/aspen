//! Core regressions for the command-pattern reservation contract.

use super::reservation::*;
use crate::fabric_time::valid_time_id;

const ACTIVE_GENERATION: u64 = 3;
const TOTAL_UNITS: u64 = 4;
const TOTAL_BYTES: u64 = 4_096;

fn reference(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}

fn workers() -> Vec<WorkerCapacity> {
    vec![
        WorkerCapacity {
            worker_id: "worker-a".to_string(),
            units: 2,
            bytes: 2_048,
        },
        WorkerCapacity {
            worker_id: "worker-b".to_string(),
            units: 2,
            bytes: 2_048,
        },
    ]
}

fn state() -> ReservationState_ {
    new_reservation_state(ACTIVE_GENERATION, TOTAL_UNITS, TOTAL_BYTES, workers()).expect("reservation state")
}

fn request(job: &str, units: u64, bytes: u64) -> ReservationRequest {
    ReservationRequest {
        job_id: job.to_string(),
        operation_ref: "operation-a".to_string(),
        requirements: ResourceRequirements::new(units, bytes),
        expected_generation: ACTIVE_GENERATION,
        max_waiting_ticks: 0,
    }
}

// r[verify molten.reservation_command.atomic_eval]
#[test]
fn contended_reservations_serialize_against_authoritative_state() {
    let state = state();
    let first = apply_reservation(&state, &request("job-a", 2, 2_048)).expect("first reservation");
    assert_eq!(first.outcome, ReservationOutcome::Committed);
    assert_eq!(first.next.pool.reserved_units, 2);
    assert_eq!(first.next.pool.available_units(), 2);

    // The second actor evaluates against the committed state, not its own
    // earlier observation.
    let second = apply_reservation(&first.next, &request("job-b", 2, 2_048)).expect("second reservation");
    assert_eq!(second.outcome, ReservationOutcome::Committed);
    assert_eq!(second.next.pool.reserved_units, 4);
    assert_eq!(second.next.pool.available_units(), 0);

    let third = apply_reservation(&second.next, &request("job-c", 1, 512)).expect("rejected reservation");
    assert_eq!(third.outcome, ReservationOutcome::Rejected);
    assert_eq!(
        third.issue,
        Some(ReservationIssue::InsufficientCapacity {
            required_units: 1,
            available_units: 0,
            required_bytes: 512,
            available_bytes: 0,
        })
    );
    assert_eq!(third.next, second.next, "a rejected command preserves state");
    assert_eq!(third.next.pool.reserved_units, 4);
}

// r[verify molten.reservation_command.atomic_eval]
#[test]
fn stale_generation_cannot_reserve_and_preserves_capacity() {
    let state = state();
    let mut stale = request("job-a", 1, 512);
    stale.expected_generation = ACTIVE_GENERATION - 1;
    let denied = apply_reservation(&state, &stale).expect("stale denial");
    assert_eq!(denied.outcome, ReservationOutcome::Rejected);
    assert_eq!(
        denied.issue,
        Some(ReservationIssue::StaleGeneration {
            expected: ACTIVE_GENERATION - 1,
            actual: ACTIVE_GENERATION,
        })
    );
    assert_eq!(denied.next.pool.reserved_units, 0);
    assert_eq!(denied.next.pool.available_units(), TOTAL_UNITS);
}

// r[verify molten.reservation_command.bounded_batches]
#[test]
fn batch_admission_enforces_limits_and_keeps_items_independent() {
    let state = state();
    let items = vec![request("job-a", 1, 512), request("job-b", 1, 512)];
    let admitted = admit_batch(&state, &BatchRequest {
        items: items.clone(),
        max_items: 4,
        max_bytes: 4_096,
        max_waiting_ticks: 10,
    })
    .expect("admitted batch");
    assert_eq!(admitted.items.len(), 2);
    assert!(admitted.items.iter().all(|item| item.outcome == ReservationOutcome::Committed));
    assert_eq!(admitted.next.pool.reserved_units, 2);

    let too_many = admit_batch(&state, &BatchRequest {
        items: items.clone(),
        max_items: 1,
        max_bytes: 4_096,
        max_waiting_ticks: 10,
    })
    .expect_err("item-count limit");
    assert_eq!(too_many, BatchIssue::TooManyItems { items: 2, maximum: 1 });

    let too_many_bytes = admit_batch(&state, &BatchRequest {
        items: items.clone(),
        max_items: 4,
        max_bytes: 512,
        max_waiting_ticks: 10,
    })
    .expect_err("byte-count limit");
    assert_eq!(too_many_bytes, BatchIssue::TooManyBytes {
        bytes: 1_024,
        maximum: 512
    });

    let mut waiting = request("job-c", 1, 512);
    waiting.max_waiting_ticks = 11;
    let too_long = admit_batch(&state, &BatchRequest {
        items: vec![waiting],
        max_items: 4,
        max_bytes: 4_096,
        max_waiting_ticks: 10,
    })
    .expect_err("waiting-time limit");
    assert_eq!(too_long, BatchIssue::WaitingTimeExceeded { ticks: 11, maximum: 10 });
}

// r[verify molten.reservation_command.bounded_batches]
#[test]
fn one_invalid_item_does_not_poison_the_rest_of_the_batch() {
    let state = state();
    let invalid = request("job-b", 9, 512);
    let admitted = admit_batch(&state, &BatchRequest {
        items: vec![request("job-a", 1, 512), invalid, request("job-c", 1, 512)],
        max_items: 8,
        max_bytes: 4_096,
        max_waiting_ticks: 10,
    })
    .expect("per-item isolation");
    assert_eq!(admitted.items.len(), 3);
    assert_eq!(admitted.items[0].outcome, ReservationOutcome::Committed);
    assert_eq!(admitted.items[1].outcome, ReservationOutcome::Rejected);
    assert_eq!(admitted.items[2].outcome, ReservationOutcome::Committed);
    assert_eq!(admitted.next.pool.reserved_units, 2);
    assert_eq!(admitted.items[1].reservation, None);
}

// r[verify molten.reservation_command.expiry_not_release]
#[test]
fn expiry_without_enforcement_keeps_capacity_counted_and_visible() {
    let state = state();
    let committed = apply_reservation(&state, &request("job-a", 2, 2_048)).expect("reservation");
    let expired = expire_reservation(&committed.next, &ReservationExpiry {
        job_id: "job-a".to_string(),
        enforcement: None,
    })
    .expect("expiry");
    assert_eq!(expired.outcome, ExpiryOutcome::StaleCapacityRetained);
    assert_eq!(expired.next.pool.reserved_units, 2, "stale capacity stays counted");
    assert_eq!(expired.next.pool.available_units(), 2);
    let status = reservation_status(&expired.next);
    assert_eq!(status.stale_held_units, 2);
    assert_eq!(status.shortfall_units, 2);
    assert_eq!(status.shortfall_bytes, 2_048);
}

// r[verify molten.reservation_command.expiry_not_release]
#[test]
fn a_termination_fact_releases_expired_capacity() {
    let state = state();
    let committed = apply_reservation(&state, &request("job-a", 2, 2_048)).expect("reservation");
    let released = expire_reservation(&committed.next, &ReservationExpiry {
        job_id: "job-a".to_string(),
        enforcement: Some(EnforcementFact::FencingTakeover {
            authority_ref: reference("takeover"),
        }),
    })
    .expect("release");
    assert_eq!(released.outcome, ExpiryOutcome::Released);
    assert_eq!(released.next.pool.reserved_units, 0);
    assert_eq!(released.next.pool.available_units(), TOTAL_UNITS);
    let status = reservation_status(&released.next);
    assert_eq!(status.shortfall_units, 0);

    let unadmitted = expire_reservation(&committed.next, &ReservationExpiry {
        job_id: "job-a".to_string(),
        enforcement: Some(EnforcementFact::ObservedStop {
            observation_ref: "not-a-ref".to_string(),
        }),
    })
    .expect("unadmitted enforcement fact keeps capacity counted");
    assert_eq!(unadmitted.outcome, ExpiryOutcome::StaleCapacityRetained);
    assert_eq!(unadmitted.next.pool.reserved_units, 2);
    assert_eq!(reservation_status(&unadmitted.next).shortfall_units, 2);
}

// r[verify molten.reservation_command.validation]
#[test]
fn capacity_invariants_hold_on_every_path() {
    let state = state();
    let committed = apply_reservation(&state, &request("job-a", 2, 2_048)).expect("reservation");
    assert!(committed.next.pool.reserved_units <= committed.next.pool.total_units);
    assert!(committed.next.pool.allocated_units <= committed.next.pool.reserved_units);
    let denied = apply_reservation(&committed.next, &request("job-b", 9, 9_999)).expect("denial");
    assert!(denied.next.pool.reserved_units <= denied.next.pool.total_units);
    assert_eq!(denied.next.pool.reserved_units, 2);
    let expired = expire_reservation(&committed.next, &ReservationExpiry {
        job_id: "job-a".to_string(),
        enforcement: Some(EnforcementFact::NonExternalWork),
    })
    .expect("expiry");
    assert!(expired.next.pool.allocated_units <= expired.next.pool.reserved_units);
    assert_eq!(expired.next.pool.reserved_units, 0);
    assert_eq!(expired.next.pool.allocated_units, 0);
}

#[test]
fn reservation_identifiers_are_bounded_and_unique() {
    assert!(new_reservation_state(0, 1, 1, Vec::new()).is_err());
    let duplicate = new_reservation_state(ACTIVE_GENERATION, 1, 1, vec![
        WorkerCapacity {
            worker_id: "worker-a".to_string(),
            units: 1,
            bytes: 1,
        },
        WorkerCapacity {
            worker_id: "worker-a".to_string(),
            units: 1,
            bytes: 1,
        },
    ])
    .expect_err("duplicate worker");
    assert_eq!(duplicate, ReservationIssue::DuplicateJob("worker-id".to_string()));
    let mut malformed = request("job-a", 1, 1);
    malformed.job_id = "not valid".to_string();
    assert_eq!(
        apply_reservation(&state(), &malformed).expect_err("malformed identifier"),
        ReservationIssue::MalformedIdentifier { field: "job-id" }
    );
    assert!(valid_time_id("job-a"));
}
