// Shell regressions for the command-pattern reservation contract.

const ACTIVE_GENERATION: u64 = 5;
const TOTAL_UNITS: u64 = 4;
const TOTAL_BYTES: u64 = 4_096;

#[derive(Default)]
struct RecordingEffects {
    allocated: Vec<String>,
    uncertain: Vec<String>,
    fail: bool,
    failures: std::collections::BTreeMap<String, crate::fabric::FabricPortError>,
}

impl EffectPort for RecordingEffects {
    fn allocate(&mut self, reservation: &ReservationRecord) -> crate::fabric::FabricPortResult<()> {
        if self.fail {
            return Err(crate::fabric::FabricPortError::capability("injected allocation failure"));
        }
        if let Some(error) = self.failures.get(&reservation.job_id) {
            if error.class() == crate::error::FailureClass::UncertainExternalOutcome {
                self.uncertain.push(reservation.job_id.clone());
            }
            return Err(error.clone());
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
        RecordingEffects { fail: true, ..RecordingEffects::default() },
    );
    let error = runtime.reserve(&request("job-a", 1, 512)).expect_err("allocation failure");
    assert!(error.to_string().contains("reservation allocation failed"));
    assert!(runtime.effects_mut().allocated.is_empty());
    assert_eq!(runtime.state().pool.reserved_units, 0, "a failed effect does not commit the reservation");
}

fn batch(items: Vec<ReservationRequest>) -> BatchRequest {
    // Byte admission is bounded separately; these batches exercise per-item allocation.
    BatchRequest { items, max_items: 8, max_bytes: 4 * TOTAL_BYTES, max_waiting_ticks: 10 }
}

fn held_units(state: &ReservationState_) -> u64 {
    state
        .reservations
        .iter()
        .filter(|record| record.state != ReservationState::Released)
        .map(|record| record.requirements.units)
        .sum()
}

// r[verify molten.fabric_time.batch_item_atomicity_and_fenced_renew]
#[test]
fn a_later_batch_item_failure_keeps_earlier_allocations_committed() {
    let mut effects = RecordingEffects::default();
    effects
        .failures
        .insert("job-b".to_string(), crate::fabric::FabricPortError::capability("injected adapter failure"));
    let mut runtime = Owner::new(state(), effects);
    let error = runtime
        .reserve_batch(&batch(vec![request("job-a", 2, 2_048), request("job-b", 2, 2_048), request("job-c", 1, 512)]))
        .expect_err("second item fails");
    assert_eq!(error.class(), Some(crate::error::FailureClass::CapabilityUnavailable), "{error}");
    assert!(error.to_string().contains("reservation allocation failed for job-b"), "{error}");
    assert_eq!(runtime.effects_mut().allocated, vec!["job-a".to_string()], "the batch stops at the failed item");
    assert_eq!(runtime.state().pool.reserved_units, 2, "item 1 stays committed");
    assert_eq!(reservations_by_job(runtime.state()).keys().copied().collect::<Vec<_>>(), vec!["job-a"]);

    // The externally allocated item cannot be allocated a second time, and its capacity stays held.
    runtime.effects_mut().failures.clear();
    let retry = runtime
        .reserve_batch(&batch(vec![request("job-a", 2, 2_048), request("job-d", 2, 2_048), request("job-e", 2, 2_048)]))
        .expect("retry batch");
    assert_eq!(retry[0].decision.issue, Some(ReservationIssue::DuplicateJob("job-a".to_string())));
    assert_eq!(retry[1].decision.outcome, ReservationOutcome::Committed);
    assert_eq!(retry[2].decision.outcome, ReservationOutcome::Rejected, "no capacity is double-allocated");
    assert_eq!(runtime.effects_mut().allocated, vec!["job-a".to_string(), "job-d".to_string()]);
    assert_eq!(runtime.state().pool.reserved_units, TOTAL_UNITS);
}

// r[verify molten.fabric_time.batch_item_atomicity_and_fenced_renew]
#[test]
fn an_uncertain_batch_allocation_keeps_its_class_and_holds_capacity_until_enforcement() {
    let mut effects = RecordingEffects::default();
    effects
        .failures
        .insert("job-b".to_string(), crate::fabric::FabricPortError::uncertain("allocation outcome unknown"));
    let mut runtime = Owner::new(state(), effects);
    let error = runtime
        .reserve_batch(&batch(vec![request("job-a", 1, 512), request("job-b", 2, 2_048), request("job-c", 1, 512)]))
        .expect_err("uncertain allocation");
    assert_eq!(error.class(), Some(crate::error::FailureClass::UncertainExternalOutcome), "{error}");
    assert!(!matches!(error, crate::error::Failure::InvalidHarness(_)));
    assert_eq!(runtime.effects_mut().allocated, vec!["job-a".to_string()]);
    assert_eq!(runtime.effects_mut().uncertain, vec!["job-b".to_string()]);
    assert_eq!(runtime.state().pool.reserved_units, 3, "uncertain capacity stays held");
    assert_eq!(runtime.status().available_units, 1);

    // A second request for the uncertain item cannot allocate again.
    runtime.effects_mut().failures.clear();
    let duplicate = runtime.reserve(&request("job-b", 2, 2_048)).expect("duplicate decision");
    assert_eq!(duplicate.issue, Some(ReservationIssue::DuplicateJob("job-b".to_string())));
    assert_eq!(runtime.effects_mut().allocated, vec!["job-a".to_string()]);

    // Only an admitted enforcement fact returns the uncertain capacity.
    let outcome = runtime
        .expire(&ReservationExpiry {
            job_id: "job-b".to_string(),
            enforcement: Some(EnforcementFact::ObservedStop { observation_ref: reference("job-b-stopped") }),
        })
        .expect("release");
    assert_eq!(outcome, ExpiryOutcome::Released);
    assert_eq!(runtime.state().pool.reserved_units, 1);
}

// r[verify molten.fabric_time.batch_item_atomicity_and_fenced_renew]
#[test]
fn a_definite_single_allocation_failure_keeps_its_port_class() {
    let mut effects = RecordingEffects::default();
    effects.failures.insert("job-a".to_string(), crate::fabric::FabricPortError::transport("adapter unreachable"));
    let mut runtime = Owner::new(state(), effects);
    let error = runtime.reserve(&request("job-a", 1, 512)).expect_err("allocation failure");
    assert_eq!(error.class(), Some(crate::error::FailureClass::TransportFailure));
    assert_eq!(runtime.state().pool.reserved_units, 0);
    assert!(runtime.state().reservations.is_empty());
}

const PROPERTY_JOBS: u64 = 6;
const PROPERTY_STEPS: u64 = 24;

fn property_job(tc: &hegel::TestCase) -> String {
    format!("job-{}", tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(PROPERTY_JOBS - 1)))
}

