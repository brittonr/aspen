//! Negative and boundary fixtures over real processes.

use molten_core::outside_in::*;

use super::client::*;
use super::node::*;
use super::profiles::*;

const MISSING_SIGNAL_TOOL: &str = "molten-outside-in-missing-signal-tool";
const TINY_WALL_CLOCK_MS: u64 = 100;
const TINY_OUTPUT_BYTES: u64 = 8;

fn no_fault() -> ScenarioDeclaration {
    declaration(FaultProfile::NoFault, 0, &["request-1"])
}

// r[verify molten.outside_in.validation]
#[test]
fn live_outside_in_consensus_scenarios_never_start_a_process() {
    let mut consensus = no_fault();
    consensus.path = ScenarioPath::Consensus;
    let result = run_scenario(&consensus, |_| panic!("a consensus-path scenario must not execute"));
    assert_eq!(result.outcome, ScenarioOutcome::DeclarationRejected(vec![ScenarioIssue::ConsensusPathExcluded]));
    assert!(!result.is_pass());
}

// r[verify molten.outside_in.validation]
#[test]
fn live_outside_in_a_listener_on_an_uninitialized_root_is_a_startup_failure() {
    let result = run_scenario(&no_fault(), |harness| {
        let state_root = harness.scratch_dir("uninitialized-state")?;
        start_listener_at(harness, &state_root, state_root.join("ticket.preserves"))?;
        Ok(empty_observation())
    });
    assert_eq!(result.outcome, ScenarioOutcome::StartupFailed("listener".to_string()), "{result:?}");
    assert_eq!(result.teardown, TeardownStatus::Complete, "owned processes and paths are released");
}

// r[verify molten.outside_in.real_stack]
#[test]
fn live_outside_in_each_exhausted_budget_stops_the_scenario_with_its_kind() {
    let mut wall_clock = no_fault();
    wall_clock.budgets.wall_clock_ms = TINY_WALL_CLOCK_MS;
    wall_clock.stabilization_ms = 0;
    let mut output = no_fault();
    output.budgets.max_retained_output_bytes = TINY_OUTPUT_BYTES;
    let mut processes = no_fault();
    processes.budgets.max_processes = 1;
    for (declared, kind) in [
        (wall_clock, BudgetKind::WallClock),
        (output, BudgetKind::RetainedOutput),
        (processes, BudgetKind::Processes),
    ] {
        let result = run_scenario(&declared, baseline_body);
        assert_eq!(result.outcome, ScenarioOutcome::BudgetExhausted(kind), "{result:?}");
        assert_eq!(result.teardown, TeardownStatus::Complete, "{}", kind.as_str());
        assert!(!result.is_pass());
    }
}

// r[verify molten.outside_in.validation]
#[test]
fn live_outside_in_a_fault_the_host_cannot_apply_is_recorded_as_unsupported() {
    let declared = declaration(FaultProfile::PauseResume, 500, &["request-1"]);
    let result = run_scenario(&declared, |harness| {
        harness.set_signal_tool(MISSING_SIGNAL_TOOL);
        pause_resume_body(harness)
    });
    assert_eq!(result.outcome, ScenarioOutcome::Unsupported("signal-STOP".to_string()), "{result:?}");
    assert_eq!(result.teardown, TeardownStatus::Complete);
}

// r[verify molten.outside_in.observation]
#[test]
fn live_outside_in_an_unreadable_history_is_incomplete_not_agreement() {
    let result = run_scenario(&no_fault(), |harness| {
        let mut receiver = prepare_receiver(harness)?;
        let request = build_request(harness, &mut receiver, 1)?;
        let listener = start_listener(harness, &mut receiver)?;
        let mut observed = empty_observation();
        observed.listener_endpoints.push(listener.endpoint.clone());
        let send = start_send(harness, &receiver, &request, &listener.ticket, 10_000)?;
        observed.attempts.push(finish_send(harness, &request, send, ResponseFate::Observed)?);
        finish_listener(harness, &listener)?;
        corrupt_processed_receipts(&receiver)?;
        observed.checkpoints.push(read_history(harness, &receiver, CheckpointKind::Final)?);
        Ok(observed)
    });
    assert_eq!(result.outcome, ScenarioOutcome::Incomplete(MissingObservation::FinalHistory), "{result:?}");
}

// r[verify molten.outside_in.real_stack]
#[test]
fn live_outside_in_a_retained_scratch_path_fails_the_scenario_despite_agreement() {
    let result = run_scenario(&no_fault(), |harness| {
        let observed = baseline_body(harness)?;
        let substituted = harness.scratch_dir("substituted")?;
        std::fs::remove_dir(&substituted).expect("remove owned scratch directory");
        std::fs::write(&substituted, b"substituted").expect("substitute a file");
        Ok(observed)
    });
    assert_eq!(result.outcome, ScenarioOutcome::Agreement, "{result:?}");
    assert_eq!(result.teardown, TeardownStatus::ScratchPathRetained {
        path_label: "substituted".to_string()
    });
    assert!(!result.is_pass());
}
