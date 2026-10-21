// Shell regressions for the command-pattern reservation contract.

const ACTIVE_GENERATION: u64 = 5;
const TOTAL_UNITS: u64 = 4;
const TOTAL_BYTES: u64 = 4_096;

#[derive(Default)]
struct RecordingEffects {
    allocated: Vec<String>,
    fail: bool,
}

impl EffectPort for RecordingEffects {
    fn allocate(&mut self, reservation: &ReservationRecord) -> crate::fabric::FabricPortResult<()> {
        if self.fail {
            return Err(crate::fabric::FabricPortError::capability("injected allocation failure"));
        }
        self.allocated.push(reservation.job_id.clone());
        Ok(())
    }
}

fn reference(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}

fn state() -> ReservationState_ {
    new_reservation_state(
        ACTIVE_GENERATION,
        TOTAL_UNITS,
        TOTAL_BYTES,
        vec![
            WorkerCapacity { worker_id: "worker-a".to_string(), units: 2, bytes: 2_048 },
            WorkerCapacity { worker_id: "worker-b".to_string(), units: 2, bytes: 2_048 },
        ],
    )
    .expect("reservation state")
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

// r[verify molten.reservation_command.pure_core]
#[test]
fn a_rejected_reservation_executes_no_allocation_effect() {
    let mut runtime = Owner::new(state(), RecordingEffects::default());
    let mut stale = request("job-a", 1, 512);
    stale.expected_generation = ACTIVE_GENERATION - 1;
    let decision = runtime.reserve(&stale).expect("stale denial");
    assert_eq!(decision.outcome, ReservationOutcome::Rejected);
    assert_eq!(decision.issue, Some(ReservationIssue::StaleGeneration {
        expected: ACTIVE_GENERATION - 1,
        actual: ACTIVE_GENERATION,
    }));
    assert!(runtime.effects_mut().allocated.is_empty(), "a rejection executes no allocation");
    assert_eq!(runtime.state().pool.reserved_units, 0);
}

// r[verify molten.reservation_command.atomic_eval]
#[test]
fn contended_reservations_allocate_once_for_the_committing_actor() {
    let mut runtime = Owner::new(state(), RecordingEffects::default());
    let first = runtime.reserve(&request("job-a", 2, 2_048)).expect("first reservation");
    assert_eq!(first.outcome, ReservationOutcome::Committed);
    let second = runtime.reserve(&request("job-b", 2, 2_048)).expect("second reservation");
    assert_eq!(second.outcome, ReservationOutcome::Committed);
    let third = runtime.reserve(&request("job-c", 1, 512)).expect("rejected reservation");
    assert_eq!(third.outcome, ReservationOutcome::Rejected);
    assert_eq!(runtime.effects_mut().allocated, vec!["job-a".to_string(), "job-b".to_string()]);
    assert_eq!(runtime.state().pool.reserved_units, 4);
}

// r[verify molten.reservation_command.bounded_batches]
#[test]
fn a_batch_allocates_only_its_committed_items() {
    let mut runtime = Owner::new(state(), RecordingEffects::default());
    let invalid = request("job-b", 9, 512);
    let outcomes = runtime
        .reserve_batch(&BatchRequest {
            items: vec![request("job-a", 1, 512), invalid, request("job-c", 1, 512)],
            max_items: 8,
            max_bytes: 4_096,
            max_waiting_ticks: 10,
        })
        .expect("batch");
    assert_eq!(outcomes.len(), 3);
    assert_eq!(outcomes[0].decision.outcome, ReservationOutcome::Committed);
    assert_eq!(outcomes[1].decision.outcome, ReservationOutcome::Rejected);
    assert_eq!(outcomes[2].decision.outcome, ReservationOutcome::Committed);
    assert_eq!(runtime.effects_mut().allocated, vec!["job-a".to_string(), "job-c".to_string()]);
    assert_eq!(runtime.state().pool.reserved_units, 2);
}

// r[verify molten.reservation_command.expiry_not_release]
#[test]
fn expiry_without_enforcement_keeps_the_shortfall_visible() {
    let mut runtime = Owner::new(state(), RecordingEffects::default());
    runtime.reserve(&request("job-a", 2, 2_048)).expect("reservation");
    let outcome = runtime
        .expire(&ReservationExpiry { job_id: "job-a".to_string(), enforcement: None })
        .expect("expiry");
    assert_eq!(outcome, ExpiryOutcome::StaleCapacityRetained);
    let status = runtime.status();
    assert_eq!(status.stale_held_units, 2);
    assert_eq!(status.shortfall_units, 2);
    assert_eq!(status.available_units, 2);
}

// r[verify molten.reservation_command.expiry_not_release]
#[test]
fn a_termination_fact_releases_the_capacity() {
    let mut runtime = Owner::new(state(), RecordingEffects::default());
    runtime.reserve(&request("job-a", 2, 2_048)).expect("reservation");
    let outcome = runtime
        .expire(&ReservationExpiry {
            job_id: "job-a".to_string(),
            enforcement: Some(EnforcementFact::FencingTakeover { authority_ref: reference("takeover") }),
        })
        .expect("release");
    assert_eq!(outcome, ExpiryOutcome::Released);
    assert_eq!(runtime.status().shortfall_units, 0);
    assert_eq!(runtime.status().available_units, TOTAL_UNITS);
}

// r[verify molten.reservation_command.validation]
#[test]
fn a_failed_allocation_is_reported_without_claiming_a_released_capacity() {
    let mut runtime = Owner::new(
        state(),
        RecordingEffects { allocated: Vec::new(), fail: true },
    );
    let error = runtime.reserve(&request("job-a", 1, 512)).expect_err("allocation failure");
    assert!(error.to_string().contains("reservation allocation failed"));
    assert!(runtime.effects_mut().allocated.is_empty());
    assert_eq!(runtime.state().pool.reserved_units, 0, "a failed effect does not commit the reservation");
}
