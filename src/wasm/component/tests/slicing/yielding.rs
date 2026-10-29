use super::super::support::engine;
use super::super::support::execute;
use super::*;

const SPIN_ITERATIONS: u32 = 3_000;
const SLICE_LIMIT: usize = 1_000;
const TEST_GENERATION: u64 = 1;

fn sliced_fixture(component_bytes: Vec<u8>) -> super::super::support::Fixture {
    let profile = time_sliced().expect("the time-sliced profile is admitted");
    let mut fixture = super::super::support::Fixture::with_profile(Consumer::Actor, profile);
    fixture.replace_component_bytes(component_bytes);
    fixture
}

fn continuation(runnable_id: &str, occurrence_sequence: u64) -> ContinuationKey {
    ContinuationKey {
        service_id: "molten.fixture.component".to_string(),
        generation: TEST_GENERATION,
        runnable_id: runnable_id.to_string(),
        occurrence_sequence,
    }
}

fn time_profile() -> molten_core::fabric_time::AdmittedTimeProfile {
    use molten_core::fabric_time::*;
    let descriptor = TimeProfileDescriptor {
        schema: FABRIC_TIME_PROFILE_SCHEMA.to_string(),
        profile_id: "component-slicing-test".to_string(),
        profile_ref: crate::preserves_rail::content_ref_from_bytes(b"component-slicing-test"),
        kind: TimeProfileKind::DeterministicSimulation,
        supported_domains: REQUIRED_TIME_DOMAINS.to_vec(),
        max_duration_ticks: 1_000,
        max_uncertainty_ticks: 1_000,
        max_timers: 16,
        max_runnables: 16,
        max_entropy_request_bytes: 64,
        max_entropy_total_bytes: 1_024,
        max_scheduler_concurrency: 4,
        max_scheduler_queue_depth: 16,
        fairness_bound_turns: None,
        scheduler_policy: policy(),
        evidence_mode: TimeEvidenceMode::Aggregate,
        non_claims: REQUIRED_TIME_NON_CLAIMS.to_vec(),
    };
    crate::fabric_time::admit_profile(&descriptor).expect("admitted time profile").profile
}

fn policy() -> molten_core::fabric_time::SchedulerPolicy {
    molten_core::fabric_time::SchedulerPolicy {
        ordering: molten_core::fabric_time::SchedulerOrdering::Fifo,
        replay: molten_core::fabric_time::SchedulerReplayPolicy::Deterministic,
        overload: molten_core::fabric_time::SchedulerOverloadPolicy::Reject,
    }
}

fn run_to_completion(execution: &mut SlicedExecution<'_>) -> (usize, ExecutionOutcome) {
    for slices in 0..SLICE_LIMIT {
        if let SliceProgress::Completed(outcome) = execution.run_slice() {
            return (slices, outcome);
        }
    }
    panic!("the sliced execution did not complete within {SLICE_LIMIT} slices");
}

#[test]
fn a_cpu_heavy_callback_yields_while_an_unrelated_runnable_completes_between_its_slices() {
    // r[verify molten.wasm_component.execution_quantum]
    // r[verify molten.wasm_component.yield_continuation]
    use molten_core::fabric_time::*;
    let profile = time_profile();
    let heavy = continuation("heavy-callback", 1);
    let sibling = continuation("sibling-runnable", 2);
    let mut state = new_scheduler_state(&profile, TEST_GENERATION);
    for key in [heavy.runnable_key(), sibling.runnable_key()] {
        let wake = SchedulerCommand::Wake { key, priority: 0 };
        state = apply_scheduler_command(&profile, policy(), &state, TEST_GENERATION, &wake).expect("wake").next;
    }
    let fixture = sliced_fixture(super::super::support::cpu_heavy_bytes(SPIN_ITERATIONS));
    let input = super::super::support::input_value();
    let request = fixture.sliced_request(&input);
    let engine = engine(&fixture);
    let mut execution = engine.start_sliced(&request, heavy.clone()).expect("admitted sliced execution");
    let mut events = Vec::with_capacity(SLICE_LIMIT);
    let mut outcome = None;
    while outcome.is_none() && events.len() < SLICE_LIMIT {
        let selection = choose_runnable(&profile, policy(), &state, TEST_GENERATION, None).expect("a ready runnable");
        state = selection.next;
        let command = if selection.selected == sibling.runnable_key() {
            events.push("sibling-completed");
            SchedulerCommand::Complete {
                key: selection.selected,
            }
        } else {
            match execution.run_slice() {
                SliceProgress::Yielded { requeue, .. } => {
                    events.push("heavy-yielded");
                    requeue
                }
                SliceProgress::Completed(completed) => {
                    events.push("heavy-completed");
                    outcome = Some(completed);
                    SchedulerCommand::Complete {
                        key: heavy.runnable_key(),
                    }
                }
            }
        };
        state = apply_scheduler_command(&profile, policy(), &state, TEST_GENERATION, &command)
            .expect("admitted")
            .next;
    }
    let outcome = outcome.expect("the heavy callback completes");
    assert!(outcome.is_pass(), "{:?}", outcome.diagnostics);
    assert_eq!(outcome.output.as_ref(), Some(&input));
    assert_eq!(events.first(), Some(&"heavy-yielded"));
    let sibling_at = events.iter().position(|event| *event == "sibling-completed").expect("sibling ran");
    let completed_at = events.iter().position(|event| *event == "heavy-completed").expect("heavy completed");
    assert!(sibling_at < completed_at, "the sibling completes between the heavy callback's slices: {events:?}");
    let ledger = execution.ledger();
    assert!(ledger.boundaries.len() >= 2);
    let remaining = outcome.receipts.last().and_then(|receipt| receipt.input.fuel_remaining).expect("fuel remaining");
    assert_eq!(ledger.consumed_fuel, ledger.total_budget_fuel - remaining);
}

