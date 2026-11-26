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

fn expiry(job: &str, enforcement: Option<EnforcementFact>) -> ReservationExpiry {
    ReservationExpiry {
        job_id: job.to_string(),
        enforcement,
    }
}

fn worker(state: &ReservationState_, worker_id: &str) -> WorkerCapacity {
    state.workers.iter().find(|worker| worker.worker_id == worker_id).cloned().expect("worker exists")
}

// r[verify molten.fabric_time.reservation_stale_release_and_worker_conservation]
#[test]
fn delayed_enforcement_releases_a_stale_reservation_and_restores_worker_capacity() {
    let initial = state();
    for enforcement in [
        EnforcementFact::ObservedStop {
            observation_ref: reference("delayed-stop"),
        },
        EnforcementFact::FencingTakeover {
            authority_ref: reference("delayed-takeover"),
        },
    ] {
        let committed = apply_reservation(&initial, &request("job-a", 2, 2_048)).expect("reservation");
        let reservation = committed.reservation.clone().expect("committed record");
        assert_eq!(worker(&committed.next, &reservation.worker_id).units, 0, "worker fully reserved");
        let stale = expire_reservation(&committed.next, &expiry("job-a", None)).expect("stale expiry");
        assert_eq!(stale.outcome, ExpiryOutcome::StaleCapacityRetained);
        assert_eq!(stale.next.reservations[0].state, ReservationState::Stale);

        let released = expire_reservation(&stale.next, &expiry("job-a", Some(enforcement))).expect("delayed release");
        assert_eq!(released.outcome, ExpiryOutcome::Released);
        assert_eq!(released.next.reservations[0].state, ReservationState::Released);
        assert_eq!(released.next.pool, initial.pool, "pool returns to its pre-reservation level");
        assert_eq!(released.next.workers, initial.workers, "worker returns to its pre-reservation level");
        assert_eq!(reservation_status(&released.next).shortfall_units, 0);
    }
}

// r[verify molten.fabric_time.reservation_stale_release_and_worker_conservation]
#[test]
fn stale_reservation_without_enforcement_keeps_capacity_and_stays_releasable() {
    let committed = apply_reservation(&state(), &request("job-a", 2, 2_048)).expect("reservation");
    let stale = expire_reservation(&committed.next, &expiry("job-a", None)).expect("stale expiry");
    let repeated = expire_reservation(&stale.next, &expiry("job-a", None)).expect("repeated stale expiry");
    assert_eq!(repeated.outcome, ExpiryOutcome::StaleCapacityRetained);
    assert_eq!(repeated.next, stale.next, "an unadmitted repeat changes nothing");
    let unadmitted = expire_reservation(
        &stale.next,
        &expiry(
            "job-a",
            Some(EnforcementFact::ObservedStop {
                observation_ref: "not-a-ref".to_string(),
            }),
        ),
    )
    .expect("unadmitted fact");
    assert_eq!(unadmitted.outcome, ExpiryOutcome::StaleCapacityRetained);
    assert_eq!(unadmitted.next.pool.reserved_units, 2);
    assert_eq!(worker(&unadmitted.next, "worker-a").units, 0);
}

// r[verify molten.fabric_time.reservation_stale_release_and_worker_conservation]
#[test]
fn released_reservation_denies_duplicate_release_without_double_credit() {
    let committed = apply_reservation(&state(), &request("job-a", 2, 2_048)).expect("reservation");
    let released =
        expire_reservation(&committed.next, &expiry("job-a", Some(EnforcementFact::NonExternalWork))).expect("release");
    for enforcement in [
        None,
        Some(EnforcementFact::NonExternalWork),
        Some(EnforcementFact::ObservedStop {
            observation_ref: reference("duplicate-stop"),
        }),
    ] {
        assert_eq!(
            expire_reservation(&released.next, &expiry("job-a", enforcement)),
            Err(ExpiryIssue::AlreadyReleased("job-a".to_string()))
        );
    }
    assert_eq!(released.next.workers, workers(), "worker credited exactly once");
    assert_eq!(released.next.pool.reserved_units, 0);
    assert_eq!(
        expire_reservation(&released.next, &expiry("job-missing", None)),
        Err(ExpiryIssue::UnknownJob("job-missing".to_string()))
    );
}

