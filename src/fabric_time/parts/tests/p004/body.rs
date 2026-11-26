// Property: the reservation owner conserves capacity across batches, failures and releases.

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
        let has_allocation_or_uncertain_attempt =
            effects.allocated.contains(&record.job_id) || effects.uncertain.contains(&record.job_id);
        assert!(
            has_allocation_or_uncertain_attempt,
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
                let is_held = runtime.state().reservations.iter().any(|record| {
                    record.job_id == job && record.state != ReservationState::Released
                });
                let result = runtime.expire(&ReservationExpiry { job_id: job, enforcement });
                assert_eq!(result.is_ok(), is_held, "only held reservations expire: {result:?}");
            }
        }
        assert_owner_conserves_capacity(&mut runtime);
    }
}