fn property_request(tc: &hegel::TestCase, effects: &mut RecordingEffects) -> ReservationRequest {
    let job = property_job(tc);
    let units = tc.draw(hegel::generators::integers::<u64>().min_value(1).max_value(2));
    let bytes = tc.draw(hegel::generators::integers::<u64>().min_value(1).max_value(1_024));
    match tc.draw(hegel::generators::integers::<u8>().max_value(2)) {
        0 => {
            effects.failures.remove(&job);
        }
        1 => {
            effects.failures.insert(job.clone(), crate::fabric::FabricPortError::capability("injected"));
        }
        _ => {
            effects.failures.insert(job.clone(), crate::fabric::FabricPortError::uncertain("injected"));
        }
    }
    request(&job, units, bytes)
}

fn assert_owner_conserves_capacity(runtime: &mut Owner<RecordingEffects>) {
    let initial = state();
    let current = runtime.state().clone();
    let held = held_units(&current);
    assert_eq!(current.pool.reserved_units, held);
    assert_eq!(current.pool.allocated_units, held);
    let worker_units: u64 = current.workers.iter().map(|worker| worker.units).sum();
    let initial_units: u64 = initial.workers.iter().map(|worker| worker.units).sum();
    assert_eq!(worker_units + held, initial_units, "worker capacity is conserved");
    let effects = runtime.effects_mut();
    let mut allocated = effects.allocated.clone();
    allocated.sort();
    let before = allocated.len();
    allocated.dedup();
    assert_eq!(allocated.len(), before, "no job is allocated twice: {:?}", effects.allocated);
    for record in &current.reservations {
        assert!(
            effects.allocated.contains(&record.job_id) || effects.uncertain.contains(&record.job_id),
            "committed record {} has an allocation or uncertain attempt",
            record.job_id
        );
    }
    for job in &effects.allocated {
        assert!(current.reservations.iter().any(|record| &record.job_id == job), "allocated {job} is committed");
    }
}

// r[verify molten.fabric_time.batch_item_atomicity_and_fenced_renew]
// r[verify molten.fabric_time.reservation_stale_release_and_worker_conservation]
#[hegel::test(test_cases = 64)]
fn hegel_owner_conserves_capacity_across_batches_failures_and_releases(tc: hegel::TestCase) {
    let mut runtime = Owner::new(state(), RecordingEffects::default());
    let steps = tc.draw(hegel::generators::integers::<u64>().min_value(1).max_value(PROPERTY_STEPS));
    for _ in 0..steps {
        match tc.draw(hegel::generators::integers::<u8>().max_value(2)) {
            0 => {
                let request = property_request(&tc, runtime.effects_mut());
                if let Err(error) = runtime.reserve(&request) {
                    assert!(error.class().is_some(), "allocation failures stay classified: {error}");
                }
            }
            1 => {
                let count = tc.draw(hegel::generators::integers::<u64>().min_value(1).max_value(3));
                let items = (0..count).map(|_| property_request(&tc, runtime.effects_mut())).collect();
                if let Err(error) = runtime.reserve_batch(&BatchRequest {
                    items,
                    max_items: 8,
                    max_bytes: u64::MAX,
                    max_waiting_ticks: 10,
                }) {
                    assert!(error.class().is_some(), "allocation failures stay classified: {error}");
                }
            }
            _ => {
                let job = property_job(&tc);
                let enforcement = tc.draw(hegel::generators::booleans()).then(|| EnforcementFact::FencingTakeover {
                    authority_ref: reference(&job),
                });
                let held = runtime.state().reservations.iter().any(|record| {
                    record.job_id == job && record.state != ReservationState::Released
                });
                let result = runtime.expire(&ReservationExpiry { job_id: job, enforcement });
                assert_eq!(result.is_ok(), held, "only held reservations expire: {result:?}");
            }
        }
        assert_owner_conserves_capacity(&mut runtime);
    }
}