// r[verify molten.fabric_time.reservation_stale_release_and_worker_conservation]
#[test]
fn worker_capacity_is_conserved_across_allocate_release_cycles() {
    const CYCLES: usize = 8;
    let mut current = state();
    for cycle in 0..CYCLES {
        for slot in ["a", "b"] {
            let job = format!("job-{cycle}-{slot}");
            let committed = apply_reservation(&current, &request(&job, 2, 2_048)).expect("reservation");
            assert_eq!(committed.outcome, ReservationOutcome::Committed, "cycle {cycle} slot {slot}: {committed:?}");
            current = committed.next;
        }
        assert!(current.workers.iter().all(|worker| worker.units == 0 && worker.bytes == 0));
        for slot in ["a", "b"] {
            let job = format!("job-{cycle}-{slot}");
            let stale = expire_reservation(&current, &expiry(&job, None)).expect("stale");
            let released = expire_reservation(
                &stale.next,
                &expiry(
                    &job,
                    Some(EnforcementFact::FencingTakeover {
                        authority_ref: reference(&job),
                    }),
                ),
            )
            .expect("release");
            current = released.next;
        }
        assert_eq!(current.workers, workers(), "cycle {cycle} restores every worker");
        assert_eq!(current.pool.reserved_units, 0);
        assert_eq!(current.pool.allocated_bytes, 0);
    }
}

#[derive(Debug, Clone, Copy)]
enum ConservationCommand {
    Reserve(usize),
    Expire(usize, ConservationFact),
}

#[derive(Debug, Clone, Copy)]
enum ConservationFact {
    None,
    ObservedStop,
    FencingTakeover,
    Unadmitted,
}

const CONSERVATION_JOBS: [(&str, u64, u64); 3] = [("job-x", 1, 512), ("job-y", 2, 1_024), ("job-z", 1, 1_536)];
const CONSERVATION_DEPTH: usize = 4;

fn conservation_commands() -> Vec<ConservationCommand> {
    let mut commands = Vec::new();
    for job in 0..CONSERVATION_JOBS.len() {
        commands.push(ConservationCommand::Reserve(job));
        for fact in [
            ConservationFact::None,
            ConservationFact::ObservedStop,
            ConservationFact::FencingTakeover,
            ConservationFact::Unadmitted,
        ] {
            commands.push(ConservationCommand::Expire(job, fact));
        }
    }
    commands
}

fn conservation_fact(job: &str, fact: ConservationFact) -> Option<EnforcementFact> {
    match fact {
        ConservationFact::None => None,
        ConservationFact::ObservedStop => Some(EnforcementFact::ObservedStop {
            observation_ref: reference(job),
        }),
        ConservationFact::FencingTakeover => Some(EnforcementFact::FencingTakeover {
            authority_ref: reference(job),
        }),
        ConservationFact::Unadmitted => Some(EnforcementFact::ObservedStop {
            observation_ref: job.to_string(),
        }),
    }
}

fn assert_capacity_conserved(initial: &ReservationState_, current: &ReservationState_, trace: &[ConservationCommand]) {
    let held = current.reservations.iter().filter(|record| record.state != ReservationState::Released);
    let (mut held_units, mut held_bytes) = (0_u64, 0_u64);
    for record in held.clone() {
        held_units += record.requirements.units;
        held_bytes += record.requirements.bytes;
    }
    assert_eq!(current.pool.reserved_units, held_units, "{trace:?}");
    assert_eq!(current.pool.reserved_bytes, held_bytes, "{trace:?}");
    assert_eq!(current.pool.allocated_units, held_units, "{trace:?}");
    assert_eq!(current.pool.allocated_bytes, held_bytes, "{trace:?}");
    for original in &initial.workers {
        let (mut worker_units, mut worker_bytes) = (0_u64, 0_u64);
        for record in held.clone().filter(|record| record.worker_id == original.worker_id) {
            worker_units += record.requirements.units;
            worker_bytes += record.requirements.bytes;
        }
        let now = worker(current, &original.worker_id);
        assert_eq!(now.units + worker_units, original.units, "{} units {trace:?}", original.worker_id);
        assert_eq!(now.bytes + worker_bytes, original.bytes, "{} bytes {trace:?}", original.worker_id);
    }
}