#[test]
fn a_sliced_execution_stops_at_the_budget_with_the_fuel_exhaustion_classification() {
    // r[verify molten.wasm_component.execution_quantum]
    let fixture = sliced_fixture(super::super::support::fuel_exhaustion_bytes());
    let input = super::super::support::input_value();
    let request = fixture.sliced_request(&input);
    let engine = engine(&fixture);
    let mut execution = engine.start_sliced(&request, continuation("spinning", 1)).expect("admitted");
    let (slices, outcome) = run_to_completion(&mut execution);
    assert!(slices >= 1, "the spinning guest yields before the budget ends");
    assert!(!outcome.is_pass());
    let denial = outcome.receipts.last().expect("denial receipt");
    assert_eq!(denial.input.stage, super::super::super::evidence::receipt::Stage::Denial);
    assert_eq!(denial.input.trap_class.as_deref(), Some("fuel-exhausted"));
}

#[test]
fn a_resumed_continuation_commits_its_turn_once_and_never_reruns_from_the_start() {
    // r[verify molten.wasm_component.yield_continuation]
    let fixture = sliced_fixture(super::super::support::cpu_heavy_bytes(SPIN_ITERATIONS));
    let input = super::super::support::input_value();
    let sliced_request = fixture.sliced_request(&input);
    let engine = engine(&fixture);
    let mut execution = engine.start_sliced(&sliced_request, continuation("resumed", 1)).expect("admitted");
    let (slices, outcome) = run_to_completion(&mut execution);
    assert!(slices >= 1);
    assert!(outcome.is_pass());
    let again = execution.run_slice();
    assert!(
        matches!(&again, SliceProgress::Completed(repeated) if !repeated.is_pass()),
        "a completed turn never publishes again"
    );

    let mut synchronous = super::super::support::Fixture::new(Consumer::Actor);
    synchronous.replace_component_bytes(super::super::support::cpu_heavy_bytes(SPIN_ITERATIONS));
    let single = execute(&synchronous.request(&input));
    let single_receipt = single.receipts.last().expect("execution receipt");
    let single_consumed =
        single_receipt.input.fuel_limit.expect("limit") - single_receipt.input.fuel_remaining.expect("remaining");
    let ledger = execution.ledger();
    assert_eq!(ledger.consumed_fuel, single_consumed, "resuming continues the same work instead of rerunning it");

    let mut replayed = engine.start_sliced(&sliced_request, continuation("resumed", 1)).expect("admitted");
    run_to_completion(&mut replayed);
    assert_eq!(replayed.ledger().compare_replay(ledger), Ok(()));
}

#[test]
fn a_synchronous_request_cannot_start_a_sliced_execution() {
    // r[verify molten.wasm_component.execution_quantum]
    let fixture = sliced_fixture(super::super::support::identity_bytes());
    let input = super::super::support::input_value();
    let synchronous = fixture.request(&input);
    let engine = engine(&fixture);
    let denied = engine.start_sliced(&synchronous, continuation("synchronous", 1)).err().expect("denied");
    assert!(!denied.is_pass());
}

/// A real sliced component execution driven as the composition's saturating work.
struct WasmtimeSaturation<'a, 'b> {
    execution: &'b mut SlicedExecution<'a>,
}

impl crate::harness::composition::SaturatingWork for WasmtimeSaturation<'_, '_> {
    fn run_slice(&mut self) -> crate::harness::composition::SaturationSlice {
        match self.execution.run_slice() {
            SliceProgress::Yielded { .. } => crate::harness::composition::SaturationSlice::Yielded,
            SliceProgress::Completed(outcome) => crate::harness::composition::SaturationSlice::Ended {
                is_within_budget: outcome.is_pass(),
            },
        }
    }
}

// r[verify molten.testing.service_recovery_composition]
#[test]
fn the_saturation_scenario_holds_with_real_wasmtime_slices() {
    let fixture = sliced_fixture(super::super::support::cpu_heavy_bytes(SPIN_ITERATIONS));
    let input = super::super::support::input_value();
    let request = fixture.sliced_request(&input);
    let engine = engine(&fixture);
    let scenario = crate::harness::composition::FailureScenario::CpuSaturationUnderTimeSlicing;
    for (variant, is_expected_to_hold) in [
        (crate::harness::composition::ScenarioVariant::Correct, true),
        (crate::harness::composition::ScenarioVariant::SlicingDisabled, false),
    ] {
        let mut execution = engine.start_sliced(&request, continuation("saturating", 1)).expect("admitted");
        let mut work = WasmtimeSaturation {
            execution: &mut execution,
        };
        let facts = crate::harness::composition::run_saturation_scenario(variant, &mut work).expect("scenario runs");
        assert!(facts.saturating_work_bounded, "both variants must complete the same bounded callback");
        assert_eq!(facts.sibling_progressed, is_expected_to_hold, "{}: {facts:?}", variant.as_str());
        let outcomes = crate::harness::composition::evaluate_properties(scenario, &facts);
        let property = outcomes
            .iter()
            .find(|outcome| {
                outcome.property == crate::harness::composition::ServiceProperty::UnaffectedServicesContinue
            })
            .expect("the saturation scenario evaluates sibling progress");
        assert_eq!(property.holds, is_expected_to_hold, "{}: {property:?}", variant.as_str());
    }
}
