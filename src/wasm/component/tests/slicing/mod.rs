use super::super::*;

const QUANTUM_FUEL: u64 = 1_000;
const TOTAL_BUDGET_FUEL: u64 = 10_000;

#[test]
fn the_time_sliced_profile_binds_a_quantum_and_an_independent_budget() {
    // r[verify molten.wasm_component.execution_quantum]
    let profile = time_sliced().expect("the time-sliced profile is admitted");
    validate_runtime(&profile).expect("the time-sliced profile validates");
    assert_eq!(profile.profile_id, TIME_SLICED_PROFILE_ID);
    let slicing = profile.slicing.expect("the time-sliced profile binds slicing");
    assert!(slicing.quantum_fuel > 0);
    assert!(slicing.total_budget_fuel >= slicing.quantum_fuel);
    assert_eq!(slicing.total_budget_fuel, profile.resources.fuel);

    let base = supported().expect("the supported profile");
    assert_eq!(base.profile_id, COMPONENT_PROFILE_ID);
    assert!(base.slicing.is_none());
}

#[test]
fn a_missing_or_invalid_quantum_binding_is_denied() {
    // r[verify molten.wasm_component.execution_quantum]
    let profile = time_sliced().expect("the time-sliced profile is admitted");
    let mut missing = profile.clone();
    missing.slicing = None;
    assert!(validate_runtime(&missing).is_err());

    let mut zero_quantum = profile.clone();
    zero_quantum.slicing = Some(SlicingProfile {
        quantum_fuel: 0,
        total_budget_fuel: TOTAL_BUDGET_FUEL,
    });
    assert!(validate_runtime(&zero_quantum).is_err());

    let mut zero_budget = profile.clone();
    zero_budget.slicing = Some(SlicingProfile {
        quantum_fuel: QUANTUM_FUEL,
        total_budget_fuel: 0,
    });
    assert!(validate_runtime(&zero_budget).is_err());

    let mut budget_below_quantum = profile;
    budget_below_quantum.slicing = Some(SlicingProfile {
        quantum_fuel: QUANTUM_FUEL,
        total_budget_fuel: QUANTUM_FUEL - 1,
    });
    assert!(validate_runtime(&budget_below_quantum).is_err());
}

#[test]
fn a_time_sliced_request_under_a_synchronous_profile_is_denied() {
    // r[verify molten.wasm_component.execution_quantum]
    let base = supported().expect("the supported profile");
    let denial = admit_requested_profile(&base, RequestedExecutionProfile::TimeSlicedComponentV1)
        .expect_err("the time-sliced request needs an admitting profile");
    assert!(denial.blockers.iter().any(|blocker| blocker.contains("time-sliced")));

    let sliced = time_sliced().expect("the time-sliced profile is admitted");
    let denial = admit_requested_profile(&sliced, RequestedExecutionProfile::ComponentV1)
        .expect_err("a synchronous request must not land on the time-sliced profile");
    assert!(denial.blockers.iter().any(|blocker| blocker.contains("time-sliced")));

    admit_requested_profile(&sliced, RequestedExecutionProfile::TimeSlicedComponentV1)
        .expect("the time-sliced request is admitted by the time-sliced profile");
    admit_requested_profile(&base, RequestedExecutionProfile::ComponentV1)
        .expect("the synchronous request is admitted by the synchronous profile");
}

#[test]
fn the_slice_ledger_records_quantum_boundaries_and_stops_at_the_budget() {
    // r[verify molten.wasm_component.execution_quantum]
    let mut ledger = SliceLedger::new(QUANTUM_FUEL, TOTAL_BUDGET_FUEL).expect("the ledger binds the budget");
    let first = ledger.record_boundary(QUANTUM_FUEL).expect("the first boundary is recorded");
    assert_eq!(first.index, 0);
    let second = ledger.record_boundary(QUANTUM_FUEL).expect("the second boundary is recorded");
    assert_eq!(second.index, 1);
    assert_eq!(ledger.boundaries.len(), 2);
    assert!(!ledger.is_budget_exhausted());
    assert_eq!(ledger.record_boundary(TOTAL_BUDGET_FUEL), Err(SliceIssue::BudgetExceeded));
    assert_eq!(SliceLedger::new(0, TOTAL_BUDGET_FUEL), Err(SliceIssue::ZeroQuantum), "a zero quantum is denied");
    assert_eq!(
        SliceLedger::new(QUANTUM_FUEL, QUANTUM_FUEL - 1),
        Err(SliceIssue::BudgetBelowQuantum),
        "a budget below one quantum is denied"
    );
}

#[test]
fn a_replay_divergence_names_the_boundary() {
    // r[verify molten.wasm_component.yield_continuation]
    let mut recorded = SliceLedger::new(QUANTUM_FUEL, TOTAL_BUDGET_FUEL).expect("the ledger binds the budget");
    recorded.record_boundary(QUANTUM_FUEL).expect("the first boundary is recorded");
    recorded.record_boundary(QUANTUM_FUEL).expect("the second boundary is recorded");

    let mut recomputed = SliceLedger::new(QUANTUM_FUEL, TOTAL_BUDGET_FUEL).expect("the ledger binds the budget");
    recomputed.record_boundary(QUANTUM_FUEL).expect("the first boundary is recorded");
    recomputed.record_boundary(QUANTUM_FUEL + 1).expect("the second boundary is recorded");

    let mismatch = recomputed.compare_replay(&recorded).expect_err("the divergent boundary is named");
    assert_eq!(mismatch.boundary_index, 1);
    assert_eq!(mismatch.recorded_fuel, QUANTUM_FUEL);
    assert_eq!(mismatch.recomputed_fuel, QUANTUM_FUEL + 1);

    let matching = SliceLedger::new(QUANTUM_FUEL, TOTAL_BUDGET_FUEL).expect("the ledger binds the budget");
    let mut matching = matching;
    matching.record_boundary(QUANTUM_FUEL).expect("the first boundary is recorded");
    matching.record_boundary(QUANTUM_FUEL).expect("the second boundary is recorded");
    matching.compare_replay(&recorded).expect("the recorded slicing replays");
}

#[test]
fn a_pending_turn_publishes_nothing_before_completion_and_publishes_once() {
    // r[verify molten.wasm_component.yield_continuation]
    let pending = PendingTurn::new("committed-turn");
    assert!(!pending.is_published());
    let published = pending.publish().expect("the completed turn publishes");
    assert_eq!(published, "committed-turn");
}

#[test]
fn a_yielded_continuation_is_requeued_as_an_ordinary_runnable() {
    // r[verify molten.wasm_component.yield_continuation]
    let continuation = ContinuationKey {
        service_id: "molten.fixture.component".to_string(),
        generation: 1,
        runnable_id: "callback-1".to_string(),
        occurrence_sequence: 1,
    };
    let key = continuation.runnable_key();
    assert_eq!(key.service_id, "molten.fixture.component");
    assert_eq!(key.runnable_id, "callback-1");
    let command = continuation.requeue_command(0);
    assert_eq!(command, molten_core::fabric_time::SchedulerCommand::Wake { key, priority: 0 });
}