fn explore_conservation(
    initial: &ReservationState_,
    current: &ReservationState_,
    commands: &[ConservationCommand],
    trace: &mut Vec<ConservationCommand>,
    explored: &mut usize,
) {
    assert_capacity_conserved(initial, current, trace);
    // Draining every held reservation with an admitted fact returns all capacity.
    let mut drained = current.clone();
    for record in current.reservations.iter().filter(|record| record.state != ReservationState::Released) {
        drained = expire_reservation(&drained, &expiry(&record.job_id, Some(EnforcementFact::NonExternalWork)))
            .expect("held reservation releases")
            .next;
    }
    assert_eq!(drained.workers, initial.workers, "{trace:?}");
    assert_eq!(drained.pool, initial.pool, "{trace:?}");
    *explored += 1;
    if trace.len() == CONSERVATION_DEPTH {
        return;
    }
    for command in commands {
        let next = match *command {
            ConservationCommand::Reserve(job) => {
                let (job_id, units, bytes) = CONSERVATION_JOBS[job];
                let transition = apply_reservation(current, &request(job_id, units, bytes)).expect("valid request");
                if transition.outcome == ReservationOutcome::Rejected {
                    assert_eq!(&transition.next, current, "rejection preserves state {trace:?}");
                }
                transition.next
            }
            ConservationCommand::Expire(job, fact) => {
                let job_id = CONSERVATION_JOBS[job].0;
                let before =
                    current.reservations.iter().find(|record| record.job_id == job_id).map(|record| record.state);
                match expire_reservation(current, &expiry(job_id, conservation_fact(job_id, fact))) {
                    Ok(transition) => {
                        let released =
                            matches!(fact, ConservationFact::ObservedStop | ConservationFact::FencingTakeover);
                        assert_eq!(transition.outcome == ExpiryOutcome::Released, released, "{trace:?}");
                        transition.next
                    }
                    Err(ExpiryIssue::UnknownJob(_)) => {
                        assert_eq!(before, None);
                        current.clone()
                    }
                    Err(ExpiryIssue::AlreadyReleased(_)) => {
                        assert_eq!(before, Some(ReservationState::Released));
                        current.clone()
                    }
                    Err(issue) => panic!("unexpected expiry issue {issue:?} after {trace:?}"),
                }
            }
        };
        trace.push(*command);
        explore_conservation(initial, &next, commands, trace, explored);
        trace.pop();
    }
}

/// Bounded exhaustive property: every reserve/stale/release interleaving over three jobs conserves
/// pool and per-worker capacity, never double-credits, and can always be drained back to the
/// initial state.
// r[verify molten.fabric_time.reservation_stale_release_and_worker_conservation]
#[test]
fn capacity_is_conserved_over_every_bounded_command_sequence() {
    let initial = state();
    let commands = conservation_commands();
    let mut explored = 0_usize;
    explore_conservation(&initial, &initial, &commands, &mut Vec::new(), &mut explored);
    assert!(explored > 10_000, "explored {explored} states");
}

// r[verify molten.runtime_spine.canonical_preserves_identity_and_embedded_scan]
#[test]
fn uppercase_enforcement_digest_is_not_an_admitted_termination_fact() {
    let lowercase = reference("takeover");
    let uppercase = format!("blake3:{}", lowercase.trim_start_matches("blake3:").to_ascii_uppercase());
    assert_ne!(uppercase, lowercase, "fixture digest must contain hex letters");
    assert!(crate::fabric_time::valid_time_ref(&lowercase));
    assert!(!crate::fabric_time::valid_time_ref(&uppercase));

    let committed = apply_reservation(&state(), &request("job-a", 2, 2_048)).expect("reservation");
    let expired = expire_reservation(&committed.next, &ReservationExpiry {
        job_id: "job-a".to_string(),
        enforcement: Some(EnforcementFact::FencingTakeover {
            authority_ref: uppercase,
        }),
    })
    .expect("uppercase enforcement fact keeps capacity counted");
    assert_eq!(expired.outcome, ExpiryOutcome::StaleCapacityRetained);
    assert_eq!(expired.next.pool.reserved_units, 2);
}
